use crate::workspace::{canonicalize_clean, path_to_clean_string, to_unix_rel_path};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use tectonic::status::{MessageKind, StatusBackend};
use tectonic_bridge_core::{SecuritySettings, SecurityStance};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub executable_path: Option<String>,
    pub is_rust_native: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileRequest {
    pub root_dir: String,
    pub entry_file: String,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatexDiagnostic {
    pub severity: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileResponse {
    pub success: bool,
    pub pdf_base64: Option<String>,
    pub pdf_path: Option<String>,
    pub log_output: String,
    pub diagnostics: Vec<LatexDiagnostic>,
    pub duration_ms: u64,
    pub engine_used: String,
    pub synctex_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheBucketInfo {
    pub path: Option<String>,
    pub bytes: u64,
    pub file_count: u64,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileCacheInfo {
    pub project_cache: CacheBucketInfo,
    pub tectonic_cache: CacheBucketInfo,
    pub total_bytes: u64,
}

struct EngineSpec {
    id: &'static str,
    name: &'static str,
    is_rust_native: bool,
}

const ENGINE_SPECS: &[EngineSpec] = &[
    EngineSpec {
        id: "tectonic",
        name: "Tectonic (Embedded Rust Engine)",
        is_rust_native: true,
    },
    EngineSpec {
        id: "pdflatex",
        name: "pdfLaTeX",
        is_rust_native: false,
    },
    EngineSpec {
        id: "xelatex",
        name: "XeLaTeX",
        is_rust_native: false,
    },
    EngineSpec {
        id: "lualatex",
        name: "LuaLaTeX",
        is_rust_native: false,
    },
];

static EMBEDDED_TECTONIC_LOCK: Mutex<()> = Mutex::new(());

fn embedded_tectonic_info() -> CompilerInfo {
    CompilerInfo {
        id: "tectonic".to_string(),
        name: "Tectonic (Embedded Rust Engine)".to_string(),
        available: true,
        version: Some("Tectonic 0.17.0 (Embedded Rust Library)".to_string()),
        executable_path: Some("embedded://tectonic-0.17.0".to_string()),
        is_rust_native: true,
    }
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn configure_tokio_command(cmd: &mut tokio::process::Command) {
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}

fn binary_filename(engine_id: &str) -> String {
    #[cfg(windows)]
    {
        format!("{engine_id}.exe")
    }
    #[cfg(not(windows))]
    {
        engine_id.to_string()
    }
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn candidate_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();
    let mut push_dir = |dir: PathBuf| {
        if !dir.as_os_str().is_empty() && seen.insert(dir.clone()) {
            dirs.push(dir);
        }
    };

    // 1. Sidecar directory next to the current executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            push_dir(exe_dir.to_path_buf());
            push_dir(exe_dir.join("bin"));
        }
    }

    // 2. Process PATH entries
    if let Some(path_os) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_os) {
            push_dir(dir);
        }
    }

    // 3. Platform-specific non-login-shell tool locations
    #[cfg(target_os = "macos")]
    {
        push_dir(PathBuf::from("/opt/homebrew/bin"));
        push_dir(PathBuf::from("/usr/local/bin"));
        if let Some(home) = std::env::var_os("HOME") {
            push_dir(PathBuf::from(home).join(".cargo").join("bin"));
        }
        push_dir(PathBuf::from("/Library/TeX/texbin"));
        push_dir(PathBuf::from("/usr/texbin"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            push_dir(PathBuf::from(home).join(".cargo").join("bin"));
        }
        push_dir(PathBuf::from("/usr/local/bin"));
        push_dir(PathBuf::from("/usr/bin"));
        push_dir(PathBuf::from("/usr/texbin"));
        push_dir(PathBuf::from("/home/linuxbrew/.linuxbrew/bin"));
    }

    #[cfg(windows)]
    {
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            push_dir(PathBuf::from(profile).join(".cargo").join("bin"));
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let base = PathBuf::from(local_app_data);
            push_dir(base.join("Programs").join("tectonic"));
            push_dir(
                base.join("Programs")
                    .join("MiKTeX")
                    .join("miktex")
                    .join("bin")
                    .join("x64"),
            );
        }
        if let Some(program_files) = std::env::var_os("ProgramFiles") {
            let pf = PathBuf::from(program_files);
            push_dir(pf.join("tectonic"));
            push_dir(pf.join("MiKTeX").join("miktex").join("bin").join("x64"));
        }
        if let Ok(entries) = std::fs::read_dir(r"C:\texlive") {
            let mut tl_years: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|x| x.path()))
                .filter(|p| p.is_dir())
                .collect();
            tl_years.sort();
            tl_years.reverse();
            for year_dir in tl_years {
                push_dir(year_dir.join("bin").join("windows"));
                push_dir(year_dir.join("bin").join("win32"));
            }
        }
    }

    dirs
}

fn find_executable(engine_id: &str) -> Option<PathBuf> {
    let bin_name = binary_filename(engine_id);
    let sidecar_target_name = {
        #[cfg(windows)]
        {
            format!("{engine_id}-x86_64-pc-windows-msvc.exe")
        }
        #[cfg(target_os = "macos")]
        {
            #[cfg(target_arch = "aarch64")]
            {
                format!("{engine_id}-aarch64-apple-darwin")
            }
            #[cfg(not(target_arch = "aarch64"))]
            {
                format!("{engine_id}-x86_64-apple-darwin")
            }
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            format!("{engine_id}-x86_64-unknown-linux-gnu")
        }
    };

    for dir in candidate_search_dirs() {
        let candidate = dir.join(&bin_name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        let sidecar_candidate = dir.join(&sidecar_target_name);
        if is_executable_file(&sidecar_candidate) {
            return Some(sidecar_candidate);
        }
    }

    None
}

async fn probe_compiler(spec: &EngineSpec) -> CompilerInfo {
    let Some(bin_path) = find_executable(spec.id) else {
        return CompilerInfo {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            available: false,
            version: None,
            executable_path: None,
            is_rust_native: spec.is_rust_native,
        };
    };

    let mut cmd = tokio::process::Command::new(&bin_path);
    configure_tokio_command(&mut cmd);
    cmd.arg("--version");

    match cmd.output().await {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = if stdout.trim().is_empty() {
                stderr.as_ref()
            } else {
                stdout.as_ref()
            };
            let version = combined
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(|s| s.to_string());

            CompilerInfo {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                available: true,
                version,
                executable_path: Some(path_to_clean_string(&bin_path)),
                is_rust_native: spec.is_rust_native,
            }
        }
        _ => CompilerInfo {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            available: false,
            version: None,
            executable_path: None,
            is_rust_native: spec.is_rust_native,
        },
    }
}

#[tauri::command]
pub async fn detect_compilers() -> Result<Vec<CompilerInfo>, String> {
    let (c1, c2, c3) = tokio::join!(
        probe_compiler(&ENGINE_SPECS[1]),
        probe_compiler(&ENGINE_SPECS[2]),
        probe_compiler(&ENGINE_SPECS[3]),
    );
    Ok(vec![embedded_tectonic_info(), c1, c2, c3])
}

fn resolve_entry_file(root_canon: &Path, entry_file: &str) -> Result<(PathBuf, String), String> {
    let trimmed = entry_file.trim();
    if trimmed.is_empty() {
        return Err("Entry file path cannot be empty.".to_string());
    }

    let raw_entry = Path::new(trimmed);
    if !raw_entry.is_absolute()
        && raw_entry
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(format!(
            "Entry file '{}' cannot traverse outside workspace root.",
            entry_file
        ));
    }

    let candidate = if raw_entry.is_absolute() {
        raw_entry.to_path_buf()
    } else {
        root_canon.join(raw_entry)
    };

    let entry_canon = canonicalize_clean(&candidate)?;
    if !entry_canon.starts_with(root_canon) {
        return Err(format!(
            "Entry file '{}' is outside workspace root '{}'.",
            entry_canon.display(),
            root_canon.display()
        ));
    }
    if !entry_canon.is_file() {
        return Err(format!(
            "Entry file '{}' is not a regular file.",
            entry_canon.display()
        ));
    }

    let rel_path = entry_canon
        .strip_prefix(root_canon)
        .map(to_unix_rel_path)
        .map_err(|e| format!("Failed to resolve relative entry path: {e}"))?;

    if rel_path.is_empty() {
        return Err("Invalid entry file path.".to_string());
    }

    Ok((entry_canon, rel_path))
}

enum ResolvedEngine {
    EmbeddedTectonic,
    ExternalCli { id: String, bin_path: PathBuf },
}

async fn resolve_engine(engine_pref: &str) -> Result<ResolvedEngine, String> {
    let requested = engine_pref.trim().to_ascii_lowercase();
    if requested.is_empty() || requested == "auto" || requested == "tectonic" {
        return Ok(ResolvedEngine::EmbeddedTectonic);
    }

    let Some(spec) = ENGINE_SPECS
        .iter()
        .find(|s| s.id == requested && !s.is_rust_native)
    else {
        return Err(format!(
            "Unsupported compiler engine '{}'. Expected one of: auto, tectonic, pdflatex, xelatex, lualatex.",
            engine_pref
        ));
    };

    let bin = find_executable(spec.id).ok_or_else(|| {
        format!(
            "Requested compiler '{}' ({}) is not installed or not on PATH.",
            spec.name, spec.id
        )
    })?;

    Ok(ResolvedEngine::ExternalCli {
        id: spec.id.to_string(),
        bin_path: bin,
    })
}

#[derive(Default)]
struct BufferStatusBackend {
    messages: Vec<String>,
    error_log_dump: String,
}

impl StatusBackend for BufferStatusBackend {
    fn report(&mut self, kind: MessageKind, args: fmt::Arguments, err: Option<&tectonic::Error>) {
        let prefix = match kind {
            MessageKind::Note => "note:",
            MessageKind::Warning => "warning:",
            MessageKind::Error => "error:",
        };
        self.messages.push(format!("{prefix} {args}"));

        if let Some(e) = err {
            for item in e.chain() {
                self.messages.push(format!("caused by: {item}"));
            }
        }
    }

    fn report_error(&mut self, err: &tectonic::Error) {
        let mut prefix = "error:";
        for item in err.chain() {
            self.messages.push(format!("{prefix} {item}"));
            prefix = "caused by:";
        }
    }

    fn note_highlighted(&mut self, before: &str, highlighted: &str, after: &str) {
        self.messages
            .push(format!("note: {before}{highlighted}{after}"));
    }

    fn dump_error_logs(&mut self, output: &[u8]) {
        let text = String::from_utf8_lossy(output);
        if !text.trim().is_empty() {
            if !self.error_log_dump.is_empty() && !self.error_log_dump.ends_with('\n') {
                self.error_log_dump.push('\n');
            }
            self.error_log_dump.push_str(&text);
        }
    }
}

async fn compile_with_embedded_tectonic(
    root_canon: PathBuf,
    entry_rel: String,
    entry_stem: String,
    build_dir: PathBuf,
) -> Result<(bool, Option<Vec<u8>>, Option<PathBuf>, String), String> {
    tokio::task::spawn_blocking(move || {
        // Tectonic's underlying C/C++ XeTeX and xdvipdfmx engines rely on global mutable state.
        let _guard = EMBEDDED_TECTONIC_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let config = tectonic::config::PersistentConfig::open(false)
            .map_err(|e| format!("Failed to open Tectonic configuration: {e:#}"))?;
        let bundle = config
            .default_bundle(false)
            .map_err(|e| format!("Failed to load default Tectonic resource bundle: {e:#}"))?;
        let format_cache_path = config
            .format_cache_path()
            .map_err(|e| format!("Failed to resolve Tectonic format cache path: {e:#}"))?;

        let mut status = BufferStatusBackend::default();
        let tex_input_name = format!("{entry_stem}.tex");
        let primary_input = root_canon.join(&entry_rel);
        let pdf_file = build_dir.join(format!("{entry_stem}.pdf"));
        let log_file = build_dir.join(format!("{entry_stem}.log"));

        let security = SecuritySettings::new(SecurityStance::MaybeAllowInsecures);
        let mut builder = tectonic::driver::ProcessingSessionBuilder::new_with_security(security);
        let mut unstables = tectonic::unstable_opts::UnstableOptions::default();
        unstables.extra_search_paths.push(build_dir.clone());
        if let Some(rel_parent) = Path::new(&entry_rel).parent() {
            if !rel_parent.as_os_str().is_empty() {
                let nested_build = build_dir.join(rel_parent);
                if nested_build != build_dir {
                    unstables.extra_search_paths.push(nested_build);
                }
            }
        }

        builder
            .bundle(bundle)
            .format_cache_path(format_cache_path)
            .filesystem_root(&root_canon)
            .primary_input_path(&primary_input)
            .tex_input_name(&tex_input_name)
            .format_name("latex")
            .output_dir(&build_dir)
            .output_format(tectonic::driver::OutputFormat::Pdf)
            .keep_logs(true)
            .keep_intermediates(true)
            .print_stdout(false)
            .synctex(true)
            .build_date_from_env(false)
            .unstables(unstables)
            .shell_escape_disabled();

        let mut sess = builder
            .create(&mut status)
            .map_err(|e| format!("Failed to initialize Tectonic processing session: {e:#}"))?;

        let run_result = sess.run(&mut status);
        let stdout_bytes = sess.get_stdout_content();
        let stdout_str = String::from_utf8_lossy(&stdout_bytes).into_owned();

        if let Err(err) = &run_result {
            if let Some(engine_err) = err.downcast_ref::<tectonic::errors::EngineError>() {
                if stdout_bytes.is_empty() {
                    tectonic::tt_error!(
                        status,
                        "something bad happened inside {}, but no output was logged",
                        engine_err.engine()
                    );
                } else {
                    tectonic::tt_error!(
                        status,
                        "something bad happened inside {}; its output follows:",
                        engine_err.engine()
                    );
                    status.dump_error_logs(&stdout_bytes);
                }
            }
            status.report_error(err);
        }

        let engine_log = std::fs::read(&log_file)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();

        let mut combined_sections = Vec::new();
        if !status.messages.is_empty() {
            combined_sections.push(status.messages.join("\n"));
        }
        if !stdout_str.trim().is_empty() {
            combined_sections.push(stdout_str.trim_end().to_string());
        }
        if !status.error_log_dump.trim().is_empty()
            && status.error_log_dump.trim() != stdout_str.trim()
        {
            combined_sections.push(status.error_log_dump.trim_end().to_string());
        }
        if !engine_log.trim().is_empty() {
            combined_sections.push(format!(
                "--- TeX Log ({entry_stem}.log) ---\n{}",
                engine_log.trim_end()
            ));
        }
        let log_output = combined_sections.join("\n\n");

        if run_result.is_ok() && pdf_file.is_file() {
            let bytes = std::fs::read(&pdf_file).map_err(|e| {
                format!(
                    "Compilation succeeded but failed to read PDF '{}': {}",
                    pdf_file.display(),
                    e
                )
            })?;
            Ok((true, Some(bytes), Some(pdf_file), log_output))
        } else {
            Ok((false, None, None, log_output))
        }
    })
    .await
    .map_err(|e| format!("Embedded Tectonic task failed: {e}"))?
}

struct DiagnosticRegexes {
    file_line_err: Regex,
    tectonic_diag: Regex,
    tectonic_note_loc: Regex,
    latex_warning: Regex,
    box_warning: Regex,
    line_num: Regex,
    input_line: Regex,
    tex_file_token: Regex,
}

fn regexes() -> &'static DiagnosticRegexes {
    static RE: OnceLock<DiagnosticRegexes> = OnceLock::new();
    RE.get_or_init(|| DiagnosticRegexes {
        file_line_err: Regex::new(
            r"^(?:\./)?([^\s:]+\.(?:tex|sty|cls|bib|ltx|bbl)):(\d+):\s*(.+)$",
        )
        .unwrap(),
        tectonic_diag: Regex::new(
            r"^(error|warning):\s+(?:([^\s:]+\.(?:tex|sty|cls|bib|ltx|bbl)):(\d+):\s*)?(.+)$",
        )
        .unwrap(),
        tectonic_note_loc: Regex::new(
            r"^(?:note|caused by|at):\s+(?:\./)?([^\s:]+\.(?:tex|sty|cls|bib|ltx)):(\d+)",
        )
        .unwrap(),
        latex_warning: Regex::new(
            r"^(?:LaTeX(?:\s+Font)?|Package\s+[^\s]+|Class\s+[^\s]+|BibTeX)\s+Warning:\s*(.+)$",
        )
        .unwrap(),
        box_warning: Regex::new(
            r"^((?:Overfull|Underfull)\s+\\[hv]box\b.+?at lines?\s+(\d+)(?:--\d+)?.*)$",
        )
        .unwrap(),
        line_num: Regex::new(r"^l\.(\d+)\b(.*)$").unwrap(),
        input_line: Regex::new(r"on input line (\d+)").unwrap(),
        tex_file_token: Regex::new(r"\((?:\./)?([^\s()]+\.(?:tex|sty|cls|bib|ltx))").unwrap(),
    })
}

fn normalize_diag_file(raw: &str) -> String {
    let trimmed = raw.trim().trim_start_matches("./").replace('\\', "/");
    trimmed
}

fn push_unique_diagnostic(
    out: &mut Vec<LatexDiagnostic>,
    seen: &mut HashSet<(String, Option<String>, Option<u32>, String)>,
    diag: LatexDiagnostic,
) {
    let key = (
        diag.severity.clone(),
        diag.file.clone(),
        diag.line,
        diag.message.clone(),
    );
    if seen.insert(key) {
        out.push(diag);
    }
}

/// Parses TeX/LaTeX/Tectonic stdout, stderr, and `.log` text into structured diagnostics.
pub fn parse_diagnostics(log_text: &str, default_file: Option<&str>) -> Vec<LatexDiagnostic> {
    let re = regexes();
    let lines: Vec<&str> = log_text.lines().collect();
    let mut diagnostics = Vec::new();
    let mut seen = HashSet::new();

    let mut current_file: Option<String> = default_file.map(normalize_diag_file);
    let mut i = 0usize;

    while i < lines.len() {
        let raw_line = lines[i];
        let trimmed = raw_line.trim();

        if let Some(caps) = re.tex_file_token.captures(trimmed) {
            if let Some(m) = caps.get(1) {
                current_file = Some(normalize_diag_file(m.as_str()));
            }
        }

        // 1. Classic TeX `! <Error>` followed by `l.<num>` within the next few lines
        if let Some(err_msg) = trimmed.strip_prefix("! ") {
            let mut msg = err_msg.trim().to_string();
            let mut found_line = None;

            for lookahead in (i + 1)..lines.len().min(i + 8) {
                let next_trimmed = lines[lookahead].trim();
                if next_trimmed.starts_with("! ") {
                    break;
                }
                if let Some(caps) = re.line_num.captures(next_trimmed) {
                    found_line = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok());
                    let snippet = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                    if !snippet.is_empty() && !msg.contains(snippet) {
                        msg = format!("{msg} ({snippet})");
                    }
                    break;
                }
            }

            push_unique_diagnostic(
                &mut diagnostics,
                &mut seen,
                LatexDiagnostic {
                    severity: "error".to_string(),
                    file: current_file.clone(),
                    line: found_line,
                    message: msg,
                },
            );
            i += 1;
            continue;
        }

        // 2. `-file-line-error` format: `path/file.tex:42: Error message`
        if let Some(caps) = re.file_line_err.captures(trimmed) {
            let file = normalize_diag_file(&caps[1]);
            let line_no = caps[2].parse::<u32>().ok();
            let msg = caps[3].trim().to_string();
            let severity = if msg.to_ascii_lowercase().contains("warning") {
                "warning"
            } else {
                "error"
            };
            current_file = Some(file.clone());
            push_unique_diagnostic(
                &mut diagnostics,
                &mut seen,
                LatexDiagnostic {
                    severity: severity.to_string(),
                    file: Some(file),
                    line: line_no,
                    message: msg,
                },
            );
            i += 1;
            continue;
        }

        // 3. Tectonic CLI messages: `error: ...` / `warning: ...`
        if let Some(caps) = re.tectonic_diag.captures(trimmed) {
            let severity = caps[1].to_ascii_lowercase();
            let mut file = caps.get(2).map(|m| normalize_diag_file(m.as_str()));
            let mut line_no = caps
                .get(3)
                .and_then(|m| m.as_str().parse::<u32>().ok());
            let msg = caps[4].trim().to_string();

            // Skip generic summary wrapper if we already captured concrete TeX errors
            let is_generic_halt = severity == "error"
                && msg.starts_with("halted on a potentially-recoverable error");

            if !is_generic_halt {
                if file.is_none() && line_no.is_none() {
                    if let Some(next_line) = lines.get(i + 1) {
                        if let Some(loc_caps) = re.tectonic_note_loc.captures(next_line.trim()) {
                            file = Some(normalize_diag_file(&loc_caps[1]));
                            line_no = loc_caps[2].parse::<u32>().ok();
                        }
                    }
                }
                if file.is_none() {
                    file = current_file.clone();
                }
                if line_no.is_none() {
                    line_no = re
                        .input_line
                        .captures(&msg)
                        .and_then(|c| c[1].parse::<u32>().ok());
                }

                push_unique_diagnostic(
                    &mut diagnostics,
                    &mut seen,
                    LatexDiagnostic {
                        severity,
                        file,
                        line: line_no,
                        message: msg,
                    },
                );
            }
            i += 1;
            continue;
        }

        // 4. LaTeX / Package / Class warnings (possibly spanning wrapped continuation lines)
        if let Some(caps) = re.latex_warning.captures(trimmed) {
            let mut full_msg = caps[1].trim().to_string();
            let mut j = i + 1;
            while j < lines.len().min(i + 4) {
                let cont = lines[j].trim();
                if cont.starts_with('(') && !cont.starts_with("(./") {
                    let cleaned = cont
                        .trim_start_matches(|c: char| c == '(' || c.is_whitespace())
                        .trim();
                    if !cleaned.is_empty() {
                        full_msg.push(' ');
                        full_msg.push_str(cleaned);
                    }
                    j += 1;
                } else {
                    break;
                }
            }

            let line_no = re
                .input_line
                .captures(&full_msg)
                .and_then(|c| c[1].parse::<u32>().ok());

            push_unique_diagnostic(
                &mut diagnostics,
                &mut seen,
                LatexDiagnostic {
                    severity: "warning".to_string(),
                    file: current_file.clone(),
                    line: line_no,
                    message: full_msg,
                },
            );
            i = j;
            continue;
        }

        // 5. Overfull / Underfull box notices
        if let Some(caps) = re.box_warning.captures(trimmed) {
            let line_no = caps[2].parse::<u32>().ok();
            push_unique_diagnostic(
                &mut diagnostics,
                &mut seen,
                LatexDiagnostic {
                    severity: "info".to_string(),
                    file: current_file.clone(),
                    line: line_no,
                    message: caps[1].trim().to_string(),
                },
            );
        }

        i += 1;
    }

    diagnostics
}

#[tauri::command]
pub async fn compile_latex(request: CompileRequest) -> Result<CompileResponse, String> {
    let started = Instant::now();

    let raw_root = request.root_dir.trim();
    if raw_root.is_empty() {
        return Err("Workspace root directory cannot be empty.".to_string());
    }
    let root_canon = canonicalize_clean(Path::new(raw_root))?;
    if !root_canon.is_dir() {
        return Err(format!(
            "Workspace root '{}' is not a directory.",
            root_canon.display()
        ));
    }

    let (entry_canon, entry_rel) = resolve_entry_file(&root_canon, &request.entry_file)?;
    let entry_stem = entry_canon
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Invalid entry file name '{}'.", entry_canon.display()))?
        .to_string();

    let resolved_engine = resolve_engine(&request.engine).await?;

    let build_dir = root_canon.join(".bex-build");
    tokio::fs::create_dir_all(&build_dir).await.map_err(|e| {
        format!(
            "Failed to create build directory '{}': {}",
            build_dir.display(),
            e
        )
    })?;

    let rel_parent = Path::new(&entry_rel)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf);

    if let Some(ref rp) = rel_parent {
        let nested_build = build_dir.join(rp);
        let _ = tokio::fs::create_dir_all(&nested_build).await;
    }

    let pdf_file = build_dir.join(format!("{entry_stem}.pdf"));
    let log_file = build_dir.join(format!("{entry_stem}.log"));

    // Clear ONLY ephemeral per-run output files so a failed compilation never returns stale
    // PDF/log/SyncTeX/XDV while preserving incremental intermediates (.aux, .toc, .bbl, .out, etc.).
    let stale_exts = ["pdf", "log", "synctex.gz", "synctex", "xdv"];
    for ext in stale_exts {
        let _ = tokio::fs::remove_file(build_dir.join(format!("{entry_stem}.{ext}"))).await;
        if let Some(ref rp) = rel_parent {
            let _ = tokio::fs::remove_file(build_dir.join(rp).join(format!("{entry_stem}.{ext}")))
                .await;
        }
    }
    crate::synctex::invalidate_synctex_cache();

    let (engine_used, success, pdf_base64, pdf_path, log_output, mut diagnostics) =
        match resolved_engine {
            ResolvedEngine::EmbeddedTectonic => {
                let (ok, pdf_bytes, out_pdf_path, log_output) = compile_with_embedded_tectonic(
                    root_canon.clone(),
                    entry_rel.clone(),
                    entry_stem.clone(),
                    build_dir.clone(),
                )
                .await?;
                let diagnostics = parse_diagnostics(&log_output, Some(&entry_rel));
                let (pdf_base64, pdf_path) = if ok {
                    (
                        pdf_bytes.as_deref().map(|b| BASE64_STANDARD.encode(b)),
                        out_pdf_path.as_deref().map(path_to_clean_string),
                    )
                } else {
                    (None, None)
                };
                (
                    "tectonic".to_string(),
                    ok,
                    pdf_base64,
                    pdf_path,
                    log_output,
                    diagnostics,
                )
            }
            ResolvedEngine::ExternalCli { id, bin_path } => {
                let mut cmd = tokio::process::Command::new(&bin_path);
                configure_tokio_command(&mut cmd);
                cmd.current_dir(&root_canon);
                cmd.args([
                    "-synctex=1",
                    "-interaction=nonstopmode",
                    "-file-line-error",
                    "-output-directory=.bex-build",
                    &entry_rel,
                ]);

                let output = cmd.output().await.map_err(|e| {
                    format!(
                        "Failed to execute compiler '{}' ({}): {}",
                        id,
                        bin_path.display(),
                        e
                    )
                })?;

                let stdout_str = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr_str = String::from_utf8_lossy(&output.stderr).into_owned();
                let engine_log = tokio::fs::read(&log_file)
                    .await
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .unwrap_or_default();

                let mut combined_sections = Vec::new();
                if !stdout_str.trim().is_empty() {
                    combined_sections.push(stdout_str.trim_end().to_string());
                }
                if !stderr_str.trim().is_empty() {
                    combined_sections.push(stderr_str.trim_end().to_string());
                }
                if !engine_log.trim().is_empty() {
                    combined_sections.push(format!(
                        "--- TeX Log ({entry_stem}.log) ---\n{}",
                        engine_log.trim_end()
                    ));
                }
                let log_output = combined_sections.join("\n\n");

                let diagnostics = parse_diagnostics(&log_output, Some(&entry_rel));
                let status_ok = output.status.success();
                let pdf_exists = tokio::fs::try_exists(&pdf_file).await.unwrap_or(false);
                let ok = status_ok && pdf_exists;

                let (pdf_base64, pdf_path) = if ok {
                    let bytes = tokio::fs::read(&pdf_file).await.map_err(|e| {
                        format!(
                            "Compilation succeeded but failed to read PDF '{}': {}",
                            pdf_file.display(),
                            e
                        )
                    })?;
                    (
                        Some(BASE64_STANDARD.encode(&bytes)),
                        Some(path_to_clean_string(&pdf_file)),
                    )
                } else {
                    (None, None)
                };

                (id, ok, pdf_base64, pdf_path, log_output, diagnostics)
            }
        };

    // Keep `.bex-build` compact by removing intermediate `.xdv` files after compilation.
    let _ = tokio::fs::remove_file(build_dir.join(format!("{entry_stem}.xdv"))).await;
    if let Some(ref rp) = rel_parent {
        let _ = tokio::fs::remove_file(build_dir.join(rp).join(format!("{entry_stem}.xdv"))).await;
    }

    crate::synctex::invalidate_synctex_cache();

    let synctex_available = success && has_synctex_artifact(&build_dir, &entry_stem, rel_parent.as_deref());

    if !success && !diagnostics.iter().any(|d| d.severity == "error") {
        let fallback_msg = log_output
            .lines()
            .map(str::trim)
            .rfind(|l| !l.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Compilation with '{}' failed.", engine_used));
        diagnostics.push(LatexDiagnostic {
            severity: "error".to_string(),
            file: Some(entry_rel.clone()),
            line: None,
            message: fallback_msg,
        });
    }

    let duration_ms = started.elapsed().as_millis() as u64;

    Ok(CompileResponse {
        success,
        pdf_base64,
        pdf_path,
        log_output,
        diagnostics,
        duration_ms,
        engine_used,
        synctex_available,
    })
}

fn has_synctex_artifact(build_dir: &Path, entry_stem: &str, rel_parent: Option<&Path>) -> bool {
    let gz = build_dir.join(format!("{entry_stem}.synctex.gz"));
    let plain = build_dir.join(format!("{entry_stem}.synctex"));
    if gz.is_file() || plain.is_file() {
        return true;
    }
    if let Some(rp) = rel_parent {
        let nested = build_dir.join(rp);
        if nested.join(format!("{entry_stem}.synctex.gz")).is_file()
            || nested.join(format!("{entry_stem}.synctex")).is_file()
        {
            return true;
        }
    }
    false
}

fn compute_dir_stats(dir: &Path) -> (u64, u64, bool) {
    let Ok(meta) = std::fs::symlink_metadata(dir) else {
        return (0, 0, false);
    };
    if !meta.is_dir() {
        return (0, 0, false);
    }

    let mut total_bytes: u64 = 0;
    let mut file_count: u64 = 0;
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(entry.path());
            } else if ft.is_file() {
                if let Ok(m) = entry.metadata() {
                    total_bytes = total_bytes.saturating_add(m.len());
                    file_count = file_count.saturating_add(1);
                }
            }
        }
    }

    (total_bytes, file_count, true)
}

fn resolve_tectonic_cache_dir() -> Result<PathBuf, String> {
    let config = tectonic::config::PersistentConfig::open(false)
        .map_err(|e| format!("Failed to open Tectonic configuration: {e:#}"))?;
    let format_cache = config
        .format_cache_path()
        .map_err(|e| format!("Failed to resolve Tectonic format cache path: {e:#}"))?;
    let parent = format_cache
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Failed to resolve Tectonic cache root directory.".to_string())?;
    Ok(parent)
}

fn validate_safe_tectonic_cache_dir(dir: &Path) -> Result<(), String> {
    if dir.as_os_str().is_empty() || dir.parent().is_none() || dir.components().count() < 3 {
        return Err(format!(
            "Refusing to clear unsafe Tectonic cache directory '{}'.",
            dir.display()
        ));
    }
    let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
        return Err(format!(
            "Refusing to clear Tectonic cache directory '{}' without a directory name.",
            dir.display()
        ));
    };
    if name.is_empty() || name == "." || name == ".." {
        return Err(format!(
            "Refusing to clear invalid Tectonic cache directory '{}'.",
            dir.display()
        ));
    }

    let check_candidate = canonicalize_clean(dir).unwrap_or_else(|_| dir.to_path_buf());
    for env_key in ["HOME", "USERPROFILE", "XDG_CACHE_HOME", "LOCALAPPDATA"] {
        if let Some(val) = std::env::var_os(env_key) {
            if !val.is_empty() {
                let p = PathBuf::from(val);
                let p_canon = canonicalize_clean(&p).unwrap_or(p);
                if check_candidate == p_canon
                    || check_candidate == p_canon.join("Library")
                    || check_candidate == p_canon.join("Library").join("Caches")
                    || check_candidate == p_canon.join(".cache")
                {
                    return Err(format!(
                        "Refusing to clear protected directory '{}'.",
                        dir.display()
                    ));
                }
            }
        }
    }

    Ok(())
}

fn collect_cache_info_sync(root_dir: Option<&str>) -> Result<CompileCacheInfo, String> {
    let project_cache = match root_dir.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => {
            let root_canon = canonicalize_clean(Path::new(raw))?;
            if !root_canon.is_dir() {
                return Err(format!(
                    "Workspace root '{}' is not a directory.",
                    root_canon.display()
                ));
            }
            let build_dir = root_canon.join(".bex-build");
            let (bytes, file_count, exists) = compute_dir_stats(&build_dir);
            CacheBucketInfo {
                path: Some(path_to_clean_string(&build_dir)),
                bytes,
                file_count,
                exists,
            }
        }
        None => CacheBucketInfo {
            path: None,
            bytes: 0,
            file_count: 0,
            exists: false,
        },
    };

    let tectonic_dir = resolve_tectonic_cache_dir()?;
    let (tec_bytes, tec_files, tec_exists) = compute_dir_stats(&tectonic_dir);
    let tectonic_cache = CacheBucketInfo {
        path: Some(path_to_clean_string(&tectonic_dir)),
        bytes: tec_bytes,
        file_count: tec_files,
        exists: tec_exists,
    };

    let total_bytes = project_cache.bytes.saturating_add(tectonic_cache.bytes);
    Ok(CompileCacheInfo {
        project_cache,
        tectonic_cache,
        total_bytes,
    })
}

#[tauri::command]
pub async fn get_compile_cache_info(root_dir: Option<String>) -> Result<CompileCacheInfo, String> {
    tokio::task::spawn_blocking(move || collect_cache_info_sync(root_dir.as_deref()))
        .await
        .map_err(|e| format!("Failed to inspect compile cache: {e}"))?
}

#[tauri::command]
pub async fn clear_compile_cache(
    root_dir: Option<String>,
    scope: String,
) -> Result<CompileCacheInfo, String> {
    tokio::task::spawn_blocking(move || {
        let normalized_scope = scope.trim().to_ascii_lowercase();
        let (clear_project, clear_tectonic) = match normalized_scope.as_str() {
            "project" => (true, false),
            "tectonic" => (false, true),
            "all" => (true, true),
            _ => {
                return Err(format!(
                    "Invalid cache scope '{}'. Expected 'project', 'tectonic', or 'all'.",
                    scope
                ))
            }
        };

        if clear_project {
            let trimmed_root = root_dir.as_deref().map(str::trim).filter(|s| !s.is_empty());
            if let Some(raw) = trimmed_root {
                let root_canon = canonicalize_clean(Path::new(raw))?;
                if !root_canon.is_dir() {
                    return Err(format!(
                        "Workspace root '{}' is not a directory.",
                        root_canon.display()
                    ));
                }
                let build_dir = root_canon.join(".bex-build");
                if build_dir.file_name() == Some(std::ffi::OsStr::new(".bex-build"))
                    && build_dir.exists()
                {
                    std::fs::remove_dir_all(&build_dir).map_err(|e| {
                        format!(
                            "Failed to remove project build cache '{}': {}",
                            build_dir.display(),
                            e
                        )
                    })?;
                }
                crate::synctex::invalidate_synctex_cache();
            } else if normalized_scope == "project" {
                return Err("Workspace root directory is required to clear project cache.".to_string());
            }
        }

        if clear_tectonic {
            let _guard = EMBEDDED_TECTONIC_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let tectonic_dir = resolve_tectonic_cache_dir()?;
            validate_safe_tectonic_cache_dir(&tectonic_dir)?;
            if tectonic_dir.exists() {
                std::fs::remove_dir_all(&tectonic_dir).map_err(|e| {
                    format!(
                        "Failed to clear Tectonic cache '{}': {}",
                        tectonic_dir.display(),
                        e
                    )
                })?;
                std::fs::create_dir_all(&tectonic_dir).map_err(|e| {
                    format!(
                        "Failed to recreate Tectonic cache directory '{}': {}",
                        tectonic_dir.display(),
                        e
                    )
                })?;
            }
        }

        collect_cache_info_sync(root_dir.as_deref())
    })
    .await
    .map_err(|e| format!("Failed to clear compile cache: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::seed_sample_project;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(label: &str) -> Self {
            let seq = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bex-comp-test-{}-{}-{}",
                label,
                std::process::id(),
                seq
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn parses_file_line_classic_tectonic_and_warnings() {
        let sample_log = r#"
(./main.tex (./sections/intro.tex
./sections/intro.tex:14: Undefined control sequence.
! Undefined control sequence.
l.14 \badmacro
              {arg}
LaTeX Warning: Reference `sec:missing' on page 1 undefined on input line 27.
Package hyperref Warning: Token not allowed in a PDF string (Unicode):
(hyperref)                removing `math shift' on input line 33.
Overfull \hbox (4.12pt too wide) in paragraph at lines 40--42
error: main.tex:9: Emergency stop.
warning: deprecated macro used on input line 50
"#;

        let diags = parse_diagnostics(sample_log, Some("main.tex"));
        assert!(diags.iter().any(|d| {
            d.severity == "error"
                && d.file.as_deref() == Some("sections/intro.tex")
                && d.line == Some(14)
                && d.message.contains("Undefined control sequence")
        }));
        assert!(diags.iter().any(|d| {
            d.severity == "warning"
                && d.line == Some(27)
                && d.message.contains("sec:missing")
        }));
        assert!(diags.iter().any(|d| {
            d.severity == "warning"
                && d.line == Some(33)
                && d.message.contains("Token not allowed")
        }));
        assert!(diags.iter().any(|d| {
            d.severity == "info"
                && d.line == Some(40)
                && d.message.contains("Overfull")
        }));
        assert!(diags.iter().any(|d| {
            d.severity == "error"
                && d.file.as_deref() == Some("main.tex")
                && d.line == Some(9)
                && d.message.contains("Emergency stop")
        }));
        assert!(diags.iter().any(|d| {
            d.severity == "warning"
                && d.line == Some(50)
                && d.message.contains("deprecated macro")
        }));
    }

    #[test]
    fn rejects_path_traversal_outside_root() {
        let dir = TempDirGuard::new("traversal");
        std::fs::write(dir.path.join("main.tex"), "\\documentclass{article}").unwrap();
        let root_canon = canonicalize_clean(&dir.path).unwrap();

        assert!(resolve_entry_file(&root_canon, "../secret.tex").is_err());
        assert!(resolve_entry_file(&root_canon, "").is_err());
        assert!(resolve_entry_file(&root_canon, "/etc/passwd").is_err());
        assert_eq!(
            resolve_entry_file(&root_canon, "main.tex").unwrap().1,
            "main.tex"
        );
    }

    #[tokio::test]
    async fn compiles_sample_project_when_tectonic_is_available() {
        let compilers = detect_compilers().await.expect("detect compilers");
        let tectonic = compilers
            .iter()
            .find(|c| c.id == "tectonic")
            .expect("tectonic compiler entry");
        assert!(tectonic.available);
        assert_eq!(tectonic.name, "Tectonic (Embedded Rust Engine)");
        assert_eq!(
            tectonic.version.as_deref(),
            Some("Tectonic 0.17.0 (Embedded Rust Library)")
        );
        assert_eq!(
            tectonic.executable_path.as_deref(),
            Some("embedded://tectonic-0.17.0")
        );
        assert!(tectonic.is_rust_native);

        let dir = TempDirGuard::new("tectonic-live");
        seed_sample_project(&dir.path).expect("seed sample project");

        let resp = compile_latex(CompileRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "main.tex".to_string(),
            engine: "auto".to_string(),
        })
        .await
        .expect("compile_latex call");

        assert!(
            resp.success,
            "Expected tectonic compilation to succeed, log:\n{}",
            resp.log_output
        );
        assert_eq!(resp.engine_used, "tectonic");
        assert!(resp.synctex_available);
        let b64 = resp.pdf_base64.expect("pdf_base64 present");
        let pdf_bytes = BASE64_STANDARD.decode(&b64).expect("valid base64");
        assert!(
            pdf_bytes.starts_with(b"%PDF"),
            "Compiled output must be a valid PDF header"
        );
        assert!(resp.pdf_path.as_deref().unwrap_or("").ends_with("main.pdf"));
    }

    #[tokio::test]
    async fn reports_diagnostics_on_invalid_latex_with_tectonic() {
        let compilers = detect_compilers().await.expect("detect compilers");
        assert!(compilers.iter().any(|c| c.id == "tectonic" && c.available));

        let dir = TempDirGuard::new("tectonic-error");
        std::fs::write(
            dir.path.join("broken.tex"),
            "\\documentclass{article}\n\\begin{document}\n\\undefinedcommandhere\n\\end{document}\n",
        )
        .unwrap();

        let resp = compile_latex(CompileRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "broken.tex".to_string(),
            engine: "tectonic".to_string(),
        })
        .await
        .expect("compile_latex response");

        assert!(!resp.success);
        assert!(!resp.synctex_available);
        assert_eq!(resp.engine_used, "tectonic");
        assert!(resp.pdf_base64.is_none());
        assert!(
            resp.diagnostics
                .iter()
                .any(|d| d.severity == "error" && d.line == Some(3)),
            "Expected error diagnostic on line 3, got {:?}\nLog:\n{}",
            resp.diagnostics,
            resp.log_output
        );
    }

    #[tokio::test]
    async fn tectonic_excludes_percent_commented_lines() {
        let compilers = detect_compilers().await.expect("detect compilers");
        assert!(compilers.iter().any(|c| c.id == "tectonic" && c.available));

        let dir = TempDirGuard::new("tectonic-comments");
        let commented_doc = concat!(
            "\\documentclass{article}\n",
            "\\begin{document}\n",
            "Hello 100\\% valid text.\n",
            "% \\undefinedmacrothatwouldfail\n",
            "% Multi-line commented block line 1\n",
            "% \\anotherundefinedmacro{arg}\n",
            "\\end{document}\n",
        );
        let tex_path = dir.path.join("comments.tex");
        std::fs::write(&tex_path, commented_doc).unwrap();

        let res = compile_latex(CompileRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "comments.tex".to_string(),
            engine: "tectonic".to_string(),
        })
        .await
        .expect("compile_latex commented response");

        assert!(
            res.success,
            "Expected Tectonic compilation to succeed when invalid macros are %-commented, log:\n{}",
            res.log_output
        );
        assert_eq!(res.engine_used, "tectonic");
        assert!(
            res.pdf_base64.is_some(),
            "Expected compiled PDF when invalid macros are %-commented"
        );

        let uncommented_doc = commented_doc.replacen(
            "% \\undefinedmacrothatwouldfail",
            "\\undefinedmacrothatwouldfail",
            1,
        );
        std::fs::write(&tex_path, uncommented_doc).unwrap();

        let res_uncommented = compile_latex(CompileRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "comments.tex".to_string(),
            engine: "tectonic".to_string(),
        })
        .await
        .expect("compile_latex uncommented response");

        assert!(
            !res_uncommented.success,
            "Expected removing '% ' prefix from \\undefinedmacrothatwouldfail to fail compilation"
        );
        assert!(res_uncommented.pdf_base64.is_none());
    }

    #[tokio::test]
    async fn incremental_compile_preserves_aux_removes_xdv_and_reduces_passes() {
        let dir = TempDirGuard::new("tectonic-incremental");
        seed_sample_project(&dir.path).expect("seed sample project");

        let count_tex_passes = |log: &str| -> usize {
            log.lines()
                .filter(|line| {
                    let t = line.trim();
                    t.starts_with("note: Running TeX ") || t.starts_with("note: Rerunning TeX ")
                })
                .count()
        };

        let first = compile_latex(CompileRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "main.tex".to_string(),
            engine: "tectonic".to_string(),
        })
        .await
        .expect("first compile");

        assert!(first.success, "first compile failed:\n{}", first.log_output);
        assert!(first.synctex_available);
        let build_dir = dir.path.join(".bex-build");
        assert!(build_dir.join("main.aux").is_file(), "expected .aux to be preserved");
        assert!(!build_dir.join("main.xdv").exists(), "expected .xdv to be removed");

        let first_passes = count_tex_passes(&first.log_output);
        assert!(
            first_passes >= 3,
            "expected >= 3 TeX passes on cold sample project compile, got {first_passes}\n{}",
            first.log_output
        );

        let second = compile_latex(CompileRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "main.tex".to_string(),
            engine: "tectonic".to_string(),
        })
        .await
        .expect("second compile");

        assert!(second.success, "second compile failed:\n{}", second.log_output);
        assert!(second.synctex_available);
        assert!(build_dir.join("main.aux").is_file(), "expected .aux to stay preserved");
        assert!(!build_dir.join("main.xdv").exists(), "expected .xdv to remain absent");

        let second_passes = count_tex_passes(&second.log_output);
        assert!(
            second_passes < first_passes,
            "expected incremental 2nd run to run fewer TeX passes ({second_passes} vs {first_passes})\nLog 2:\n{}",
            second.log_output
        );
    }

    #[tokio::test]
    async fn inspects_and_clears_project_compile_cache() {
        let dir = TempDirGuard::new("cache-info");
        let build_dir = dir.path.join(".bex-build");
        std::fs::create_dir_all(&build_dir).unwrap();
        std::fs::write(build_dir.join("main.aux"), b"\\relax\n").unwrap();
        std::fs::write(build_dir.join("main.pdf"), b"%PDF-1.4\n12345").unwrap();

        let root_str = dir.path.to_string_lossy().into_owned();
        let before = get_compile_cache_info(Some(root_str.clone()))
            .await
            .expect("get_compile_cache_info");
        assert!(before.project_cache.exists);
        assert_eq!(before.project_cache.file_count, 2);
        assert_eq!(before.project_cache.bytes, 7 + 14);
        assert_eq!(
            before.total_bytes,
            before.project_cache.bytes + before.tectonic_cache.bytes
        );

        let after = clear_compile_cache(Some(root_str.clone()), "project".to_string())
            .await
            .expect("clear_compile_cache project");
        assert!(!after.project_cache.exists);
        assert_eq!(after.project_cache.file_count, 0);
        assert_eq!(after.project_cache.bytes, 0);
        assert!(!build_dir.exists());

        assert!(clear_compile_cache(Some(root_str), "invalid".to_string())
            .await
            .is_err());

        let no_proj = get_compile_cache_info(None)
            .await
            .expect("get_compile_cache_info without root");
        assert!(!no_proj.project_cache.exists);
        assert!(no_proj.project_cache.path.is_none());
    }

    #[test]
    fn buffer_status_backend_captures_messages_and_logs() {
        let mut status = BufferStatusBackend::default();
        tectonic::tt_note!(status, "loading bundle");
        tectonic::tt_warning!(status, "deprecated option");
        status.note_highlighted("Running ", "TeX", " ...");
        let err = tectonic::Error::msg("root failure").context("outer failure");
        status.report_error(&err);
        status.dump_error_logs(b"! Undefined control sequence.\nl.3 \\bad\n");

        assert_eq!(
            status.messages,
            vec![
                "note: loading bundle",
                "warning: deprecated option",
                "note: Running TeX ...",
                "error: outer failure",
                "caused by: root failure",
            ]
        );
        assert!(status
            .error_log_dump
            .contains("! Undefined control sequence."));
    }
}

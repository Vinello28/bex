use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::Manager;

static TEMP_WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub abs_path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub root_path: String,
    pub name: String,
    pub files: Vec<FileNode>,
    pub default_entry_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub path: String,
    pub bytes_written: usize,
}

const IGNORED_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".bex-build",
    ".DS_Store",
    "Thumbs.db",
];

const IGNORED_EXTENSIONS: &[&str] = &[
    "aux",
    "log",
    "fls",
    "fdb_latexmk",
    "toc",
    "out",
    "blg",
    "bcf",
    "nav",
    "snm",
    "vrb",
    "lof",
    "lot",
    "synctex",
];

const SAMPLE_MAIN_TEX: &str = r#"\documentclass[11pt,a4paper]{article}

\usepackage[margin=1in]{geometry}
\usepackage{amsmath,amssymb}
\usepackage{booktabs}
\usepackage{xcolor}
\usepackage[colorlinks=true,linkcolor=teal,citecolor=teal,urlcolor=teal]{hyperref}

\title{\textbf{Bex LaTeX Studio --- Sample Paper}}
\author{Bex Research Group}
\date{\today}

\begin{document}

\maketitle

\begin{abstract}
This starter document demonstrates multi-file \LaTeX{} authoring inside
\textbf{Bex LaTeX Studio}. It combines modular section imports, mathematical
typesetting, structured tables, and Bib\TeX{} citations compiled natively
with the Tectonic Rust engine.
\end{abstract}

\input{sections/intro}
\input{sections/methodology}

\section{Conclusion}
Modular document organization paired with fast incremental compilation enables
a responsive writing workflow~\cite{knuth1984,tectonic2024}. Edit any section
and press \texttt{Cmd/Ctrl+S} to recompile and update the live PDF preview.

\bibliographystyle{plain}
\bibliography{references}

\end{document}
"#;

const SAMPLE_INTRO_TEX: &str = r#"\section{Introduction}
\label{sec:intro}

Reproducible scientific publishing relies on separating structure from styling.
Using \LaTeX{}, authors express equations, cross-references, and citations
declaratively while maintaining deterministic typographic layout~\cite{lamport1994}.

As an example of inline and display mathematics, consider the regularized
empirical risk minimization objective:
\begin{equation}
\label{eq:erm}
\mathcal{L}(\theta) = \frac{1}{N} \sum_{i=1}^{N} \ell\big(f_\theta(x_i), y_i\big)
  + \frac{\lambda}{2} \|\theta\|_2^2.
\end{equation}
Equation~\eqref{eq:erm} balances data fidelity against parameter smoothness
via the regularization hyperparameter $\lambda > 0$.
"#;

const SAMPLE_METHODOLOGY_TEX: &str = r#"\section{Methodology}
\label{sec:methodology}

Bex compiles your project into an isolated build directory (\texttt{.bex-build})
so auxiliary compiler files never clutter the workspace tree. Table~\ref{tab:engines}
summarizes the supported engines.

\begin{table}[h]
\centering
\caption{Supported \LaTeX{} compilation backends in Bex LaTeX Studio.}
\label{tab:engines}
\begin{tabular}{@{}lll@{}}
\toprule
Engine & Implementation & Key Advantage \\
\midrule
Tectonic & Rust (XeTeX-based) & Self-contained, automatic reruns \& Bib\TeX{} \\
pdfLaTeX & C / Web2C & Classic TeX Live \& MiKTeX compatibility \\
XeLaTeX & C++ / Unicode & System OpenType fonts and Unicode input \\
LuaLaTeX & C / Lua & Programmable scripting and modern typesetting \\
\bottomrule
\end{tabular}
\end{table}

Key workflow guidelines:
\begin{itemize}
  \item Keep chapters or sections in dedicated files under \texttt{sections/}.
  \item Store bibliographic entries in \texttt{references.bib}.
  \item Inspect clickable compiler diagnostics whenever warnings or errors occur.
\end{itemize}
"#;

const SAMPLE_REFERENCES_BIB: &str = r#"@book{knuth1984,
  author    = {Donald E. Knuth},
  title     = {The {\TeX}book},
  year      = {1984},
  publisher = {Addison-Wesley}
}

@book{lamport1994,
  author    = {Leslie Lamport},
  title     = {{\LaTeX}: A Document Preparation System},
  edition   = {2nd},
  year      = {1994},
  publisher = {Addison-Wesley}
}

@misc{tectonic2024,
  author    = {{The Tectonic Project}},
  title     = {Tectonic: A Modernized, Complete, Self-Contained {\TeX}/{\LaTeX} Engine},
  year      = {2024},
  url       = {https://tectonic-typesetting.github.io/}
}
"#;

/// Strips the Windows verbatim (`\\?\`) prefix so paths display and run cleanly.
pub(crate) fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{}", rest));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

/// Canonicalizes an existing filesystem path and normalizes Windows verbatim prefixes.
pub(crate) fn canonicalize_clean(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path)
        .map(strip_verbatim_prefix)
        .map_err(|e| format!("Failed to resolve path '{}': {}", path.display(), e))
}

/// Converts an OS path into a clean string representation.
pub(crate) fn path_to_clean_string(path: &Path) -> String {
    strip_verbatim_prefix(path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// Normalizes a relative path into forward-slash separated segments.
pub(crate) fn to_unix_rel_path(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(seg) => Some(seg.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn should_ignore_entry(name: &str, is_dir: bool) -> bool {
    if IGNORED_NAMES.contains(&name) {
        return true;
    }
    if is_dir {
        return false;
    }

    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".synctex.gz") || lower.ends_with(".run.xml") {
        return true;
    }

    if let Some(ext) = Path::new(&lower).extension().and_then(|e| e.to_str()) {
        return IGNORED_EXTENSIONS.contains(&ext);
    }

    false
}

fn collect_file_nodes(
    dir: &Path,
    rel_prefix: &str,
    visited_dirs: &mut HashSet<PathBuf>,
) -> Result<Vec<FileNode>, String> {
    let canon_dir = canonicalize_clean(dir)?;
    if !visited_dirs.insert(canon_dir.clone()) {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&canon_dir).map_err(|e| {
        format!(
            "Failed to read directory '{}': {}",
            canon_dir.display(),
            e
        )
    })?;

    let mut nodes = Vec::new();
    for entry_res in entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().into_owned();
        let entry_path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let is_dir = if file_type.is_symlink() {
            entry_path.is_dir()
        } else {
            file_type.is_dir()
        };

        if should_ignore_entry(&name, is_dir) {
            continue;
        }

        let rel_path = if rel_prefix.is_empty() {
            name.clone()
        } else {
            format!("{rel_prefix}/{name}")
        };
        let abs_path = canonicalize_clean(&entry_path)
            .map(|p| path_to_clean_string(&p))
            .unwrap_or_else(|_| path_to_clean_string(&entry_path));

        if is_dir {
            let children = collect_file_nodes(&entry_path, &rel_path, visited_dirs)?;
            nodes.push(FileNode {
                name,
                path: rel_path,
                abs_path,
                is_dir: true,
                children: Some(children),
                extension: None,
            });
        } else {
            let extension = Path::new(&name)
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase());
            nodes.push(FileNode {
                name,
                path: rel_path,
                abs_path,
                is_dir: false,
                children: None,
                extension,
            });
        }
    }

    visited_dirs.remove(&canon_dir);

    nodes.sort_by(|a, b| {
        (!a.is_dir)
            .cmp(&(!b.is_dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(nodes)
}

fn collect_tex_files_in_order(nodes: &[FileNode], out: &mut Vec<FileNode>) {
    for node in nodes {
        if !node.is_dir && node.extension.as_deref() == Some("tex") {
            out.push(node.clone());
        }
    }
    for node in nodes {
        if node.is_dir {
            if let Some(children) = &node.children {
                collect_tex_files_in_order(children, out);
            }
        }
    }
}

fn detect_default_entry_file(files: &[FileNode]) -> Option<String> {
    if let Some(main) = files
        .iter()
        .find(|n| !n.is_dir && n.name.eq_ignore_ascii_case("main.tex"))
    {
        return Some(main.path.clone());
    }

    let mut tex_files = Vec::new();
    collect_tex_files_in_order(files, &mut tex_files);

    for node in &tex_files {
        if let Ok(content) = fs::read_to_string(&node.abs_path) {
            if content.contains("\\documentclass") {
                return Some(node.path.clone());
            }
        }
    }

    tex_files.into_iter().next().map(|n| n.path)
}

/// Seeds the starter multi-file LaTeX project when `main.tex` does not yet exist.
pub fn seed_sample_project(project_dir: &Path) -> Result<(), String> {
    let sections_dir = project_dir.join("sections");
    fs::create_dir_all(&sections_dir).map_err(|e| {
        format!(
            "Failed to create sample project directory '{}': {}",
            sections_dir.display(),
            e
        )
    })?;

    let main_tex = project_dir.join("main.tex");
    if main_tex.exists() {
        return Ok(());
    }

    let seed_files: &[(&Path, &str)] = &[
        (&main_tex, SAMPLE_MAIN_TEX),
        (&sections_dir.join("intro.tex"), SAMPLE_INTRO_TEX),
        (&sections_dir.join("methodology.tex"), SAMPLE_METHODOLOGY_TEX),
        (&project_dir.join("references.bib"), SAMPLE_REFERENCES_BIB),
    ];

    for (path, content) in seed_files {
        if !path.exists() {
            write_utf8_atomic(path, content)?;
        }
    }

    Ok(())
}

pub fn open_workspace_dir(root_dir: &Path) -> Result<WorkspaceInfo, String> {
    let canon_root = canonicalize_clean(root_dir)?;
    if !canon_root.is_dir() {
        return Err(format!(
            "Workspace path '{}' is not a directory.",
            canon_root.display()
        ));
    }

    let name = canon_root
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Workspace")
        .to_string();

    let mut visited = HashSet::new();
    let files = collect_file_nodes(&canon_root, "", &mut visited)?;
    let default_entry_file = detect_default_entry_file(&files);

    Ok(WorkspaceInfo {
        root_path: path_to_clean_string(&canon_root),
        name,
        files,
        default_entry_file,
    })
}

pub(crate) fn write_utf8_atomic(target: &Path, content: &str) -> Result<usize, String> {
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create parent directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
    }

    let bytes = content.as_bytes();
    let seq = TEMP_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let tmp_name = format!(".{file_name}.bex-tmp-{}-{seq}", std::process::id());
    let tmp_path = target
        .parent()
        .map(|p| p.join(&tmp_name))
        .unwrap_or_else(|| PathBuf::from(&tmp_name));

    let write_res = (|| -> Result<(), String> {
        let mut tmp_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .map_err(|e| format!("Failed to open temp file '{}': {}", tmp_path.display(), e))?;
        tmp_file
            .write_all(bytes)
            .map_err(|e| format!("Failed to write temp file '{}': {}", tmp_path.display(), e))?;
        tmp_file
            .flush()
            .map_err(|e| format!("Failed to flush temp file '{}': {}", tmp_path.display(), e))?;
        drop(tmp_file);

        fs::rename(&tmp_path, target).map_err(|e| {
            format!(
                "Failed to replace target file '{}': {}",
                target.display(),
                e
            )
        })?;
        Ok(())
    })();

    if let Err(err) = write_res {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    Ok(bytes.len())
}

fn validate_non_empty_path(raw: &str, label: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} cannot be empty."));
    }
    Ok(PathBuf::from(trimmed))
}

#[tauri::command]
pub fn init_default_workspace(app: tauri::AppHandle) -> Result<WorkspaceInfo, String> {
    let base_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("bex-latex-studio"));
    let sample_dir = base_dir.join("sample_project");
    seed_sample_project(&sample_dir)?;
    open_workspace_dir(&sample_dir)
}

#[tauri::command]
pub fn open_workspace(root_path: String) -> Result<WorkspaceInfo, String> {
    let path = validate_non_empty_path(&root_path, "Workspace path")?;
    open_workspace_dir(&path)
}

#[tauri::command]
pub fn create_new_project(root_path: String, seed_template: bool) -> Result<WorkspaceInfo, String> {
    let dir = validate_non_empty_path(&root_path, "Project path")?;
    fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "Failed to create project directory '{}': {}",
            dir.display(),
            e
        )
    })?;

    if seed_template {
        seed_sample_project(&dir)?;
    } else {
        let mut visited = HashSet::new();
        let has_tex = if let Ok(nodes) = collect_file_nodes(&dir, "", &mut visited) {
            let mut tex_files = Vec::new();
            collect_tex_files_in_order(&nodes, &mut tex_files);
            !tex_files.is_empty()
        } else {
            false
        };

        if !has_tex {
            let main_tex = dir.join("main.tex");
            let minimal_content = "\\documentclass{article}\n\n\\title{New Project}\n\\author{}\n\\date{\\today}\n\n\\begin{document}\n\n\\maketitle\n\n\\section{Introduction}\n\n\\end{document}\n";
            write_utf8_atomic(&main_tex, minimal_content)?;
        }
    }

    open_workspace_dir(&dir)
}

#[tauri::command]
pub fn list_workspace_files(root_path: String) -> Result<Vec<FileNode>, String> {
    let path = validate_non_empty_path(&root_path, "Workspace path")?;
    let canon_root = canonicalize_clean(&path)?;
    if !canon_root.is_dir() {
        return Err(format!(
            "Workspace path '{}' is not a directory.",
            canon_root.display()
        ));
    }
    let mut visited = HashSet::new();
    collect_file_nodes(&canon_root, "", &mut visited)
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    let target = validate_non_empty_path(&path, "File path")?;
    let meta = fs::metadata(&target)
        .map_err(|e| format!("Failed to access '{}': {}", target.display(), e))?;
    if meta.is_dir() {
        return Err(format!("Cannot read directory '{}' as a file.", target.display()));
    }
    fs::read_to_string(&target)
        .map_err(|e| format!("Failed to read UTF-8 file '{}': {}", target.display(), e))
}

#[tauri::command]
pub fn save_text_file(path: String, content: String) -> Result<SaveResult, String> {
    let target = validate_non_empty_path(&path, "File path")?;
    if target.is_dir() {
        return Err(format!(
            "Cannot overwrite directory '{}' as a file.",
            target.display()
        ));
    }
    let bytes_written = write_utf8_atomic(&target, &content)?;
    let clean_path = canonicalize_clean(&target)
        .map(|p| path_to_clean_string(&p))
        .unwrap_or_else(|_| path_to_clean_string(&target));

    Ok(SaveResult {
        path: clean_path,
        bytes_written,
    })
}

#[tauri::command]
pub fn create_file(path: String, is_dir: bool) -> Result<(), String> {
    let target = validate_non_empty_path(&path, "Target path")?;
    if fs::symlink_metadata(&target).is_ok() {
        return Err(format!("Path already exists: '{}'", target.display()));
    }

    if is_dir {
        fs::create_dir_all(&target).map_err(|e| {
            format!(
                "Failed to create directory '{}': {}",
                target.display(),
                e
            )
        })
    } else {
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Failed to create parent directory '{}': {}",
                        parent.display(),
                        e
                    )
                })?;
            }
        }
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map(|_| ())
            .map_err(|e| format!("Failed to create file '{}': {}", target.display(), e))
    }
}

#[tauri::command]
pub fn rename_path(old_path: String, new_path: String) -> Result<(), String> {
    let old_p = validate_non_empty_path(&old_path, "Source path")?;
    let new_p = validate_non_empty_path(&new_path, "Destination path")?;

    if fs::symlink_metadata(&old_p).is_err() {
        return Err(format!("Source path does not exist: '{}'", old_p.display()));
    }
    if old_p == new_p {
        return Ok(());
    }

    if fs::symlink_metadata(&new_p).is_ok() {
        // Allow case-only renames on case-insensitive filesystems (macOS/Windows)
        // when both paths resolve to the exact same underlying entry.
        let same_entry = match (fs::canonicalize(&old_p), fs::canonicalize(&new_p)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if !same_entry {
            return Err(format!(
                "Destination path already exists: '{}'",
                new_p.display()
            ));
        }
    }

    if let Some(parent) = new_p.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create destination parent '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
    }

    fs::rename(&old_p, &new_p).map_err(|e| {
        format!(
            "Failed to rename '{}' to '{}': {}",
            old_p.display(),
            new_p.display(),
            e
        )
    })
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let target = validate_non_empty_path(&path, "Delete path")?;
    if target.parent().is_none() {
        return Err("Refusing to delete filesystem root.".to_string());
    }
    if let Ok(canon) = fs::canonicalize(&target) {
        if canon.parent().is_none() {
            return Err("Refusing to delete filesystem root.".to_string());
        }
    }

    let meta = fs::symlink_metadata(&target)
        .map_err(|e| format!("Failed to inspect '{}': {}", target.display(), e))?;
    if meta.is_dir() {
        fs::remove_dir_all(&target)
            .map_err(|e| format!("Failed to remove directory '{}': {}", target.display(), e))
    } else {
        fs::remove_file(&target)
            .map_err(|e| format!("Failed to remove file '{}': {}", target.display(), e))
    }
}

#[tauri::command]
pub fn export_pdf(source_pdf: String, dest_path: String) -> Result<(), String> {
    let src = validate_non_empty_path(&source_pdf, "Source PDF path")?;
    let dst = validate_non_empty_path(&dest_path, "Destination PDF path")?;

    let meta = fs::metadata(&src)
        .map_err(|e| format!("Source PDF '{}' is not accessible: {}", src.display(), e))?;
    if !meta.is_file() {
        return Err(format!("Source PDF '{}' is not a file.", src.display()));
    }

    if let Some(parent) = dst.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create destination directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
    }

    fs::copy(&src, &dst).map(|_| ()).map_err(|e| {
        format!(
            "Failed to export PDF from '{}' to '{}': {}",
            src.display(),
            dst.display(),
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempWorkspace {
        dir: PathBuf,
    }

    impl TempWorkspace {
        fn new(label: &str) -> Self {
            let seq = TEMP_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "bex-ws-test-{}-{}-{}",
                label,
                std::process::id(),
                seq
            ));
            fs::create_dir_all(&dir).expect("create temp test dir");
            Self { dir }
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn lists_sorts_and_ignores_build_and_aux_artifacts() {
        let ws = TempWorkspace::new("tree");
        fs::create_dir_all(ws.dir.join(".bex-build")).unwrap();
        fs::create_dir_all(ws.dir.join(".git")).unwrap();
        fs::create_dir_all(ws.dir.join("sections")).unwrap();
        fs::create_dir_all(ws.dir.join("Appendices")).unwrap();

        fs::write(ws.dir.join(".bex-build/main.pdf"), b"%PDF").unwrap();
        fs::write(ws.dir.join(".git/HEAD"), "ref: refs/heads/main").unwrap();
        fs::write(ws.dir.join(".DS_Store"), "junk").unwrap();
        fs::write(ws.dir.join("main.tex"), "\\documentclass{article}").unwrap();
        fs::write(ws.dir.join("alpha.bib"), "@book{a,}").unwrap();
        fs::write(ws.dir.join("main.aux"), "aux").unwrap();
        fs::write(ws.dir.join("main.log"), "log").unwrap();
        fs::write(ws.dir.join("main.synctex.gz"), "gz").unwrap();
        fs::write(ws.dir.join("main.fls"), "fls").unwrap();
        fs::write(ws.dir.join("main.fdb_latexmk"), "fdb").unwrap();
        fs::write(ws.dir.join("main.toc"), "toc").unwrap();
        fs::write(ws.dir.join("main.out"), "out").unwrap();
        fs::write(ws.dir.join("sections/intro.tex"), "\\section{Intro}").unwrap();
        fs::write(ws.dir.join("sections/intro.aux"), "aux").unwrap();
        fs::write(ws.dir.join("Appendices/notes.md"), "# Notes").unwrap();

        let info = open_workspace_dir(&ws.dir).expect("open workspace");
        assert_eq!(info.default_entry_file.as_deref(), Some("main.tex"));

        // Directories come first (case-insensitive: Appendices before sections),
        // then files (alpha.bib before main.tex). Ignored entries are absent.
        let top_names: Vec<&str> = info.files.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(
            top_names,
            vec!["Appendices", "sections", "alpha.bib", "main.tex"]
        );
        assert!(info.files[0].is_dir);
        assert!(info.files[1].is_dir);
        assert!(!info.files[2].is_dir);
        assert_eq!(info.files[2].extension.as_deref(), Some("bib"));
        assert_eq!(info.files[3].extension.as_deref(), Some("tex"));

        let section_children = info.files[1].children.as_ref().expect("sections children");
        assert_eq!(section_children.len(), 1);
        assert_eq!(section_children[0].name, "intro.tex");
        assert_eq!(section_children[0].path, "sections/intro.tex");
    }

    #[test]
    fn crud_operations_and_root_delete_guard() {
        let ws = TempWorkspace::new("crud");
        let file_a = ws.dir.join("notes/draft.tex");
        let file_b = ws.dir.join("notes/renamed.tex");
        let folder = ws.dir.join("assets");

        create_file(folder.to_string_lossy().into_owned(), true).expect("create dir");
        assert!(folder.is_dir());
        assert!(create_file(folder.to_string_lossy().into_owned(), true).is_err());

        create_file(file_a.to_string_lossy().into_owned(), false).expect("create file");
        assert!(create_file(file_a.to_string_lossy().into_owned(), false).is_err());

        let save = save_text_file(
            file_a.to_string_lossy().into_owned(),
            "Hello \\LaTeX!".to_string(),
        )
        .expect("save file");
        assert_eq!(save.bytes_written, "Hello \\LaTeX!".len());

        let content = read_text_file(file_a.to_string_lossy().into_owned()).expect("read file");
        assert_eq!(content, "Hello \\LaTeX!");

        rename_path(
            file_a.to_string_lossy().into_owned(),
            file_b.to_string_lossy().into_owned(),
        )
        .expect("rename file");
        assert!(!file_a.exists());
        assert!(file_b.exists());

        delete_path(file_b.to_string_lossy().into_owned()).expect("delete file");
        assert!(!file_b.exists());
        delete_path(folder.to_string_lossy().into_owned()).expect("delete dir");
        assert!(!folder.exists());

        // Export PDF copy & missing source error handling
        let src_pdf = ws.dir.join(".bex-build/out.pdf");
        let dst_pdf = ws.dir.join("exports/paper.pdf");
        fs::create_dir_all(src_pdf.parent().unwrap()).unwrap();
        fs::write(&src_pdf, b"%PDF-1.4 sample").unwrap();
        export_pdf(
            src_pdf.to_string_lossy().into_owned(),
            dst_pdf.to_string_lossy().into_owned(),
        )
        .expect("export pdf");
        assert_eq!(fs::read(&dst_pdf).unwrap(), b"%PDF-1.4 sample");
        assert!(export_pdf(
            ws.dir.join("missing.pdf").to_string_lossy().into_owned(),
            dst_pdf.to_string_lossy().into_owned()
        )
        .is_err());

        // Guard against deleting empty path or filesystem root
        assert!(delete_path(String::new()).is_err());
        #[cfg(not(windows))]
        assert!(delete_path("/".to_string()).is_err());
        #[cfg(windows)]
        assert!(delete_path(r"C:\".to_string()).is_err());
    }

    #[test]
    fn seed_sample_project_preserves_existing_main_tex() {
        let ws = TempWorkspace::new("seed");
        seed_sample_project(&ws.dir).expect("initial seed");
        assert!(ws.dir.join("main.tex").is_file());
        assert!(ws.dir.join("sections/intro.tex").is_file());
        assert!(ws.dir.join("sections/methodology.tex").is_file());
        assert!(ws.dir.join("references.bib").is_file());

        // Modify main.tex and ensure re-seeding never overwrites user edits
        let custom = "% user custom edit\n\\documentclass{article}";
        fs::write(ws.dir.join("main.tex"), custom).unwrap();
        seed_sample_project(&ws.dir).expect("re-seed");
        let after = fs::read_to_string(ws.dir.join("main.tex")).unwrap();
        assert_eq!(after, custom);
    }

    #[test]
    fn create_new_project_seeded_and_empty() {
        let ws1 = TempWorkspace::new("create-seeded");
        let proj1_dir = ws1.dir.join("seeded_proj");
        let res1 = create_new_project(proj1_dir.to_string_lossy().into_owned(), true)
            .expect("create seeded project");
        assert_eq!(res1.default_entry_file.as_deref(), Some("main.tex"));
        assert!(proj1_dir.join("main.tex").is_file());
        assert!(proj1_dir.join("sections/intro.tex").is_file());

        let ws2 = TempWorkspace::new("create-blank");
        let proj2_dir = ws2.dir.join("blank_proj");
        let res2 = create_new_project(proj2_dir.to_string_lossy().into_owned(), false)
            .expect("create blank project");
        assert_eq!(res2.default_entry_file.as_deref(), Some("main.tex"));
        assert!(proj2_dir.join("main.tex").is_file());
        assert!(!proj2_dir.join("sections/intro.tex").exists());
        let content = fs::read_to_string(proj2_dir.join("main.tex")).unwrap();
        assert!(content.contains("\\documentclass{article}"));
    }
}

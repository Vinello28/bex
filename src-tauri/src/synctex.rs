use crate::workspace::{canonicalize_clean, path_to_clean_string};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

const DEFAULT_SCALE_PT_PER_UNIT: f64 = 1.0 / 65536.0 * (72.0 / 72.27);
const MIN_RECT_WIDTH: f64 = 12.0;
const MIN_RECT_HEIGHT: f64 = 10.0;
const LINE_SEARCH_RADIUS: i64 = 8;
const LINE_MERGE_Y_TOLERANCE: f64 = 3.0;
const LINE_MERGE_X_GAP: f64 = 16.0;
const INVERSE_BOX_PADDING: f64 = 4.0;
const INVERSE_NEAREST_RADIUS: f64 = 36.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTexForwardRequest {
    pub root_dir: String,
    pub entry_file: String,
    pub file_path: String,
    pub line: u32,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncTexRect {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTexInverseRequest {
    pub root_dir: String,
    pub entry_file: String,
    pub page: u32,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncTexLocation {
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordKind {
    HBox,
    VBox,
    Node,
}

#[derive(Debug, Clone)]
struct SyncTexRecord {
    kind: RecordKind,
    page: u32,
    tag: i32,
    line: u32,
    column: Option<u32>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone)]
struct BoxFrame {
    kind: RecordKind,
    y: f64,
    height: f64,
}

#[derive(Debug, Clone)]
struct SyncTexData {
    inputs: HashMap<i32, String>,
    records: Vec<SyncTexRecord>,
}

#[derive(Clone)]
struct CachedSyncTex {
    path: PathBuf,
    modified: Option<SystemTime>,
    len: u64,
    data: Arc<SyncTexData>,
}

static SYNCTEX_CACHE: OnceLock<Mutex<Option<CachedSyncTex>>> = OnceLock::new();

/// Clears the cached parsed SyncTeX structure.
pub fn invalidate_synctex_cache() {
    let cache = SYNCTEX_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = cache.lock() {
        *guard = None;
    }
}

fn compute_scale(unit: i64, magnification: i64) -> f64 {
    let eff_unit = if unit > 0 { unit as f64 } else { 1.0 };
    let eff_mag = if magnification > 0 {
        magnification as f64
    } else {
        1000.0
    };
    eff_unit * eff_mag / 1000.0 * DEFAULT_SCALE_PT_PER_UNIT
}

fn parse_tag_line_col(spec: &str) -> Option<(i32, u32, Option<u32>)> {
    let mut parts = spec.split(',');
    let tag = parts.next()?.trim().parse::<i32>().ok()?;
    let line = parts.next()?.trim().parse::<u32>().ok()?;
    let column = parts
        .next()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&c| c > 0);
    Some((tag, line, column))
}

fn parse_hv(spec: &str, last_v: &mut i64) -> Option<(i64, i64)> {
    let (h_str, v_str) = spec.split_once(',')?;
    let h = h_str.trim().parse::<i64>().ok()?;
    let v_trim = v_str.trim();
    let v = if v_trim == "=" {
        *last_v
    } else {
        let parsed = v_trim.parse::<i64>().ok()?;
        *last_v = parsed;
        parsed
    };
    Some((h, v))
}

fn parse_whd(spec: &str) -> Option<(i64, i64, i64)> {
    let mut parts = spec.split(',');
    let w = parts.next()?.trim().parse::<i64>().ok()?;
    let h = parts
        .next()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    let d = parts
        .next()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    Some((w, h, d))
}

fn pop_box_frame(stack: &mut Vec<BoxFrame>, target: RecordKind) {
    if let Some(pos) = stack.iter().rposition(|f| f.kind == target) {
        stack.truncate(pos);
    }
}

fn enclosing_hbox_bounds(stack: &[BoxFrame], baseline_y: f64) -> Option<(f64, f64)> {
    for frame in stack.iter().rev() {
        if frame.kind == RecordKind::HBox
            && frame.height >= 4.0
            && frame.height <= 32.0
            && baseline_y >= frame.y - 1.0
            && baseline_y <= frame.y + frame.height + 2.0
        {
            return Some((frame.y, frame.height));
        }
    }
    None
}

fn parse_synctex_text(text: &str) -> SyncTexData {
    let mut inputs: HashMap<i32, String> = HashMap::new();
    let mut records: Vec<SyncTexRecord> = Vec::new();

    let mut magnification: i64 = 1000;
    let mut unit: i64 = 1;
    let mut x_off: i64 = 0;
    let mut y_off: i64 = 0;
    let mut scale = compute_scale(unit, magnification);

    let mut current_page: u32 = 0;
    let mut last_v: i64 = 0;
    let mut box_stack: Vec<BoxFrame> = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }

        if line.starts_with("Postamble:") {
            break;
        }

        if let Some(rest) = line.strip_prefix("Input:") {
            if let Some((tag_str, path_str)) = rest.split_once(':') {
                if let Ok(tag) = tag_str.trim().parse::<i32>() {
                    inputs.insert(tag, path_str.to_string());
                }
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("Magnification:") {
            if let Ok(val) = rest.trim().parse::<i64>() {
                magnification = val;
                scale = compute_scale(unit, magnification);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("Unit:") {
            if let Ok(val) = rest.trim().parse::<i64>() {
                unit = val;
                scale = compute_scale(unit, magnification);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("X Offset:") {
            if let Ok(val) = rest.trim().parse::<i64>() {
                x_off = val;
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("Y Offset:") {
            if let Ok(val) = rest.trim().parse::<i64>() {
                y_off = val;
            }
            continue;
        }

        if line.starts_with("SyncTeX Version:")
            || line.starts_with("Output:")
            || line.starts_with("Content:")
        {
            continue;
        }

        let Some(first_byte) = line.as_bytes().first().copied() else {
            continue;
        };

        match first_byte {
            b'{' => {
                if let Ok(page) = line[1..].trim().parse::<u32>() {
                    current_page = page;
                    last_v = 0;
                    box_stack.clear();
                }
            }
            b'}' => {
                current_page = 0;
                box_stack.clear();
            }
            b')' => pop_box_frame(&mut box_stack, RecordKind::HBox),
            b']' => pop_box_frame(&mut box_stack, RecordKind::VBox),
            b'!' | b'<' | b'>' => {}
            b'f' if current_page > 0 => {
                // `f<obj>:<h>,<v>` updates last_v if explicit, produces no record.
                if let Some((_, hv_str)) = line[1..].split_once(':') {
                    let _ = parse_hv(hv_str, &mut last_v);
                }
            }
            b'r' if current_page > 0 => {
                // `r<tag>,<line>:<h>,<v>:<W>,<H>,<D>` updates last_v if explicit, produces no record.
                let mut parts = line[1..].splitn(3, ':');
                if let (Some(_), Some(hv_str)) = (parts.next(), parts.next()) {
                    let _ = parse_hv(hv_str, &mut last_v);
                }
            }
            b'(' | b'h' | b'[' | b'v' if current_page > 0 => {
                let kind = if first_byte == b'(' || first_byte == b'h' {
                    RecordKind::HBox
                } else {
                    RecordKind::VBox
                };
                let pushes_stack = first_byte == b'(' || first_byte == b'[';

                let mut parts = line[1..].splitn(3, ':');
                let (Some(src_spec), Some(hv_spec), Some(whd_spec)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };

                let Some((tag, src_line, column)) = parse_tag_line_col(src_spec) else {
                    continue;
                };
                let Some((h_raw, v_raw)) = parse_hv(hv_spec, &mut last_v) else {
                    continue;
                };
                let Some((w_raw, h_box_raw, d_box_raw)) = parse_whd(whd_spec) else {
                    continue;
                };

                let x = (h_raw + x_off) as f64 * scale;
                let y = (v_raw - h_box_raw + y_off) as f64 * scale;
                let width = (w_raw as f64 * scale).max(0.0);
                let height = ((h_box_raw + d_box_raw) as f64 * scale).max(0.0);

                if pushes_stack {
                    box_stack.push(BoxFrame { kind, y, height });
                }

                if tag > 0 && src_line > 0 {
                    records.push(SyncTexRecord {
                        kind,
                        page: current_page,
                        tag,
                        line: src_line,
                        column,
                        x,
                        y,
                        width,
                        height,
                    });
                }
            }
            b'x' | b'k' | b'g' | b'$' if current_page > 0 => {
                let mut parts = line[1..].splitn(3, ':');
                let (Some(src_spec), Some(hv_spec)) = (parts.next(), parts.next()) else {
                    continue;
                };
                let opt_w_spec = parts.next();

                let Some((tag, src_line, column)) = parse_tag_line_col(src_spec) else {
                    continue;
                };
                let Some((h_raw, v_raw)) = parse_hv(hv_spec, &mut last_v) else {
                    continue;
                };

                let x = (h_raw + x_off) as f64 * scale;
                let v_pt = (v_raw + y_off) as f64 * scale;
                let width = opt_w_spec
                    .and_then(|s| s.trim().split(',').next()?.trim().parse::<i64>().ok())
                    .map(|w| w as f64 * scale)
                    .filter(|&w| w > 0.5)
                    .unwrap_or(12.0);
                let (y, height) = enclosing_hbox_bounds(&box_stack, v_pt)
                    .unwrap_or_else(|| ((v_pt - 10.0).max(0.0), 12.0));

                if tag > 0 && src_line > 0 {
                    records.push(SyncTexRecord {
                        kind: RecordKind::Node,
                        page: current_page,
                        tag,
                        line: src_line,
                        column,
                        x,
                        y,
                        width,
                        height,
                    });
                }
            }
            _ => {}
        }
    }

    SyncTexData { inputs, records }
}

fn read_synctex_file(path: &Path) -> Result<SyncTexData, String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open SyncTeX file '{}': {}", path.display(), e))?;
    let mut raw_bytes = Vec::new();
    file.read_to_end(&mut raw_bytes)
        .map_err(|e| format!("Failed to read SyncTeX file '{}': {}", path.display(), e))?;

    let is_gzip = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(".gz")
        || raw_bytes.starts_with(&[0x1f, 0x8b]);

    let text = if is_gzip {
        let mut decoder = GzDecoder::new(&raw_bytes[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).map_err(|e| {
            format!(
                "Failed to decompress SyncTeX file '{}': {}",
                path.display(),
                e
            )
        })?;
        String::from_utf8_lossy(&decompressed).into_owned()
    } else {
        String::from_utf8_lossy(&raw_bytes).into_owned()
    };

    Ok(parse_synctex_text(&text))
}

fn load_synctex_cached(path: &Path) -> Result<Arc<SyncTexData>, String> {
    let canon_path = canonicalize_clean(path)?;
    let meta = std::fs::metadata(&canon_path).map_err(|e| {
        format!(
            "Failed to inspect SyncTeX metadata '{}': {}",
            canon_path.display(),
            e
        )
    })?;
    let modified = meta.modified().ok();
    let len = meta.len();

    let cache = SYNCTEX_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.path == canon_path && entry.modified == modified && entry.len == len {
                return Ok(Arc::clone(&entry.data));
            }
        }
    }

    let parsed = Arc::new(read_synctex_file(&canon_path)?);
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(CachedSyncTex {
            path: canon_path,
            modified,
            len,
            data: Arc::clone(&parsed),
        });
    }
    Ok(parsed)
}

fn resolve_entry_rel_and_stem(
    root_canon: &Path,
    entry_file: &str,
) -> Result<(String, String), String> {
    let trimmed = entry_file.trim();
    if trimmed.is_empty() {
        return Err("Entry file path cannot be empty.".to_string());
    }

    let cleaned_input = trimmed.replace('\\', "/");
    let raw_entry = Path::new(&cleaned_input);
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

    let resolved = resolve_path_against_base(trimmed, root_canon);
    let rel_path = resolved.rel_to_root.ok_or_else(|| {
        format!(
            "Entry file '{}' is outside workspace root '{}'.",
            entry_file,
            root_canon.display()
        )
    })?;

    if rel_path.is_empty() {
        return Err("Invalid entry file path.".to_string());
    }

    let stem = Path::new(&rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Invalid entry file name '{}'.", entry_file))?
        .to_string();

    Ok((rel_path, stem))
}

fn locate_synctex_file(root_canon: &Path, entry_rel: &str, entry_stem: &str) -> Option<PathBuf> {
    let build_dir = root_canon.join(".bex-build");
    let mut candidates = vec![
        build_dir.join(format!("{entry_stem}.synctex.gz")),
        build_dir.join(format!("{entry_stem}.synctex")),
    ];

    if let Some(rel_parent) = Path::new(entry_rel).parent() {
        if !rel_parent.as_os_str().is_empty() {
            let nested_dir = build_dir.join(rel_parent);
            candidates.push(nested_dir.join(format!("{entry_stem}.synctex.gz")));
            candidates.push(nested_dir.join(format!("{entry_stem}.synctex")));
        }
    }

    candidates.into_iter().find(|p| p.is_file())
}

fn is_windows_drive_abs(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn lexical_clean_unix(raw: &str) -> String {
    let mut s = raw.trim().replace('\\', "/");
    if let Some(rest) = s.strip_prefix("//?/UNC/") {
        s = format!("//{rest}");
    } else if let Some(rest) = s.strip_prefix("//?/") {
        s = rest.to_string();
    }

    let (prefix, rest) = if s.len() >= 2 && s.as_bytes()[0].is_ascii_alphabetic() && s.as_bytes()[1] == b':' {
        let drive_prefix = format!("{}:", s.as_bytes()[0] as char);
        let tail = s[2..].trim_start_matches('/');
        (Some(format!("{drive_prefix}/")), tail)
    } else if s.starts_with('/') {
        (Some("/".to_string()), s.trim_start_matches('/'))
    } else {
        (None, s.as_str())
    };

    let mut stack: Vec<&str> = Vec::new();
    for seg in rest.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if let Some(last) = stack.last() {
                    if *last != ".." {
                        stack.pop();
                    } else if prefix.is_none() {
                        stack.push("..");
                    }
                } else if prefix.is_none() {
                    stack.push("..");
                }
            }
            other => stack.push(other),
        }
    }

    match prefix {
        Some(p) => format!("{p}{}", stack.join("/")),
        None => stack.join("/"),
    }
}

fn normalize_macos_private_alias(unix_abs: &str) -> String {
    if let Some(rest) = unix_abs.strip_prefix("/private") {
        if rest.starts_with("/var/") || rest == "/var" || rest.starts_with("/tmp/") || rest == "/tmp" {
            return rest.to_string();
        }
    }
    unix_abs.to_string()
}

fn same_abs_path(a: &str, b: &str) -> bool {
    let na = normalize_macos_private_alias(a);
    let nb = normalize_macos_private_alias(b);
    #[cfg(windows)]
    {
        na.eq_ignore_ascii_case(&nb)
    }
    #[cfg(not(windows))]
    {
        na == nb
    }
}

fn strip_root_prefix_from_abs(abs_unix: &str, root_unix: &str) -> Option<String> {
    let abs_norm = normalize_macos_private_alias(abs_unix);
    let root_norm = normalize_macos_private_alias(root_unix);
    let root_trimmed = root_norm.trim_end_matches('/');

    #[cfg(windows)]
    let matches_prefix = abs_norm
        .to_ascii_lowercase()
        .starts_with(&root_trimmed.to_ascii_lowercase());
    #[cfg(not(windows))]
    let matches_prefix = abs_norm.starts_with(root_trimmed);

    if !matches_prefix {
        return None;
    }

    let suffix = &abs_norm[root_trimmed.len()..];
    if suffix.is_empty() {
        return Some(String::new());
    }
    if let Some(rel) = suffix.strip_prefix('/') {
        return Some(rel.to_string());
    }
    None
}

#[derive(Debug, Clone)]
struct ResolvedPath {
    abs_unix: String,
    rel_to_root: Option<String>,
    is_project_file: bool,
    is_build_artifact: bool,
}

fn resolve_path_against_base(raw_path: &str, root_canon: &Path) -> ResolvedPath {
    resolve_path_with_join_base(raw_path, root_canon, root_canon)
}

fn resolve_path_with_join_base(
    raw_path: &str,
    root_canon: &Path,
    rel_join_base: &Path,
) -> ResolvedPath {
    let trimmed = raw_path.trim();
    let root_unix = lexical_clean_unix(&path_to_clean_string(root_canon));
    let cleaned = lexical_clean_unix(trimmed);

    let is_abs = cleaned.starts_with('/') || is_windows_drive_abs(&cleaned);
    let candidate = if is_abs {
        PathBuf::from(&cleaned)
    } else {
        rel_join_base.join(&cleaned)
    };

    let abs_unix = if let Ok(canon) = canonicalize_clean(&candidate) {
        lexical_clean_unix(&path_to_clean_string(&canon))
    } else if is_abs {
        cleaned
    } else {
        let base_unix = lexical_clean_unix(&path_to_clean_string(rel_join_base));
        lexical_clean_unix(&format!("{}/{}", base_unix.trim_end_matches('/'), cleaned))
    };

    let rel_to_root = strip_root_prefix_from_abs(&abs_unix, &root_unix);
    let is_project_file = rel_to_root.is_some();
    let is_build_artifact = rel_to_root
        .as_deref()
        .map(|rel| rel == ".bex-build" || rel.starts_with(".bex-build/"))
        .unwrap_or(false);

    ResolvedPath {
        abs_unix,
        rel_to_root,
        is_project_file,
        is_build_artifact,
    }
}

fn resolve_root_canon(raw_root: &str) -> Result<PathBuf, String> {
    let trimmed = raw_root.trim();
    if trimmed.is_empty() {
        return Err("Workspace root directory cannot be empty.".to_string());
    }
    let root_canon = canonicalize_clean(Path::new(trimmed))?;
    if !root_canon.is_dir() {
        return Err(format!(
            "Workspace root '{}' is not a directory.",
            root_canon.display()
        ));
    }
    Ok(root_canon)
}

fn paths_match(a: &ResolvedPath, b: &ResolvedPath) -> bool {
    if let (Some(ra), Some(rb)) = (&a.rel_to_root, &b.rel_to_root) {
        #[cfg(windows)]
        {
            if ra.eq_ignore_ascii_case(rb) {
                return true;
            }
        }
        #[cfg(not(windows))]
        {
            if ra == rb {
                return true;
            }
        }
    }
    same_abs_path(&a.abs_unix, &b.abs_unix)
}

fn match_tags_for_file(
    inputs: &HashMap<i32, String>,
    root_canon: &Path,
    entry_rel: &str,
    target_file_path: &str,
) -> HashSet<i32> {
    let target = resolve_path_against_base(target_file_path, root_canon);
    let mut tags = HashSet::new();

    for (&tag, raw_input) in inputs {
        let resolved = resolve_path_against_base(raw_input, root_canon);
        if paths_match(&resolved, &target) {
            tags.insert(tag);
        }
    }

    if tags.is_empty() {
        if let Some(rel_parent) = Path::new(entry_rel).parent() {
            if !rel_parent.as_os_str().is_empty() {
                let entry_dir = root_canon.join(rel_parent);
                for (&tag, raw_input) in inputs {
                    let resolved = resolve_path_with_join_base(raw_input, root_canon, &entry_dir);
                    if paths_match(&resolved, &target) {
                        tags.insert(tag);
                    }
                }
            }
        }
    }

    tags
}

fn normalize_forward_rect(rec: &SyncTexRecord) -> SyncTexRect {
    let x = rec.x.max(0.0);
    let width = rec.width.max(MIN_RECT_WIDTH);
    let (y, height) = if rec.height < MIN_RECT_HEIGHT {
        let shifted = rec.y - (MIN_RECT_HEIGHT - rec.height) * 0.5;
        (shifted.max(0.0), MIN_RECT_HEIGHT)
    } else {
        (rec.y.max(0.0), rec.height)
    };
    SyncTexRect {
        page: rec.page,
        x,
        y,
        width,
        height,
    }
}

fn merge_line_rects(mut rects: Vec<SyncTexRect>) -> Vec<SyncTexRect> {
    if rects.is_empty() {
        return rects;
    }

    rects.sort_by(|a, b| {
        a.y.total_cmp(&b.y)
            .then_with(|| a.x.total_cmp(&b.x))
    });

    // Group rects whose vertical start is within LINE_MERGE_Y_TOLERANCE, then merge horizontally.
    let mut line_clusters: Vec<(f64, Vec<SyncTexRect>)> = Vec::new();
    for rect in rects {
        if let Some((_, group)) = line_clusters
            .iter_mut()
            .find(|(ref_y, _)| (rect.y - *ref_y).abs() <= LINE_MERGE_Y_TOLERANCE)
        {
            group.push(rect);
        } else {
            line_clusters.push((rect.y, vec![rect]));
        }
    }

    let mut merged = Vec::new();
    for (_, mut group) in line_clusters {
        group.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)));
        let mut iter = group.into_iter();
        let Some(mut current) = iter.next() else {
            continue;
        };
        for next in iter {
            let current_right = current.x + current.width;
            if next.x <= current_right + LINE_MERGE_X_GAP {
                let new_x = current.x.min(next.x);
                let new_y = current.y.min(next.y);
                let new_right = current_right.max(next.x + next.width);
                let new_bottom = (current.y + current.height).max(next.y + next.height);
                current.x = new_x;
                current.y = new_y;
                current.width = new_right - new_x;
                current.height = new_bottom - new_y;
            } else {
                merged.push(current);
                current = next;
            }
        }
        merged.push(current);
    }

    merged
}

fn compute_forward(
    data: &SyncTexData,
    root_canon: &Path,
    entry_rel: &str,
    file_path: &str,
    target_line: u32,
) -> Vec<SyncTexRect> {
    if target_line == 0 || file_path.trim().is_empty() {
        return Vec::new();
    }

    let tags = match_tags_for_file(&data.inputs, root_canon, entry_rel, file_path);
    if tags.is_empty() {
        return Vec::new();
    }

    let file_records: Vec<&SyncTexRecord> = data
        .records
        .iter()
        .filter(|r| tags.contains(&r.tag) && r.line > 0)
        .collect();
    if file_records.is_empty() {
        return Vec::new();
    }

    let exact: Vec<&SyncTexRecord> = file_records
        .iter()
        .copied()
        .filter(|r| r.line == target_line)
        .collect();

    let line_records = if !exact.is_empty() {
        exact
    } else {
        let target_i64 = target_line as i64;
        let mut best_line: Option<u32> = None;
        let mut best_key = (i64::MAX, 1u8);

        for rec in &file_records {
            let delta = rec.line as i64 - target_i64;
            let dist = delta.abs();
            if dist <= LINE_SEARCH_RADIUS {
                let dir_rank = if delta >= 0 { 0u8 } else { 1u8 };
                let key = (dist, dir_rank);
                if key < best_key {
                    best_key = key;
                    best_line = Some(rec.line);
                }
            }
        }

        let Some(matched_line) = best_line else {
            return Vec::new();
        };

        file_records
            .into_iter()
            .filter(|r| r.line == matched_line)
            .collect()
    };

    let has_hbox = line_records.iter().any(|r| r.kind == RecordKind::HBox);
    let has_node = line_records.iter().any(|r| r.kind == RecordKind::Node);

    let preferred_kind = if has_hbox {
        RecordKind::HBox
    } else if has_node {
        RecordKind::Node
    } else {
        RecordKind::VBox
    };

    let kind_records: Vec<&SyncTexRecord> = line_records
        .into_iter()
        .filter(|r| r.kind == preferred_kind)
        .collect();
    if kind_records.is_empty() {
        return Vec::new();
    }

    let mut page_counts: HashMap<u32, usize> = HashMap::new();
    for rec in &kind_records {
        *page_counts.entry(rec.page).or_insert(0) += 1;
    }

    let primary_page = page_counts
        .into_iter()
        .max_by(|(p1, c1), (p2, c2)| c1.cmp(c2).then_with(|| p2.cmp(p1)))
        .map(|(p, _)| p)
        .unwrap_or(kind_records[0].page);

    let page_rects: Vec<SyncTexRect> = kind_records
        .into_iter()
        .filter(|r| r.page == primary_page)
        .map(normalize_forward_rect)
        .collect();

    merge_line_rects(page_rects)
}

fn rect_distance(rec: &SyncTexRecord, qx: f64, qy: f64) -> f64 {
    let dx = if qx < rec.x {
        rec.x - qx
    } else if qx > rec.x + rec.width {
        qx - (rec.x + rec.width)
    } else {
        0.0
    };
    let dy = if qy < rec.y {
        rec.y - qy
    } else if qy > rec.y + rec.height {
        qy - (rec.y + rec.height)
    } else {
        0.0
    };
    dx.hypot(dy)
}

fn contains_padded(rec: &SyncTexRecord, qx: f64, qy: f64, pad: f64) -> bool {
    qx >= rec.x - pad
        && qx <= rec.x + rec.width + pad
        && qy >= rec.y - pad
        && qy <= rec.y + rec.height + pad
}

fn select_inverse_from_pool<'a>(
    pool: &[(&'a SyncTexRecord, &'a ResolvedPath)],
    qx: f64,
    qy: f64,
) -> Option<SyncTexLocation> {
    if pool.is_empty() {
        return None;
    }

    // Priority (1): smallest-area non-degenerate HBox containing (x, y) with ~4pt padding.
    let best_hbox = pool
        .iter()
        .filter(|(rec, _)| {
            rec.kind == RecordKind::HBox
                && rec.width > 0.5
                && rec.height > 0.5
                && contains_padded(rec, qx, qy, INVERSE_BOX_PADDING)
        })
        .min_by(|(a, _), (b, _)| {
            let area_a = a.width * a.height;
            let area_b = b.width * b.height;
            area_a
                .total_cmp(&area_b)
                .then_with(|| rect_distance(a, qx, qy).total_cmp(&rect_distance(b, qx, qy)))
        });

    if let Some((rec, path)) = best_hbox {
        return Some(SyncTexLocation {
            file: path
                .rel_to_root
                .clone()
                .unwrap_or_else(|| path.abs_unix.clone()),
            line: rec.line,
            column: rec.column,
        });
    }

    // Priority (2): closest Node or HBox within ~36pt.
    let best_near = pool
        .iter()
        .filter(|(rec, _)| rec.kind == RecordKind::Node || rec.kind == RecordKind::HBox)
        .filter_map(|(rec, path)| {
            let dist = rect_distance(rec, qx, qy);
            if dist <= INVERSE_NEAREST_RADIUS {
                Some((dist, rec, path))
            } else {
                None
            }
        })
        .min_by(|(da, ra, _), (db, rb, _)| {
            let kind_rank = |k: RecordKind| if k == RecordKind::Node { 0u8 } else { 1u8 };
            da.total_cmp(db)
                .then_with(|| kind_rank(ra.kind).cmp(&kind_rank(rb.kind)))
                .then_with(|| (ra.width * ra.height).total_cmp(&(rb.width * rb.height)))
        });

    if let Some((_, rec, path)) = best_near {
        return Some(SyncTexLocation {
            file: path
                .rel_to_root
                .clone()
                .unwrap_or_else(|| path.abs_unix.clone()),
            line: rec.line,
            column: rec.column,
        });
    }

    // Priority (3): smallest enclosing VBox or nearest record on page.
    let best_vbox = pool
        .iter()
        .filter(|(rec, _)| {
            rec.kind == RecordKind::VBox
                && rec.width > 0.5
                && rec.height > 0.5
                && contains_padded(rec, qx, qy, INVERSE_BOX_PADDING)
        })
        .min_by(|(a, _), (b, _)| (a.width * a.height).total_cmp(&(b.width * b.height)));

    if let Some((rec, path)) = best_vbox {
        return Some(SyncTexLocation {
            file: path
                .rel_to_root
                .clone()
                .unwrap_or_else(|| path.abs_unix.clone()),
            line: rec.line,
            column: rec.column,
        });
    }

    let nearest = pool
        .iter()
        .min_by(|(a, _), (b, _)| rect_distance(a, qx, qy).total_cmp(&rect_distance(b, qx, qy)));

    nearest.map(|(rec, path)| SyncTexLocation {
        file: path
            .rel_to_root
            .clone()
            .unwrap_or_else(|| path.abs_unix.clone()),
        line: rec.line,
        column: rec.column,
    })
}

fn compute_inverse(
    data: &SyncTexData,
    root_canon: &Path,
    entry_rel: &str,
    page: u32,
    x: f64,
    y: f64,
) -> Option<SyncTexLocation> {
    if page == 0 {
        return None;
    }

    let entry_dir = Path::new(entry_rel)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| root_canon.join(p));

    let resolved_inputs: HashMap<i32, ResolvedPath> = data
        .inputs
        .iter()
        .map(|(&tag, raw)| {
            let mut r = resolve_path_against_base(raw, root_canon);
            if !r.is_project_file {
                if let Some(ref ed) = entry_dir {
                    let alt = resolve_path_with_join_base(raw, root_canon, ed);
                    if alt.is_project_file {
                        r = alt;
                    }
                }
            }
            (tag, r)
        })
        .collect();

    let mut project_pool: Vec<(&SyncTexRecord, &ResolvedPath)> = Vec::new();
    let mut fallback_pool: Vec<(&SyncTexRecord, &ResolvedPath)> = Vec::new();

    for rec in &data.records {
        if rec.page != page || rec.line == 0 {
            continue;
        }
        let Some(resolved) = resolved_inputs.get(&rec.tag) else {
            continue;
        };
        if resolved.is_project_file && !resolved.is_build_artifact {
            project_pool.push((rec, resolved));
        } else {
            fallback_pool.push((rec, resolved));
        }
    }

    if !project_pool.is_empty() {
        select_inverse_from_pool(&project_pool, x, y)
    } else {
        select_inverse_from_pool(&fallback_pool, x, y)
    }
}

#[tauri::command]
pub async fn synctex_forward(request: SyncTexForwardRequest) -> Result<Vec<SyncTexRect>, String> {
    tokio::task::spawn_blocking(move || {
        let root_canon = resolve_root_canon(&request.root_dir)?;
        let (entry_rel, entry_stem) = resolve_entry_rel_and_stem(&root_canon, &request.entry_file)?;
        let Some(synctex_path) = locate_synctex_file(&root_canon, &entry_rel, &entry_stem) else {
            return Ok(Vec::new());
        };
        let data = load_synctex_cached(&synctex_path)?;
        Ok(compute_forward(
            &data,
            &root_canon,
            &entry_rel,
            &request.file_path,
            request.line,
        ))
    })
    .await
    .map_err(|e| format!("SyncTeX forward task failed: {e}"))?
}

#[tauri::command]
pub async fn synctex_inverse(
    request: SyncTexInverseRequest,
) -> Result<Option<SyncTexLocation>, String> {
    tokio::task::spawn_blocking(move || {
        let root_canon = resolve_root_canon(&request.root_dir)?;
        let (entry_rel, entry_stem) = resolve_entry_rel_and_stem(&root_canon, &request.entry_file)?;
        let Some(synctex_path) = locate_synctex_file(&root_canon, &entry_rel, &entry_stem) else {
            return Ok(None);
        };
        let data = load_synctex_cached(&synctex_path)?;
        Ok(compute_inverse(
            &data,
            &root_canon,
            &entry_rel,
            request.page,
            request.x,
            request.y,
        ))
    })
    .await
    .map_err(|e| format!("SyncTeX inverse task failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{compile_latex, CompileRequest};
    use crate::workspace::seed_sample_project;
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(label: &str) -> Self {
            let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bex-synctex-test-{}-{}-{}",
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

    fn write_gz(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let file = File::create(path).unwrap();
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(content.as_bytes()).unwrap();
        encoder.finish().unwrap();
    }

    #[test]
    fn parses_gz_with_late_input_shorthand_v_and_multi_file_queries() {
        let dir = TempDirGuard::new("synthetic-gz");
        std::fs::create_dir_all(dir.path.join("sections")).unwrap();
        std::fs::write(dir.path.join("main.tex"), "% main").unwrap();
        std::fs::write(dir.path.join("sections/intro.tex"), "% intro").unwrap();

        let root_canon = canonicalize_clean(&dir.path).unwrap();
        let main_path_str = path_to_clean_string(&root_canon.join("main.tex"));
        let intro_path_str = path_to_clean_string(&root_canon.join("sections/intro.tex"));

        // 1pt in TeX scaled points = 65536, 72.27 pt/in -> ~1bp
        // With Unit:1 and Magnification:1000:
        // 72.27 * 65536 = 4736287 -> 72.0 bp
        // 100 * 65536 = 6553600 -> ~99.626 bp
        let synctex_content = format!(
            "SyncTeX Version:1\n\
             Input:1:{main_path_str}\n\
             Input:99:/texmf-dist/tex/latex/base/article.cls\n\
             Output:pdf\n\
             Magnification:1000\n\
             Unit:1\n\
             X Offset:0\n\
             Y Offset:0\n\
             Content:\n\
             !10\n\
             {{1\n\
             [1,10:4736287,6553600:26214400,655360,131072\n\
             (1,12:4736287,6553600:13107200,655360,131072\n\
             x1,12:4736287,=\n\
             k1,12:6553600,=:655360\n\
             )\n\
             Input:2:{intro_path_str}\n\
             (2,5,3:4736287,13107200:9830400,655360,131072\n\
             x2,5,3:6046874,=\n\
             )\n\
             (99,42:4736287,13107200:3276800,655360,131072\n\
             )\n\
             ]\n\
             !5\n\
             }}1\n\
             Postamble:\n\
             Count:7\n"
        );

        let synctex_gz = root_canon.join(".bex-build/main.synctex.gz");
        write_gz(&synctex_gz, &synctex_content);

        let data = load_synctex_cached(&synctex_gz).expect("load synthetic synctex.gz");
        assert_eq!(data.inputs.get(&1).map(String::as_str), Some(main_path_str.as_str()));
        assert_eq!(data.inputs.get(&2).map(String::as_str), Some(intro_path_str.as_str()));

        // Verify `=` shorthand preserved last_v = 6553600 and 13107200
        let intro_node = data
            .records
            .iter()
            .find(|r| r.tag == 2 && r.kind == RecordKind::Node)
            .expect("intro node");
        assert_eq!(intro_node.line, 5);
        assert_eq!(intro_node.column, Some(3));
        assert!((intro_node.y - ((13107200 - 655360) as f64 / 65536.0 * (72.0 / 72.27))).abs() < 0.01);

        // Forward query on late-declared Input:2 via relative Windows-style path
        let rects = compute_forward(&data, &root_canon, "main.tex", r".\sections\intro.tex", 5);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].page, 1);
        assert!((rects[0].x - 72.0).abs() < 0.2);

        // Nearest fallback within +-8 lines (line 11 is 1 away from 12, and 1 from 10 -> prefers downward line 12!)
        let near_rects = compute_forward(&data, &root_canon, "main.tex", "main.tex", 11);
        assert_eq!(near_rects.len(), 1);
        assert!((near_rects[0].width - 200.0 * (72.0 / 72.27)).abs() < 1.0);

        // Inverse query near intro line (which overlaps bundle tag 99 at same coordinates!)
        // Must prefer project file `sections/intro.tex` over `/texmf-dist/.../article.cls`!
        let intro_y = rects[0].y + rects[0].height * 0.5;
        let loc = compute_inverse(&data, &root_canon, "main.tex", 1, 90.0, intro_y)
            .expect("inverse hit");
        assert_eq!(loc.file, "sections/intro.tex");
        assert_eq!(loc.line, 5);
        assert_eq!(loc.column, Some(3));

        // Verify macOS /var vs /private/var lexical equivalence
        let target_var = ResolvedPath {
            abs_unix: "/var/folders/xx/proj/sections/intro.tex".to_string(),
            rel_to_root: None,
            is_project_file: false,
            is_build_artifact: false,
        };
        let input_priv = ResolvedPath {
            abs_unix: "/private/var/folders/xx/proj/sections/intro.tex".to_string(),
            rel_to_root: None,
            is_project_file: false,
            is_build_artifact: false,
        };
        assert!(paths_match(&target_var, &input_priv));
    }

    #[test]
    fn forward_merges_adjacent_nodes_and_enforces_min_size() {
        let dir = TempDirGuard::new("merge-rects");
        std::fs::write(dir.path.join("main.tex"), "% main").unwrap();
        let root_canon = canonicalize_clean(&dir.path).unwrap();
        let main_path = path_to_clean_string(&root_canon.join("main.tex"));

        // Two close `x` nodes on line 7 + one far column node on line 7
        let synctex_content = format!(
            "SyncTeX Version:1\n\
             Input:1:{main_path}\n\
             Magnification:1000\n\
             Unit:1\n\
             X Offset:0\n\
             Y Offset:0\n\
             Content:\n\
             {{1\n\
             x1,7:4736287,6553600\n\
             x1,7:5391647,=\n\
             x1,7:19660800,=\n\
             }}1\n\
             Postamble:\n"
        );
        let synctex_plain = root_canon.join(".bex-build/main.synctex");
        std::fs::create_dir_all(synctex_plain.parent().unwrap()).unwrap();
        std::fs::write(&synctex_plain, synctex_content).unwrap();

        let data = load_synctex_cached(&synctex_plain).expect("load plain .synctex");
        let rects = compute_forward(&data, &root_canon, "main.tex", "main.tex", 7);
        assert_eq!(rects.len(), 2, "expected first two nodes merged and far node separate");
        assert!(rects[0].width >= 12.0 && rects[0].height >= 10.0);
        assert!(rects[1].width >= 12.0 && rects[1].height >= 10.0);
        assert!(rects[0].width > 12.0);
    }

    #[tokio::test]
    async fn live_tectonic_synctex_forward_and_inverse_roundtrip() {
        let dir = TempDirGuard::new("live-roundtrip");
        seed_sample_project(&dir.path).expect("seed sample project");

        let resp = compile_latex(CompileRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "main.tex".to_string(),
            engine: "tectonic".to_string(),
        })
        .await
        .expect("compile_latex call");

        assert!(resp.success, "compile failed:\n{}", resp.log_output);
        assert!(resp.synctex_available, "expected synctex_available to be true");

        // Forward sync on sections/intro.tex (line 4 is paragraph in Introduction)
        let rects = synctex_forward(SyncTexForwardRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "main.tex".to_string(),
            file_path: "sections/intro.tex".to_string(),
            line: 4,
            column: None,
        })
        .await
        .expect("synctex_forward");

        assert!(
            !rects.is_empty(),
            "expected non-empty SyncTeX rects for sections/intro.tex line 4"
        );
        let first = &rects[0];
        assert_eq!(first.page, 1);
        assert!(first.width >= 12.0 && first.height >= 10.0);

        // Inverse sync back from the center of that rect
        let loc = synctex_inverse(SyncTexInverseRequest {
            root_dir: dir.path.to_string_lossy().into_owned(),
            entry_file: "main.tex".to_string(),
            page: first.page,
            x: first.x + first.width * 0.5,
            y: first.y + first.height * 0.5,
        })
        .await
        .expect("synctex_inverse")
        .expect("expected SyncTexLocation");

        assert_eq!(loc.file, "sections/intro.tex");
        assert!(
            (loc.line as i64 - 4).abs() <= 4,
            "expected inverse line close to 4, got {}",
            loc.line
        );
    }
}

use crate::compiler::{compile_latex, CompileRequest, CompileResponse, LatexDiagnostic};
use crate::workspace::{
    canonicalize_clean, path_to_clean_string, strip_verbatim_prefix, to_unix_rel_path,
    write_utf8_atomic, FileNode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_READ_BYTES: u64 = 512 * 1024; // 512 KB limit

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct McpContext {
    pub workspace_root: PathBuf,
    pub active_file: Option<String>,
    pub entry_file: Option<String>,
    pub selected_engine: String,
    pub diagnostics: Vec<LatexDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorMutation {
    pub kind: String, // "write" | "create" | "insert" | "replace"
    pub rel_path: String,
    pub abs_path: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpExecutionOutcome {
    pub text: String,
    pub is_error: bool,
    pub mutation: Option<EditorMutation>,
    pub compile_result: Option<CompileResponse>,
}

/// Resolves a workspace-relative or absolute path securely against `workspace_root`.
/// Strictly prevents path traversal (`..`), absolute path escapes, and symlink escapes.
pub fn resolve_safe_path(workspace_root: &Path, rel_or_abs: &str) -> Result<PathBuf, String> {
    let canon_root = canonicalize_clean(workspace_root)
        .map_err(|e| format!("Invalid workspace root '{}': {}", workspace_root.display(), e))?;

    let trimmed = rel_or_abs.trim();
    if trimmed.is_empty() {
        return Err("File path cannot be empty.".to_string());
    }

    let input_path = Path::new(trimmed);
    let target = if input_path.is_absolute() {
        let clean = strip_verbatim_prefix(input_path.to_path_buf());
        if !clean.starts_with(&canon_root) {
            return Err(format!(
                "Access denied: Path '{}' is outside workspace root.",
                trimmed
            ));
        }
        clean
    } else {
        let mut resolved = canon_root.clone();
        for comp in input_path.components() {
            match comp {
                Component::Normal(seg) => resolved.push(seg),
                Component::CurDir => {}
                Component::ParentDir => {
                    if resolved == canon_root {
                        return Err(format!(
                            "Access denied: Path '{}' traverses outside workspace root.",
                            trimmed
                        ));
                    }
                    resolved.pop();
                    if !resolved.starts_with(&canon_root) {
                        return Err(format!(
                            "Access denied: Path '{}' traverses outside workspace root.",
                            trimmed
                        ));
                    }
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(format!(
                        "Access denied: Invalid path component in '{}'.",
                        trimmed
                    ));
                }
            }
        }
        resolved
    };

    if target.exists() {
        let canon_target = canonicalize_clean(&target)
            .map_err(|e| format!("Failed to canonicalize path '{}': {}", target.display(), e))?;
        if !canon_target.starts_with(&canon_root) {
            return Err(format!(
                "Access denied: Path '{}' escapes workspace root via symlink.",
                trimmed
            ));
        }
        Ok(canon_target)
    } else {
        let mut ancestor = target.as_path();
        while let Some(parent) = ancestor.parent() {
            if parent.exists() {
                let canon_parent = canonicalize_clean(parent).map_err(|e| {
                    format!("Failed to canonicalize parent '{}': {}", parent.display(), e)
                })?;
                if !canon_parent.starts_with(&canon_root) {
                    return Err(format!(
                        "Access denied: Parent of path '{}' escapes workspace root.",
                        trimmed
                    ));
                }
                break;
            }
            ancestor = parent;
        }
        Ok(target)
    }
}

pub fn list_tools() -> Vec<McpToolInfo> {
    vec![
        McpToolInfo {
            name: "get_editor_context".to_string(),
            description: "Get the current Bex editor context, including workspace root, active file, entry file, selected LaTeX engine, and current compiler diagnostics.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        McpToolInfo {
            name: "list_project_files".to_string(),
            description: "List all files and directories in the LaTeX project workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        McpToolInfo {
            name: "read_file".to_string(),
            description: "Read the UTF-8 text content of a file within the project workspace (max 512 KB).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative or workspace path to the file to read"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        McpToolInfo {
            name: "write_file".to_string(),
            description: "Write full UTF-8 text content to a file, replacing existing content or creating it if it does not exist.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative or workspace path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The full UTF-8 text content to write"
                    }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        },
        McpToolInfo {
            name: "insert_text".to_string(),
            description: "Insert text into a file at a specific position (before \\end{document}, at the start, at the end, or at a 1-based line number).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to insert"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional relative path to target file (defaults to active file)"
                    },
                    "position": {
                        "type": "string",
                        "enum": ["before_end_document", "end", "start", "line"],
                        "description": "Insertion position (defaults to 'before_end_document')"
                    },
                    "line_number": {
                        "type": "integer",
                        "description": "1-based line number when position is 'line'"
                    }
                },
                "required": ["text"],
                "additionalProperties": false
            }),
        },
        McpToolInfo {
            name: "replace_in_file".to_string(),
            description: "Search and replace text within a file.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "search": {
                        "type": "string",
                        "description": "Exact text substring to search for"
                    },
                    "replacement": {
                        "type": "string",
                        "description": "Replacement text"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional relative path to target file (defaults to active file)"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "If true, replaces all occurrences; if false, replaces first occurrence only (default false)"
                    }
                },
                "required": ["search", "replacement"],
                "additionalProperties": false
            }),
        },
        McpToolInfo {
            name: "create_file".to_string(),
            description: "Create a new file or directory in the workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of file or directory to create"
                    },
                    "content": {
                        "type": "string",
                        "description": "Initial text content if creating a file (optional)"
                    },
                    "is_dir": {
                        "type": "boolean",
                        "description": "True to create a directory, false to create a file (default false)"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        McpToolInfo {
            name: "compile_project".to_string(),
            description: "Compile the LaTeX project using the selected engine or embedded Tectonic engine, returning success status, logs, and diagnostics.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entry_file": {
                        "type": "string",
                        "description": "Optional entry file relative to workspace root (defaults to project entry file or main.tex)"
                    }
                },
                "additionalProperties": false
            }),
        },
    ]
}

async fn execute_tool(
    name: &str,
    args: &serde_json::Value,
    ctx: &McpContext,
) -> McpExecutionOutcome {
    let canon_root = match canonicalize_clean(&ctx.workspace_root) {
        Ok(r) => r,
        Err(e) => {
            return McpExecutionOutcome {
                text: format!("Error resolving workspace root: {}", e),
                is_error: true,
                mutation: None,
                compile_result: None,
            }
        }
    };

    match name {
        "get_editor_context" => {
            let info = json!({
                "workspaceRoot": path_to_clean_string(&canon_root),
                "activeFile": ctx.active_file,
                "entryFile": ctx.entry_file,
                "selectedEngine": ctx.selected_engine,
                "diagnostics": ctx.diagnostics,
            });
            McpExecutionOutcome {
                text: serde_json::to_string_pretty(&info).unwrap_or_else(|_| "{}".to_string()),
                is_error: false,
                mutation: None,
                compile_result: None,
            }
        }

        "list_project_files" => {
            match crate::workspace::list_workspace_files(path_to_clean_string(&canon_root)) {
                Ok(files) => McpExecutionOutcome {
                    text: serde_json::to_string_pretty(&files)
                        .unwrap_or_else(|_| "[]".to_string()),
                    is_error: false,
                    mutation: None,
                    compile_result: None,
                },
                Err(e) => McpExecutionOutcome {
                    text: format!("Error listing project files: {}", e),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                },
            }
        }

        "read_file" => {
            let path_str = match args.get("path").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => {
                    return McpExecutionOutcome {
                        text: "Missing required parameter 'path'.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let safe_path = match resolve_safe_path(&canon_root, path_str) {
                Ok(p) => p,
                Err(e) => {
                    return McpExecutionOutcome {
                        text: e,
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let meta = match fs::metadata(&safe_path) {
                Ok(m) => m,
                Err(e) => {
                    return McpExecutionOutcome {
                        text: format!("Cannot access '{}': {}", path_str, e),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            if meta.is_dir() {
                return McpExecutionOutcome {
                    text: format!("Cannot read directory '{}' as a file.", path_str),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                };
            }

            if meta.len() > MAX_READ_BYTES {
                return McpExecutionOutcome {
                    text: format!(
                        "File '{}' exceeds maximum allowed size of 512 KB (size: {} bytes).",
                        path_str,
                        meta.len()
                    ),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                };
            }

            match fs::read_to_string(&safe_path) {
                Ok(content) => McpExecutionOutcome {
                    text: content,
                    is_error: false,
                    mutation: None,
                    compile_result: None,
                },
                Err(e) => McpExecutionOutcome {
                    text: format!("Failed to read UTF-8 file '{}': {}", path_str, e),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                },
            }
        }

        "write_file" => {
            let path_str = match args.get("path").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => {
                    return McpExecutionOutcome {
                        text: "Missing required parameter 'path'.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let content = match args.get("content").and_then(|v| v.as_str()) {
                Some(c) => c,
                None => {
                    return McpExecutionOutcome {
                        text: "Missing required parameter 'content'.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let safe_path = match resolve_safe_path(&canon_root, path_str) {
                Ok(p) => p,
                Err(e) => {
                    return McpExecutionOutcome {
                        text: e,
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            if safe_path.is_dir() {
                return McpExecutionOutcome {
                    text: format!("Cannot write to directory path '{}'.", path_str),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                };
            }

            match write_utf8_atomic(&safe_path, content) {
                Ok(bytes) => {
                    let rel = to_unix_rel_path(
                        safe_path.strip_prefix(&canon_root).unwrap_or(&safe_path),
                    );
                    let abs = path_to_clean_string(&safe_path);
                    McpExecutionOutcome {
                        text: format!("Successfully wrote {} bytes to '{}'.", bytes, rel),
                        is_error: false,
                        mutation: Some(EditorMutation {
                            kind: "write".to_string(),
                            rel_path: rel,
                            abs_path: abs,
                            content: Some(content.to_string()),
                        }),
                        compile_result: None,
                    }
                }
                Err(e) => McpExecutionOutcome {
                    text: format!("Failed to write to '{}': {}", path_str, e),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                },
            }
        }

        "insert_text" => {
            let text_to_insert = match args.get("text").and_then(|v| v.as_str()) {
                Some(t) => t,
                None => {
                    return McpExecutionOutcome {
                        text: "Missing required parameter 'text'.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let target_rel = args
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| ctx.active_file.clone());

            let path_str = match target_rel {
                Some(p) => p,
                None => {
                    return McpExecutionOutcome {
                        text: "No file path specified and no active file is open.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let safe_path = match resolve_safe_path(&canon_root, &path_str) {
                Ok(p) => p,
                Err(e) => {
                    return McpExecutionOutcome {
                        text: e,
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let existing = match fs::read_to_string(&safe_path) {
                Ok(c) => c,
                Err(e) => {
                    return McpExecutionOutcome {
                        text: format!("Failed to read target file '{}': {}", path_str, e),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let position = args
                .get("position")
                .and_then(|v| v.as_str())
                .unwrap_or("before_end_document");

            let new_content = match position {
                "before_end_document" => {
                    if let Some(idx) = existing.rfind("\\end{document}") {
                        let prefix = &existing[..idx];
                        let suffix = &existing[idx..];
                        let lead_sep = if prefix.ends_with('\n') { "" } else { "\n" };
                        let trail_sep = if text_to_insert.ends_with('\n') { "\n" } else { "\n\n" };
                        format!("{prefix}{lead_sep}{text_to_insert}{trail_sep}{suffix}")
                    } else {
                        let lead_sep = if existing.ends_with('\n') { "" } else { "\n" };
                        format!("{existing}{lead_sep}{text_to_insert}\n")
                    }
                }
                "start" => {
                    let trail_sep = if text_to_insert.ends_with('\n') { "" } else { "\n" };
                    format!("{text_to_insert}{trail_sep}{existing}")
                }
                "end" => {
                    let lead_sep = if existing.ends_with('\n') { "" } else { "\n" };
                    format!("{existing}{lead_sep}{text_to_insert}\n")
                }
                "line" => {
                    let line_num = args
                        .get("line_number")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as usize;
                    let mut lines: Vec<&str> = existing.lines().collect();
                    let insert_idx = if line_num == 0 {
                        0
                    } else if line_num > lines.len() {
                        lines.len()
                    } else {
                        line_num - 1
                    };
                    lines.insert(insert_idx, text_to_insert);
                    let mut joined = lines.join("\n");
                    if existing.ends_with('\n') {
                        joined.push('\n');
                    }
                    joined
                }
                other => {
                    return McpExecutionOutcome {
                        text: format!("Unknown position '{}'. Allowed: before_end_document, start, end, line.", other),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            match write_utf8_atomic(&safe_path, &new_content) {
                Ok(bytes) => {
                    let rel = to_unix_rel_path(
                        safe_path.strip_prefix(&canon_root).unwrap_or(&safe_path),
                    );
                    let abs = path_to_clean_string(&safe_path);
                    McpExecutionOutcome {
                        text: format!(
                            "Inserted text into '{}' at position '{}' ({} bytes written).",
                            rel, position, bytes
                        ),
                        is_error: false,
                        mutation: Some(EditorMutation {
                            kind: "insert".to_string(),
                            rel_path: rel,
                            abs_path: abs,
                            content: Some(new_content),
                        }),
                        compile_result: None,
                    }
                }
                Err(e) => McpExecutionOutcome {
                    text: format!("Failed to write updated content to '{}': {}", path_str, e),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                },
            }
        }

        "replace_in_file" => {
            let search = match args.get("search").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => {
                    return McpExecutionOutcome {
                        text: "Missing required parameter 'search'.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let replacement = match args.get("replacement").and_then(|v| v.as_str()) {
                Some(r) => r,
                None => {
                    return McpExecutionOutcome {
                        text: "Missing required parameter 'replacement'.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let target_rel = args
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| ctx.active_file.clone());

            let path_str = match target_rel {
                Some(p) => p,
                None => {
                    return McpExecutionOutcome {
                        text: "No file path specified and no active file is open.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let safe_path = match resolve_safe_path(&canon_root, &path_str) {
                Ok(p) => p,
                Err(e) => {
                    return McpExecutionOutcome {
                        text: e,
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let existing = match fs::read_to_string(&safe_path) {
                Ok(c) => c,
                Err(e) => {
                    return McpExecutionOutcome {
                        text: format!("Failed to read target file '{}': {}", path_str, e),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            if !existing.contains(search) {
                return McpExecutionOutcome {
                    text: format!("Search string '{}' not found in '{}'.", search, path_str),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                };
            }

            let replace_all = args
                .get("replace_all")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let new_content = if replace_all {
                existing.replace(search, replacement)
            } else {
                existing.replacen(search, replacement, 1)
            };

            match write_utf8_atomic(&safe_path, &new_content) {
                Ok(bytes) => {
                    let rel = to_unix_rel_path(
                        safe_path.strip_prefix(&canon_root).unwrap_or(&safe_path),
                    );
                    let abs = path_to_clean_string(&safe_path);
                    McpExecutionOutcome {
                        text: format!(
                            "Replaced occurrence(s) in '{}' ({} bytes written).",
                            rel, bytes
                        ),
                        is_error: false,
                        mutation: Some(EditorMutation {
                            kind: "replace".to_string(),
                            rel_path: rel,
                            abs_path: abs,
                            content: Some(new_content),
                        }),
                        compile_result: None,
                    }
                }
                Err(e) => McpExecutionOutcome {
                    text: format!("Failed to write replaced content to '{}': {}", path_str, e),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                },
            }
        }

        "create_file" => {
            let path_str = match args.get("path").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => {
                    return McpExecutionOutcome {
                        text: "Missing required parameter 'path'.".to_string(),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            let is_dir = args
                .get("is_dir")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let content_opt = args.get("content").and_then(|v| v.as_str());

            let safe_path = match resolve_safe_path(&canon_root, path_str) {
                Ok(p) => p,
                Err(e) => {
                    return McpExecutionOutcome {
                        text: e,
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    }
                }
            };

            if safe_path.exists() {
                return McpExecutionOutcome {
                    text: format!("Target path '{}' already exists.", path_str),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                };
            }

            let rel =
                to_unix_rel_path(safe_path.strip_prefix(&canon_root).unwrap_or(&safe_path));
            let abs = path_to_clean_string(&safe_path);

            if is_dir {
                match fs::create_dir_all(&safe_path) {
                    Ok(_) => McpExecutionOutcome {
                        text: format!("Directory '{}' created successfully.", rel),
                        is_error: false,
                        mutation: Some(EditorMutation {
                            kind: "create".to_string(),
                            rel_path: rel,
                            abs_path: abs,
                            content: None,
                        }),
                        compile_result: None,
                    },
                    Err(e) => McpExecutionOutcome {
                        text: format!("Failed to create directory '{}': {}", path_str, e),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    },
                }
            } else {
                let initial = content_opt.unwrap_or("");
                match write_utf8_atomic(&safe_path, initial) {
                    Ok(bytes) => McpExecutionOutcome {
                        text: format!("File '{}' created successfully ({} bytes).", rel, bytes),
                        is_error: false,
                        mutation: Some(EditorMutation {
                            kind: "create".to_string(),
                            rel_path: rel,
                            abs_path: abs,
                            content: Some(initial.to_string()),
                        }),
                        compile_result: None,
                    },
                    Err(e) => McpExecutionOutcome {
                        text: format!("Failed to create file '{}': {}", path_str, e),
                        is_error: true,
                        mutation: None,
                        compile_result: None,
                    },
                }
            }
        }

        "compile_project" => {
            let entry = args
                .get("entry_file")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| ctx.entry_file.clone())
                .unwrap_or_else(|| "main.tex".to_string());

            let compile_req = CompileRequest {
                root_dir: path_to_clean_string(&canon_root),
                entry_file: entry,
                engine: ctx.selected_engine.clone(),
            };

            match compile_latex(compile_req).await {
                Ok(comp_res) => {
                    let diag_count = comp_res.diagnostics.len();
                    let diag_summary = if diag_count > 0 {
                        format!(
                            "\nDiagnostics ({}):\n{}",
                            diag_count,
                            comp_res
                                .diagnostics
                                .iter()
                                .take(10)
                                .map(|d| format!(
                                    "  [{}] {}: {}",
                                    d.severity,
                                    d.file.as_deref().unwrap_or("?"),
                                    d.message
                                ))
                                .collect::<Vec<_>>()
                                .join("\n")
                        )
                    } else {
                        String::new()
                    };

                    let status_str = if comp_res.success {
                        format!(
                            "LaTeX compilation SUCCEEDED in {} ms with engine '{}'. PDF generated at: {}{}",
                            comp_res.duration_ms,
                            comp_res.engine_used,
                            comp_res.pdf_path.as_deref().unwrap_or("in-memory"),
                            diag_summary
                        )
                    } else {
                        format!(
                            "LaTeX compilation FAILED in {} ms with engine '{}'.{}",
                            comp_res.duration_ms, comp_res.engine_used, diag_summary
                        )
                    };

                    let is_err = !comp_res.success;
                    McpExecutionOutcome {
                        text: status_str,
                        is_error: is_err,
                        mutation: None,
                        compile_result: Some(comp_res),
                    }
                }
                Err(e) => McpExecutionOutcome {
                    text: format!("LaTeX compiler invocation error: {}", e),
                    is_error: true,
                    mutation: None,
                    compile_result: None,
                },
            }
        }

        unknown => McpExecutionOutcome {
            text: format!("Unknown tool name '{}'.", unknown),
            is_error: true,
            mutation: None,
            compile_result: None,
        },
    }
}

fn collect_resources_from_nodes(nodes: &[FileNode], out: &mut Vec<serde_json::Value>) {
    for node in nodes {
        if node.is_dir {
            if let Some(children) = &node.children {
                collect_resources_from_nodes(children, out);
            }
        } else {
            let ext = node.extension.as_deref().unwrap_or("");
            let mime = match ext {
                "tex" => Some("text/x-tex"),
                "bib" => Some("text/x-bibtex"),
                "md" => Some("text/markdown"),
                "txt" => Some("text/plain"),
                _ => None,
            };
            if let Some(mime_type) = mime {
                out.push(json!({
                    "uri": format!("bex://workspace/{}", node.path),
                    "name": node.name,
                    "mimeType": mime_type
                }));
            }
        }
    }
}

pub async fn dispatch_jsonrpc(
    req: JsonRpcRequest,
    ctx: &McpContext,
) -> (JsonRpcResponse, Option<EditorMutation>, Option<CompileResponse>) {
    let id = req.id.clone();
    let method = req.method.as_str();

    match method {
        "initialize" => {
            let resp = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": { "subscribe": false, "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "bex-mcp-latex-server",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })),
                error: None,
            };
            (resp, None, None)
        }

        "tools/list" => {
            let tools = list_tools();
            let resp = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "tools": tools
                })),
                error: None,
            };
            (resp, None, None)
        }

        "tools/call" => {
            let params = match req.params.as_ref() {
                Some(p) => p,
                None => {
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing params object for tools/call.".to_string(),
                            data: None,
                        }),
                    };
                    return (resp, None, None);
                }
            };

            let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                Some(n) => n,
                None => {
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing required parameter 'name' in tools/call.".to_string(),
                            data: None,
                        }),
                    };
                    return (resp, None, None);
                }
            };

            let empty_map = json!({});
            let args = params.get("arguments").unwrap_or(&empty_map);

            let outcome = execute_tool(tool_name, args, ctx).await;

            let resp = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": outcome.text
                        }
                    ],
                    "isError": outcome.is_error
                })),
                error: None,
            };

            (resp, outcome.mutation, outcome.compile_result)
        }

        "resources/list" => {
            let mut resources = Vec::new();
            if let Ok(canon_root) = canonicalize_clean(&ctx.workspace_root) {
                if let Ok(files) =
                    crate::workspace::list_workspace_files(path_to_clean_string(&canon_root))
                {
                    collect_resources_from_nodes(&files, &mut resources);
                }
            }

            let resp = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "resources": resources
                })),
                error: None,
            };
            (resp, None, None)
        }

        "resources/read" => {
            let uri = req
                .params
                .as_ref()
                .and_then(|p| p.get("uri"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let rel_path = uri
                .strip_prefix("bex://workspace/")
                .or_else(|| uri.strip_prefix("bex://workspace"))
                .unwrap_or(uri);

            let canon_root = match canonicalize_clean(&ctx.workspace_root) {
                Ok(r) => r,
                Err(e) => {
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: format!("Workspace error: {}", e),
                            data: None,
                        }),
                    };
                    return (resp, None, None);
                }
            };

            let safe_path = match resolve_safe_path(&canon_root, rel_path) {
                Ok(p) => p,
                Err(e) => {
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: e,
                            data: None,
                        }),
                    };
                    return (resp, None, None);
                }
            };

            match fs::read_to_string(&safe_path) {
                Ok(content) => {
                    let ext = safe_path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let mime = match ext {
                        "tex" => "text/x-tex",
                        "bib" => "text/x-bibtex",
                        "md" => "text/markdown",
                        _ => "text/plain",
                    };
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(json!({
                            "contents": [
                                {
                                    "uri": uri,
                                    "mimeType": mime,
                                    "text": content
                                }
                            ]
                        })),
                        error: None,
                    };
                    (resp, None, None)
                }
                Err(e) => {
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: format!("Failed to read resource '{}': {}", uri, e),
                            data: None,
                        }),
                    };
                    (resp, None, None)
                }
            }
        }

        _ => {
            let resp = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method '{}' not found.", method),
                    data: None,
                }),
            };
            (resp, None, None)
        }
    }
}

#[tauri::command]
pub fn mcp_list_tools() -> Vec<McpToolInfo> {
    list_tools()
}

#[tauri::command]
pub async fn mcp_jsonrpc(
    request: JsonRpcRequest,
    workspace_root: String,
    active_file: Option<String>,
    entry_file: Option<String>,
    selected_engine: Option<String>,
) -> Result<serde_json::Value, String> {
    let ctx = McpContext {
        workspace_root: PathBuf::from(&workspace_root),
        active_file,
        entry_file,
        selected_engine: selected_engine.unwrap_or_else(|| "tectonic".to_string()),
        diagnostics: Vec::new(),
    };

    let (response, _, _) = dispatch_jsonrpc(request, &ctx).await;
    serde_json::to_value(&response).map_err(|e| format!("Failed to serialize MCP response: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempMcpWorkspace {
        dir: PathBuf,
    }

    impl TempMcpWorkspace {
        fn new(label: &str) -> Self {
            let unique = format!(
                "bex-mcp-test-{}-{}-{}",
                label,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let dir = std::env::temp_dir().join(unique);
            fs::create_dir_all(&dir).expect("create temp mcp dir");
            Self { dir }
        }
    }

    impl Drop for TempMcpWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[tokio::test]
    async fn test_mcp_initialize() {
        let ws = TempMcpWorkspace::new("init");
        let ctx = McpContext {
            workspace_root: ws.dir.clone(),
            active_file: None,
            entry_file: None,
            selected_engine: "tectonic".to_string(),
            diagnostics: vec![],
        };

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let (resp, mut_opt, comp_opt) = dispatch_jsonrpc(req, &ctx).await;
        assert!(mut_opt.is_none());
        assert!(comp_opt.is_none());
        assert!(resp.error.is_none());
        let res = resp.result.expect("result present");
        assert_eq!(res["protocolVersion"], "2024-11-05");
        assert_eq!(res["serverInfo"]["name"], "bex-mcp-latex-server");
    }

    #[tokio::test]
    async fn test_mcp_tools_list() {
        let ws = TempMcpWorkspace::new("tools-list");
        let ctx = McpContext {
            workspace_root: ws.dir.clone(),
            active_file: None,
            entry_file: None,
            selected_engine: "tectonic".to_string(),
            diagnostics: vec![],
        };

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!("req-2")),
            method: "tools/list".to_string(),
            params: None,
        };

        let (resp, _, _) = dispatch_jsonrpc(req, &ctx).await;
        let res = resp.result.expect("tools result");
        let tools = res["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 8);
        let names: Vec<&str> = tools
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"get_editor_context"));
        assert!(names.contains(&"list_project_files"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"insert_text"));
        assert!(names.contains(&"replace_in_file"));
        assert!(names.contains(&"create_file"));
        assert!(names.contains(&"compile_project"));
    }

    #[tokio::test]
    async fn test_mcp_tools_call_crud_workflow() {
        let ws = TempMcpWorkspace::new("crud-workflow");
        let ctx = McpContext {
            workspace_root: ws.dir.clone(),
            active_file: Some("main.tex".to_string()),
            entry_file: Some("main.tex".to_string()),
            selected_engine: "tectonic".to_string(),
            diagnostics: vec![],
        };

        // 1. Create file
        let create_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(10)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "create_file",
                "arguments": {
                    "path": "main.tex",
                    "content": "\\documentclass{article}\n\\begin{document}\nHello World\n\\end{document}\n"
                }
            })),
        };
        let (resp, mut_opt, _) = dispatch_jsonrpc(create_req, &ctx).await;
        assert!(resp.error.is_none());
        assert_eq!(mut_opt.as_ref().map(|m| m.kind.as_str()), Some("create"));

        // 2. Read file
        let read_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(11)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "read_file",
                "arguments": {
                    "path": "main.tex"
                }
            })),
        };
        let (resp2, _, _) = dispatch_jsonrpc(read_req, &ctx).await;
        let content_text = resp2.result.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(content_text.contains("Hello World"));

        // 3. Insert text before \end{document}
        let insert_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(12)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "insert_text",
                "arguments": {
                    "path": "main.tex",
                    "text": "\\section{Methodology}\nWe test Bex MCP.",
                    "position": "before_end_document"
                }
            })),
        };
        let (resp3, mut3, _) = dispatch_jsonrpc(insert_req, &ctx).await;
        assert!(resp3.error.is_none());
        assert_eq!(mut3.as_ref().map(|m| m.kind.as_str()), Some("insert"));
        let updated = fs::read_to_string(ws.dir.join("main.tex")).unwrap();
        assert!(updated.contains("\\section{Methodology}"));
        assert!(updated.ends_with("\\end{document}\n") || updated.ends_with("\\end{document}"));

        // 4. Replace in file
        let replace_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(13)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "replace_in_file",
                "arguments": {
                    "path": "main.tex",
                    "search": "Hello World",
                    "replacement": "Hello Bex AI",
                    "replace_all": false
                }
            })),
        };
        let (resp4, mut4, _) = dispatch_jsonrpc(replace_req, &ctx).await;
        assert!(resp4.error.is_none());
        assert_eq!(mut4.as_ref().map(|m| m.kind.as_str()), Some("replace"));
        let replaced_content = fs::read_to_string(ws.dir.join("main.tex")).unwrap();
        assert!(replaced_content.contains("Hello Bex AI"));
        assert!(!replaced_content.contains("Hello World"));
    }

    #[tokio::test]
    async fn test_path_traversal_rejection() {
        let ws = TempMcpWorkspace::new("traversal-guard");
        let ctx = McpContext {
            workspace_root: ws.dir.clone(),
            active_file: None,
            entry_file: None,
            selected_engine: "tectonic".to_string(),
            diagnostics: vec![],
        };

        // Attempting to read outside workspace root MUST fail
        let traversal_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(99)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "read_file",
                "arguments": {
                    "path": "../outside.txt"
                }
            })),
        };

        let (resp, _, _) = dispatch_jsonrpc(traversal_req, &ctx).await;
        let res = resp.result.expect("result object");
        assert_eq!(res["isError"], true);
        let err_msg = res["content"][0]["text"].as_str().unwrap();
        assert!(err_msg.contains("Access denied") || err_msg.contains("traverses outside"));
    }
}

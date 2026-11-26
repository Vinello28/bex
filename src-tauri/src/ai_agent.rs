use crate::compiler::{compile_latex, CompileRequest, CompileResponse, LatexDiagnostic};
use crate::mcp::{
    dispatch_jsonrpc, list_tools, EditorMutation, JsonRpcRequest, McpContext,
    McpToolInfo,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const MAX_ATTACHMENT_CHARS: usize = 60_000;
const MAX_AGENT_TURNS: usize = 8;
const HTTP_TIMEOUT_SECS: u64 = 90;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub auto_compile_after_ai_edit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiEnvStatus {
    pub has_google_env_key: bool,
    pub default_google_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAttachment {
    pub name: String,
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatHistoryItem {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolTrace {
    pub id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub result_summary: String,
    pub is_error: bool,
    pub mutation: Option<EditorMutation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAgentRequest {
    pub settings: AiSettings,
    pub messages: Vec<AiChatHistoryItem>,
    pub attachments: Vec<AiAttachment>,
    pub workspace_root: String,
    pub active_file_path: Option<String>,
    pub active_file_content: Option<String>,
    pub entry_file: Option<String>,
    pub selected_engine: String,
    pub diagnostics: Vec<LatexDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAgentResponse {
    pub assistant_message: String,
    pub tool_traces: Vec<McpToolTrace>,
    pub mutations: Vec<EditorMutation>,
    pub compile_result: Option<CompileResponse>,
}

/// Parses a key from dotenv file contents line-by-line.
/// Handles comments, whitespace, `export KEY=val`, and quoted values.
pub fn parse_dotenv_key(contents: &str, key_name: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let without_export = if let Some(stripped) = trimmed.strip_prefix("export ") {
            stripped.trim()
        } else {
            trimmed
        };

        if let Some((k, v)) = without_export.split_once('=') {
            let k = k.trim();
            if k == key_name {
                let v = v.trim();
                let clean_val = if (v.starts_with('"') && v.ends_with('"'))
                    || (v.starts_with('\'') && v.ends_with('\''))
                {
                    if v.len() >= 2 {
                        &v[1..v.len() - 1]
                    } else {
                        v
                    }
                } else {
                    v
                };
                let result = clean_val.trim();
                if !result.is_empty() {
                    return Some(result.to_string());
                }
            }
        }
    }
    None
}

/// Resolves Google AI Studio key from environment or local `.env` files.
/// Never logs or displays the raw key!
pub fn resolve_google_env_key() -> Option<String> {
    if let Ok(key) = std::env::var("GOOGLE_AI_STUDIO") {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Ok(mut current) = std::env::current_dir() {
        for _ in 0..3 {
            let candidate = current.join(".env");
            if candidate.is_file() {
                if let Ok(contents) = fs::read_to_string(&candidate) {
                    if let Some(key) = parse_dotenv_key(&contents, "GOOGLE_AI_STUDIO") {
                        return Some(key);
                    }
                }
            }
            if !current.pop() {
                break;
            }
        }
    }

    None
}

/// Recursively sanitizes JSON schemas for Google Gemini function declarations:
/// uppercases types, deletes `additionalProperties`, and recurses properties/items.
pub fn sanitize_gemini_schema(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if k == "additionalProperties" {
                    continue;
                }
                if k == "type" {
                    if let Some(type_str) = v.as_str() {
                        let lower = type_str.to_ascii_lowercase();
                        let upper = match lower.as_str() {
                            "string" => "STRING",
                            "object" => "OBJECT",
                            "array" => "ARRAY",
                            "number" => "NUMBER",
                            "integer" => "INTEGER",
                            "boolean" => "BOOLEAN",
                            other => other,
                        };
                        out.insert(k.clone(), json!(upper));
                        continue;
                    }
                }
                out.insert(k.clone(), sanitize_gemini_schema(v));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            let out: Vec<serde_json::Value> = arr.iter().map(sanitize_gemini_schema).collect();
            serde_json::Value::Array(out)
        }
        other => other.clone(),
    }
}

pub fn convert_mcp_to_gemini_tools(tools: &[McpToolInfo]) -> serde_json::Value {
    let declarations: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "parameters": sanitize_gemini_schema(&t.input_schema)
            })
        })
        .collect();

    json!([
        {
            "functionDeclarations": declarations
        }
    ])
}

pub fn convert_mcp_to_openai_tools(tools: &[McpToolInfo]) -> serde_json::Value {
    let out: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema
                }
            })
        })
        .collect();

    serde_json::Value::Array(out)
}

fn truncate_summary(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut s = text.chars().take(max_len).collect::<String>();
        s.push_str("... [truncated]");
        s
    }
}

fn build_system_prompt(req: &AiAgentRequest) -> String {
    let diag_str = if req.diagnostics.is_empty() {
        "No current compiler diagnostics.".to_string()
    } else {
        format!(
            "Current compiler diagnostics ({}):\n{}",
            req.diagnostics.len(),
            req.diagnostics
                .iter()
                .take(15)
                .map(|d| format!(
                    "- [{}] {}: {}",
                    d.severity,
                    d.file.as_deref().unwrap_or("?"),
                    d.message
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let active_file_str = req
        .active_file_path
        .as_deref()
        .unwrap_or("None (no file currently focused)");
    let entry_file_str = req.entry_file.as_deref().unwrap_or("main.tex");

    let active_content_snippet = if let Some(content) = &req.active_file_content {
        let truncated = if content.chars().count() > 8000 {
            let prefix: String = content.chars().take(8000).collect();
            format!(
                "{}\n... [remainder of active file omitted, use read_file for full content]",
                prefix
            )
        } else {
            content.clone()
        };
        format!("\nActive File Content Preview:\n```latex\n{}\n```", truncated)
    } else {
        String::new()
    };

    format!(
        r#"You are Bex AI, an expert LaTeX typesetting, authoring, and scientific document assistant embedded inside Bex LaTeX Studio.

WORKSPACE CONTEXT:
- Workspace Root: {}
- Active File: {}
- Entry File: {}
- Selected Engine: {}
- Diagnostics:
{}
{}

CAPABILITIES & MCP TOOLS:
You have direct access to Model Context Protocol (MCP) workspace tools:
- `get_editor_context`: Retrieve full editor and project context.
- `list_project_files`: Explore the project workspace tree.
- `read_file`: Read full UTF-8 text of files (up to 512 KB).
- `write_file`: Overwrite or create full file contents.
- `insert_text`: Insert text at position (`before_end_document`, `end`, `start`, `line`).
- `replace_in_file`: Search and replace exact text in a file.
- `create_file`: Create a new file or directory.
- `compile_project`: Compile the LaTeX project and verify output.

CRITICAL INSTRUCTIONS:
1. When asked to write, fix, insert, create, edit, or refactor LaTeX code, ALWAYS invoke the corresponding MCP tool directly (e.g. `write_file`, `insert_text`, `replace_in_file`, or `create_file`).
2. When asked to resolve compilation errors or inspect files, read the file or inspect diagnostics, apply the fixes via MCP tools, and optionally compile to verify correctness.
3. When user attaches files, summarize, extract, or translate their findings into clear, elegant, well-structured LaTeX sections or tables as requested.
4. Always produce standards-compliant, beautiful LaTeX (using packages like amsmath, booktabs, hyperref, xcolor).
5. When presenting code in chat messages, use fenced code blocks with language annotations (e.g. ```latex). Keep explanations concise and focused."#,
        req.workspace_root,
        active_file_str,
        entry_file_str,
        req.selected_engine,
        diag_str,
        active_content_snippet
    )
}

#[tauri::command]
pub fn ai_get_env_status() -> AiEnvStatus {
    AiEnvStatus {
        has_google_env_key: resolve_google_env_key().is_some(),
        default_google_model: "gemini-2.5-flash".to_string(),
    }
}

#[tauri::command]
pub async fn ai_list_models(
    provider: String,
    base_url: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    match provider.as_str() {
        "google" => {
            let key = if !api_key.trim().is_empty() {
                api_key.trim().to_string()
            } else {
                resolve_google_env_key().ok_or_else(|| {
                    "Google AI Studio API key not found. Please provide an API key or configure GOOGLE_AI_STUDIO in .env.".to_string()
                })?
            };

            let base = if base_url.trim().is_empty() {
                "https://generativelanguage.googleapis.com/v1beta"
            } else {
                base_url.trim().trim_end_matches('/')
            };

            let url = format!("{}/models", base);
            let resp = client
                .get(&url)
                .header("x-goog-api-key", key)
                .send()
                .await
                .map_err(|e| format!("Failed to connect to Google AI Studio at '{}': {}", base, e))?;

            if !resp.status().is_success() {
                return Err(format!(
                    "Google AI Studio returned HTTP status {}. Please check your API key.",
                    resp.status()
                ));
            }

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse Google AI Studio response: {}", e))?;

            let mut models = Vec::new();
            if let Some(arr) = body.get("models").and_then(|v| v.as_array()) {
                for m in arr {
                    let is_generate = m
                        .get("supportedGenerationMethods")
                        .and_then(|v| v.as_array())
                        .map(|methods| {
                            methods.iter().any(|method| {
                                method.as_str() == Some("generateContent")
                            })
                        })
                        .unwrap_or(false);

                    if is_generate {
                        if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                            let clean_name = name.strip_prefix("models/").unwrap_or(name);
                            models.push(clean_name.to_string());
                        }
                    }
                }
            }

            if models.is_empty() {
                models.push("gemini-2.5-flash".to_string());
                models.push("gemini-2.5-pro".to_string());
            }

            Ok(models)
        }

        "ollama" | "lmstudio" | "vllm" => {
            let base = if base_url.trim().is_empty() {
                match provider.as_str() {
                    "ollama" => "http://localhost:11434/v1",
                    "lmstudio" => "http://localhost:1234/v1",
                    _ => "http://localhost:8000/v1",
                }
            } else {
                base_url.trim().trim_end_matches('/')
            };

            let models_url = format!("{}/models", base);
            let mut req = client.get(&models_url);
            if !api_key.trim().is_empty() {
                req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
            }

            let resp_res = req.send().await;

            if let Ok(resp) = resp_res {
                if resp.status().is_success() {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        let mut models = Vec::new();
                        if let Some(data) = body.get("data").and_then(|v| v.as_array()) {
                            for item in data {
                                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                                    models.push(id.to_string());
                                }
                            }
                        }
                        if !models.is_empty() {
                            return Ok(models);
                        }
                    }
                }
            }

            // Ollama native API fallback: /api/tags
            if provider == "ollama" {
                let host_base = base.strip_suffix("/v1").unwrap_or(base);
                let tags_url = format!("{}/api/tags", host_base);
                if let Ok(resp) = client.get(&tags_url).send().await {
                    if resp.status().is_success() {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            let mut models = Vec::new();
                            if let Some(arr) = body.get("models").and_then(|v| v.as_array()) {
                                for item in arr {
                                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                                        models.push(name.to_string());
                                    }
                                }
                            }
                            if !models.is_empty() {
                                return Ok(models);
                            }
                        }
                    }
                }
            }

            Err(format!(
                "Could not connect to {} at '{}'. Please make sure the service is running and accessible.",
                provider, base
            ))
        }

        unknown => Err(format!("Unsupported AI provider '{}'.", unknown)),
    }
}

#[tauri::command]
pub async fn ai_send_message(request: AiAgentRequest) -> Result<AiAgentResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mcp_tools = list_tools();
    let system_prompt = build_system_prompt(&request);

    // Prepare attachments to append to user message
    let mut attachments_text = String::new();
    for att in &request.attachments {
        let content = if att.content.len() > MAX_ATTACHMENT_CHARS {
            let mut s = att.content.chars().take(MAX_ATTACHMENT_CHARS).collect::<String>();
            s.push_str("\n[Attachment truncated at 60,000 characters]");
            s
        } else {
            att.content.clone()
        };

        attachments_text.push_str(&format!(
            "\n\n--- Attached File: {} ({}) ---\n{}\n--- End of Attached File ---\n",
            att.name, att.path, content
        ));
    }

    let mcp_ctx = McpContext {
        workspace_root: PathBuf::from(&request.workspace_root),
        active_file: request.active_file_path.clone(),
        entry_file: request.entry_file.clone(),
        selected_engine: request.selected_engine.clone(),
        diagnostics: request.diagnostics.clone(),
    };

    let mut tool_traces: Vec<McpToolTrace> = Vec::new();
    let mut mutations: Vec<EditorMutation> = Vec::new();
    let mut compile_result: Option<CompileResponse> = None;
    let mut assistant_message = String::new();

    let provider = request.settings.provider.to_lowercase();

    match provider.as_str() {
        "google" => {
            let key = if !request.settings.api_key.trim().is_empty() {
                request.settings.api_key.trim().to_string()
            } else {
                resolve_google_env_key().ok_or_else(|| {
                    "Missing API Key. Please provide a Google AI Studio API key in AI settings or configure GOOGLE_AI_STUDIO in your .env file.".to_string()
                })?
            };

            let base = if request.settings.base_url.trim().is_empty() {
                "https://generativelanguage.googleapis.com/v1beta"
            } else {
                request.settings.base_url.trim().trim_end_matches('/')
            };

            let model = if request.settings.model.trim().is_empty() {
                "gemini-2.5-flash"
            } else {
                request.settings.model.trim()
            };

            let gemini_tools = convert_mcp_to_gemini_tools(&mcp_tools);

            let mut contents: Vec<serde_json::Value> = Vec::new();
            let msg_len = request.messages.len();

            for (idx, msg) in request.messages.iter().enumerate() {
                let role = if msg.role == "assistant" {
                    "model"
                } else {
                    "user"
                };
                let mut text = msg.content.clone();
                if idx == msg_len.saturating_sub(1) && role == "user" && !attachments_text.is_empty() {
                    text.push_str(&attachments_text);
                }
                contents.push(json!({
                    "role": role,
                    "parts": [{ "text": text }]
                }));
            }

            if contents.is_empty() {
                let text = if attachments_text.is_empty() {
                    "Hello".to_string()
                } else {
                    format!("Analyze the attached file:\n{}", attachments_text)
                };
                contents.push(json!({
                    "role": "user",
                    "parts": [{ "text": text }]
                }));
            }

            let endpoint = format!("{}/models/{}:generateContent", base, model);

            for turn in 0..MAX_AGENT_TURNS {
                let req_body = json!({
                    "contents": contents,
                    "systemInstruction": {
                        "parts": [{ "text": system_prompt }]
                    },
                    "tools": gemini_tools
                });

                let res = client
                    .post(&endpoint)
                    .header("x-goog-api-key", &key)
                    .json(&req_body)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to reach Google AI Studio: {}", e))?;

                if !res.status().is_success() {
                    let status = res.status();
                    let err_body = res.text().await.unwrap_or_default();
                    return Err(format!(
                        "Google AI Studio error (status {}): {}",
                        status,
                        truncate_summary(&err_body, 300)
                    ));
                }

                let res_json: serde_json::Value = res
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse Google Gemini response: {}", e))?;

                let candidate = match res_json
                    .get("candidates")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                {
                    Some(c) => c,
                    None => {
                        return Err(
                            "Google AI Studio returned no candidate responses.".to_string()
                        );
                    }
                };

                let parts = candidate
                    .get("content")
                    .and_then(|c| c.get("parts"))
                    .and_then(|p| p.as_array())
                    .cloned()
                    .unwrap_or_default();

                let mut model_fc_parts = Vec::new();
                let mut function_calls = Vec::new();
                let mut text_parts = Vec::new();

                for part in &parts {
                    if let Some(fc) = part.get("functionCall") {
                        model_fc_parts.push(part.clone());
                        function_calls.push(fc.clone());
                    }
                    if let Some(t) = part.get("text").and_then(|s| s.as_str()) {
                        text_parts.push(t.to_string());
                    }
                }

                if !function_calls.is_empty() {
                    let mut response_parts = Vec::new();

                    for (call_idx, fc) in function_calls.iter().enumerate() {
                        let fn_name = fc
                            .get("name")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let empty_args = json!({});
                        let fn_args = fc.get("args").unwrap_or(&empty_args);

                        let mcp_req = JsonRpcRequest {
                            jsonrpc: "2.0".to_string(),
                            id: Some(json!(turn * 100 + call_idx + 1)),
                            method: "tools/call".to_string(),
                            params: Some(json!({
                                "name": fn_name,
                                "arguments": fn_args
                            })),
                        };

                        let (mcp_res, mut_opt, comp_opt) =
                            dispatch_jsonrpc(mcp_req, &mcp_ctx).await;

                        let outcome_text = mcp_res
                            .result
                            .as_ref()
                            .and_then(|r| r.get("content"))
                            .and_then(|c| c.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|item| item.get("text"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("Tool execution finished.")
                            .to_string();

                        let is_err = mcp_res
                            .result
                            .as_ref()
                            .and_then(|r| r.get("isError"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if let Some(m) = &mut_opt {
                            mutations.push(m.clone());
                        }
                        if comp_opt.is_some() {
                            compile_result = comp_opt;
                        }

                        tool_traces.push(McpToolTrace {
                            id: format!("trace-turn-{}-{}", turn + 1, call_idx + 1),
                            tool_name: fn_name.clone(),
                            arguments_json: fn_args.to_string(),
                            result_summary: truncate_summary(&outcome_text, 400),
                            is_error: is_err,
                            mutation: mut_opt,
                        });

                        response_parts.push(json!({
                            "functionResponse": {
                                "name": fn_name,
                                "response": {
                                    "output": outcome_text
                                }
                            }
                        }));
                    }

                    // Preserve exact model parts (including any thoughtSignature)
                    contents.push(json!({
                        "role": "model",
                        "parts": model_fc_parts
                    }));

                    // Add function response turn matching all calls
                    contents.push(json!({
                        "role": "function",
                        "parts": response_parts
                    }));
                } else {
                    assistant_message = text_parts.join("\n\n");
                    break;
                }
            }
        }

        "ollama" | "lmstudio" | "vllm" => {
            let base = if request.settings.base_url.trim().is_empty() {
                match provider.as_str() {
                    "ollama" => "http://localhost:11434/v1",
                    "lmstudio" => "http://localhost:1234/v1",
                    _ => "http://localhost:8000/v1",
                }
            } else {
                request.settings.base_url.trim().trim_end_matches('/')
            };

            let model = if request.settings.model.trim().is_empty() {
                "llama3.2"
            } else {
                request.settings.model.trim()
            };

            let endpoint = format!("{}/chat/completions", base);
            let openai_tools = convert_mcp_to_openai_tools(&mcp_tools);

            let mut messages: Vec<serde_json::Value> = Vec::new();
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));

            let msg_len = request.messages.len();
            for (idx, msg) in request.messages.iter().enumerate() {
                let mut text = msg.content.clone();
                if idx == msg_len.saturating_sub(1) && msg.role == "user" && !attachments_text.is_empty() {
                    text.push_str(&attachments_text);
                }
                messages.push(json!({
                    "role": msg.role,
                    "content": text
                }));
            }

            if messages.len() == 1 {
                let text = if attachments_text.is_empty() {
                    "Hello".to_string()
                } else {
                    format!("Analyze the attached file:\n{}", attachments_text)
                };
                messages.push(json!({
                    "role": "user",
                    "content": text
                }));
            }

            for turn in 0..MAX_AGENT_TURNS {
                let req_body = json!({
                    "model": model,
                    "messages": messages,
                    "tools": openai_tools,
                    "stream": false
                });

                let mut req = client.post(&endpoint).json(&req_body);
                if !request.settings.api_key.trim().is_empty() {
                    req = req.header(
                        "Authorization",
                        format!("Bearer {}", request.settings.api_key.trim()),
                    );
                }

                let res = req
                    .send()
                    .await
                    .map_err(|e| format!("Failed to reach {}: {}", provider, e))?;

                if !res.status().is_success() {
                    let status = res.status();
                    let err_body = res.text().await.unwrap_or_default();
                    return Err(format!(
                        "{} error (status {}): {}",
                        provider,
                        status,
                        truncate_summary(&err_body, 300)
                    ));
                }

                let res_json: serde_json::Value = res
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response from {}: {}", provider, e))?;

                let message = match res_json
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|c| c.get("message"))
                {
                    Some(m) => m.clone(),
                    None => {
                        return Err(format!(
                            "{} returned no choices in response.",
                            provider
                        ));
                    }
                };

                let tool_calls = message
                    .get("tool_calls")
                    .and_then(|tc| tc.as_array())
                    .cloned()
                    .unwrap_or_default();

                if !tool_calls.is_empty() {
                    messages.push(message.clone());

                    let empty_fn = json!({});
                    for tc in tool_calls {
                        let call_id = tc
                            .get("id")
                            .and_then(|s| s.as_str())
                            .unwrap_or("call_1")
                            .to_string();
                        let fn_obj = tc.get("function").unwrap_or(&empty_fn);
                        let fn_name = fn_obj
                            .get("name")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let args_str = fn_obj
                            .get("arguments")
                            .and_then(|s| s.as_str())
                            .unwrap_or("{}");
                        let fn_args: serde_json::Value =
                            serde_json::from_str(args_str).unwrap_or(json!({}));

                        let mcp_req = JsonRpcRequest {
                            jsonrpc: "2.0".to_string(),
                            id: Some(json!(turn + 1)),
                            method: "tools/call".to_string(),
                            params: Some(json!({
                                "name": fn_name,
                                "arguments": fn_args
                            })),
                        };

                        let (mcp_res, mut_opt, comp_opt) =
                            dispatch_jsonrpc(mcp_req, &mcp_ctx).await;

                        let outcome_text = mcp_res
                            .result
                            .as_ref()
                            .and_then(|r| r.get("content"))
                            .and_then(|c| c.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|item| item.get("text"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("Tool execution finished.")
                            .to_string();

                        let is_err = mcp_res
                            .result
                            .as_ref()
                            .and_then(|r| r.get("isError"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if let Some(m) = &mut_opt {
                            mutations.push(m.clone());
                        }
                        if comp_opt.is_some() {
                            compile_result = comp_opt;
                        }

                        tool_traces.push(McpToolTrace {
                            id: call_id.clone(),
                            tool_name: fn_name.clone(),
                            arguments_json: fn_args.to_string(),
                            result_summary: truncate_summary(&outcome_text, 400),
                            is_error: is_err,
                            mutation: mut_opt,
                        });

                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": call_id,
                            "content": outcome_text
                        }));
                    }
                } else {
                    assistant_message = message
                        .get("content")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    break;
                }
            }
        }

        unknown => {
            return Err(format!("Unsupported AI provider: '{}'", unknown));
        }
    }

    // Auto-compile if edits were made and compile wasn't run yet
    if request.settings.auto_compile_after_ai_edit
        && !mutations.is_empty()
        && compile_result.is_none()
    {
        let entry = request
            .entry_file
            .clone()
            .unwrap_or_else(|| "main.tex".to_string());
        if let Ok(comp) = compile_latex(CompileRequest {
            root_dir: request.workspace_root.clone(),
            entry_file: entry,
            engine: request.selected_engine.clone(),
        })
        .await
        {
            compile_result = Some(comp);
        }
    }

    Ok(AiAgentResponse {
        assistant_message,
        tool_traces,
        mutations,
        compile_result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dotenv_key() {
        let sample_env = r#"
# Comment line
SOME_OTHER_KEY=12345
GOOGLE_AI_STUDIO="AIzaSySampleKey123"
ANOTHER_KEY='single_quoted'
export EXPORTED_KEY=unquoted_val
"#;

        assert_eq!(
            parse_dotenv_key(sample_env, "GOOGLE_AI_STUDIO"),
            Some("AIzaSySampleKey123".to_string())
        );
        assert_eq!(
            parse_dotenv_key(sample_env, "ANOTHER_KEY"),
            Some("single_quoted".to_string())
        );
        assert_eq!(
            parse_dotenv_key(sample_env, "EXPORTED_KEY"),
            Some("unquoted_val".to_string())
        );
        assert_eq!(parse_dotenv_key(sample_env, "NON_EXISTENT"), None);
    }

    #[test]
    fn test_gemini_schema_sanitization() {
        let raw_schema = json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path"
                },
                "count": {
                    "type": "integer"
                },
                "items": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                }
            },
            "required": ["path"]
        });

        let sanitized = sanitize_gemini_schema(&raw_schema);

        assert_eq!(sanitized["type"], "OBJECT");
        assert!(sanitized.get("additionalProperties").is_none());
        assert_eq!(sanitized["properties"]["path"]["type"], "STRING");
        assert_eq!(sanitized["properties"]["count"]["type"], "INTEGER");
        assert_eq!(sanitized["properties"]["items"]["type"], "ARRAY");
        assert_eq!(sanitized["properties"]["items"]["items"]["type"], "STRING");
        assert_eq!(sanitized["required"][0], "path");
    }

    #[test]
    fn test_openai_tool_schema_conversion() {
        let tools = list_tools();
        let openai_val = convert_mcp_to_openai_tools(&tools);
        let arr = openai_val.as_array().expect("array of tools");
        assert_eq!(arr.len(), 8);

        let write_tool = arr
            .iter()
            .find(|t| t["function"]["name"] == "write_file")
            .expect("write_file tool");
        assert_eq!(write_tool["type"], "function");
        assert_eq!(
            write_tool["function"]["description"],
            "Write full UTF-8 text content to a file, replacing existing content or creating it if it does not exist."
        );
        assert_eq!(
            write_tool["function"]["parameters"]["properties"]["path"]["type"],
            "string"
        );
    }
}

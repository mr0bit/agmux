//! MCP JSON-RPC stdio adapter. All operations cross the authenticated daemon IPC seam.
use agmux_proto::IpcMessage;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Tool {
    name: &'static str,
    request: &'static str,
    required: &'static [&'static str],
    optional: &'static [&'static str],
}
const TOOLS: &[Tool] = &[
    Tool {
        name: "list_sessions",
        request: "ListSessionsRequest",
        required: &[],
        optional: &[],
    },
    Tool {
        name: "spawn_session",
        request: "SpawnSessionRequest",
        required: &["session_id", "command"],
        optional: &["args", "cwd", "worktree_branch"],
    },
    Tool {
        name: "send_message",
        request: "SendMessageRequest",
        required: &["target_session_id", "message"],
        optional: &[],
    },
    Tool {
        name: "read_session",
        request: "ReadSessionRequest",
        required: &["session_id"],
        optional: &[],
    },
    Tool {
        name: "task_list",
        request: "ListTasksRequest",
        required: &[],
        optional: &["status"],
    },
    Tool {
        name: "task_create",
        request: "CreateTaskRequest",
        required: &["task_id", "title"],
        optional: &["description"],
    },
    Tool {
        name: "task_claim",
        request: "ClaimTaskRequest",
        required: &["task_id", "session_id"],
        optional: &[],
    },
    Tool {
        name: "task_finish",
        request: "FinishTaskRequest",
        required: &["task_id", "session_id"],
        optional: &[],
    },
    Tool {
        name: "report_submit",
        request: "PostReportRequest",
        required: &["task_id", "session_id", "summary"],
        optional: &["pr_url", "commit_sha"],
    },
    Tool {
        name: "lock_acquire",
        request: "AcquireLockRequest",
        required: &["lock_name", "session_id"],
        optional: &["wait"],
    },
    Tool {
        name: "lock_release",
        request: "ReleaseLockRequest",
        required: &["lock_name", "session_id"],
        optional: &[],
    },
    Tool {
        name: "reserve_files",
        request: "ReserveFilesRequest",
        required: &["session_id", "paths"],
        optional: &[],
    },
    Tool {
        name: "release_files",
        request: "ReleaseFilesRequest",
        required: &["session_id", "paths"],
        optional: &[],
    },
    Tool {
        name: "resolve_review_comment",
        request: "ResolveReviewCommentRequest",
        required: &["comment_id"],
        optional: &["session_id"],
    },
    Tool {
        name: "archive_self",
        request: "TeardownRequest",
        required: &["session_id"],
        optional: &[],
    },
    Tool {
        name: "kill_self",
        request: "TeardownRequest",
        required: &["session_id"],
        optional: &[],
    },
];
fn schema(tool: &Tool) -> Value {
    let mut properties = serde_json::Map::new();
    for &key in tool.required.iter().chain(tool.optional) {
        let field = match key {
            "args" | "paths" => json!({"type":"array","items":{"type":"string"}}),
            "wait" => json!({"type":"boolean"}),
            _ => json!({"type":"string"}),
        };
        properties.insert(key.into(), field);
    }
    json!({"name":tool.name,"description":format!("Agmux {} through daemon IPC",tool.name),"inputSchema":{"type":"object","properties":properties,"required":tool.required,"additionalProperties":false}})
}
fn request(tool: &Tool, args: &Value) -> Result<IpcMessage> {
    let mut payload = args
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("arguments must be an object"))?;
    for key in payload.keys() {
        if !tool.required.contains(&key.as_str()) && !tool.optional.contains(&key.as_str()) {
            bail!("Unknown argument: {key}");
        }
    }
    if !payload.contains_key("session_id") && tool.required.contains(&"session_id") {
        if let Ok(id) = std::env::var("AGMUX_SESSION_ID") {
            payload.insert("session_id".into(), json!(id));
        }
    }
    for &key in tool.required {
        if !payload.contains_key(key) {
            bail!("Missing argument: {key}");
        }
    }
    for &key in tool.optional {
        if !payload.contains_key(key) {
            payload.insert(
                key.into(),
                match key {
                    "args" => json!([]),
                    "description" => json!(""),
                    "wait" => json!(false),
                    _ => Value::Null,
                },
            );
        }
    }
    if tool.name == "archive_self" || tool.name == "kill_self" {
        payload.insert(
            "action".into(),
            json!(if tool.name == "archive_self" {
                "archive"
            } else {
                "kill"
            }),
        );
    }
    if matches!(tool.name, "reserve_files" | "release_files") {
        let cwd = std::env::current_dir()?;
        if let Some(paths) = payload.get_mut("paths").and_then(Value::as_array_mut) {
            for path in paths {
                if let Some(p) = path.as_str() {
                    *path = json!(crate::ipc_client::canonicalize_repo_path(p, &cwd));
                }
            }
        }
    }
    Ok(serde_json::from_value(
        json!({"type":tool.request,"payload":payload}),
    )?)
}
fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

/// Serve newline-delimited JSON-RPC until stdin closes.
pub async fn serve(socket: &Path, token: &str) -> Result<()> {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    while let Some(line) = lines.next_line().await? {
        let response = match serde_json::from_str::<Value>(&line) {
            Err(_) => Some(error(Value::Null, -32700, "Parse error")),
            Ok(value) => handle(value, socket, token).await,
        };
        if let Some(response) = response {
            stdout
                .write_all(serde_json::to_string(&response)?.as_bytes())
                .await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}
async fn handle(value: Value, socket: &Path, token: &str) -> Option<Value> {
    let Some(obj) = value.as_object() else {
        return Some(error(Value::Null, -32600, "Invalid Request"));
    };
    let id = obj.get("id").cloned();
    if obj.get("jsonrpc") != Some(&json!("2.0"))
        || !obj.get("method").is_some_and(Value::is_string)
        || id
            .as_ref()
            .is_some_and(|v| !(v.is_string() || v.is_number() || v.is_null()))
    {
        return Some(error(Value::Null, -32600, "Invalid Request"));
    }
    let method = obj["method"].as_str().unwrap_or_default();
    let id = id?; // Notifications receive no reply.
    let result = match method {
        "initialize" => {
            let valid = obj
                .get("params")
                .and_then(Value::as_object)
                .is_some_and(|p| {
                    p.get("protocolVersion").is_some_and(Value::is_string)
                        && p.get("capabilities").is_some_and(Value::is_object)
                        && p.get("clientInfo").is_some_and(Value::is_object)
                });
            if !valid {
                return Some(error(id, -32602, "Invalid initialize parameters"));
            }
            json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"agmux","version":env!("CARGO_PKG_VERSION")}})
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools":TOOLS.iter().map(schema).collect::<Vec<_>>()}),
        "tools/call" => {
            let params = obj.get("params").and_then(Value::as_object);
            let Some(name) = params.and_then(|p| p.get("name")).and_then(Value::as_str) else {
                return Some(error(id, -32602, "Missing tool name"));
            };
            let Some(tool) = TOOLS.iter().find(|t| t.name == name) else {
                return Some(error(id, -32602, "Unknown tool"));
            };
            let args = params
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let req = match request(tool, &args) {
                Ok(r) => r,
                Err(e) => return Some(error(id, -32602, &e.to_string())),
            };
            match crate::ipc_client::review_request(socket, token, req).await {
                Ok(response) => {
                    let encoded = serde_json::to_value(response).unwrap_or(Value::Null);
                    let payload = encoded.get("payload").cloned().unwrap_or(encoded);
                    let failed = payload.get("success") == Some(&json!(false))
                        || payload.get("error").is_some_and(|e| !e.is_null())
                        || payload.get("acquired") == Some(&json!(false));
                    json!({"content":[{"type":"text","text":payload.to_string()}],"isError":failed})
                }
                Err(e) => {
                    json!({"content":[{"type":"text","text":format!("Daemon IPC error: {e:#}")}],"isError":true})
                }
            }
        }
        _ => return Some(error(id, -32601, "Method not found")),
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}

/// Add an opt-in MCP registration using each supported agent's command-line configuration.
pub fn agent_arguments(
    command: &str,
    args: Vec<String>,
    socket: &Path,
    token_path: &Path,
) -> Result<Vec<String>> {
    let executable = std::env::current_exe()?.to_string_lossy().into_owned();
    let cwd = std::env::current_dir()?;
    let socket = if socket.is_absolute() {
        socket.to_path_buf()
    } else {
        cwd.join(socket)
    };
    let token_path = if token_path.is_absolute() {
        token_path.to_path_buf()
    } else {
        cwd.join(token_path)
    };
    let server_args = vec![
        "--socket".to_string(),
        socket.to_string_lossy().into_owned(),
        "--token-path".to_string(),
        token_path.to_string_lossy().into_owned(),
        "mcp".to_string(),
    ];
    let name = Path::new(command)
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or(command);
    let mut config = match name {
        "claude" => vec![
            "--mcp-config".into(),
            json!({"mcpServers":{"agmux":{"command":executable,"args":server_args}}}).to_string(),
        ],
        "codex" => vec![
            "-c".into(),
            format!(
                "mcp_servers.agmux.command={}",
                serde_json::to_string(&executable)?
            ),
            "-c".into(),
            format!(
                "mcp_servers.agmux.args={}",
                serde_json::to_string(&server_args)?
            ),
        ],
        _ => {
            bail!("--enable-mcp supports claude and codex; use agmux mcp to configure other agents")
        }
    };
    config.extend(args);
    Ok(config)
}

//! Persistent Gemini ACP conversations, independent of the interactive terminal.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::{Mutex, oneshot},
};

type Pending = HashMap<u64, oneshot::Sender<Result<Value, String>>>;
#[derive(Default)]
struct State {
    messages: Vec<Value>,
    status: String,
    error: Option<String>,
    permissions: Vec<Value>,
    bytes: usize,
    models: Value,
    session: Option<String>,
    requests: Vec<String>,
    model: Option<String>,
}
struct Wire {
    input: Mutex<ChildStdin>,
    child: Mutex<Child>,
    pending: Mutex<Pending>,
    next: AtomicU64,
}
struct Chat {
    state: Arc<Mutex<State>>,
    wire: Mutex<Option<Arc<Wire>>>,
}
static CHATS: OnceLock<Mutex<HashMap<String, Arc<Chat>>>> = OnceLock::new();
fn chats() -> &'static Mutex<HashMap<String, Arc<Chat>>> {
    CHATS.get_or_init(Mutex::default)
}
fn snapshot(s: &State) -> Value {
    json!({"messages":s.messages,"status":s.status,"error":s.error,"permission":s.permissions.first(),"models":s.models,"model":s.model})
}
impl Chat {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                status: "idle".into(),
                ..Default::default()
            })),
            wire: Mutex::new(None),
        }
    }
    async fn connect(&self, cwd: &Path) -> Result<(Arc<Wire>, String)> {
        // Node's project/module resolution expects ordinary Windows paths,
        // including the cwd sent inside the ACP request (not just process cwd).
        let cwd = dunce::simplified(cwd);
        let mut slot = self.wire.lock().await;
        if let Some(w) = slot.as_ref() {
            if let Some(id) = self.state.lock().await.session.clone() {
                return Ok((w.clone(), id));
            }
        }
        let model = self.state.lock().await.model.clone();
        let w = Wire::start(cwd, model.as_deref(), self.state.clone()).await?;
        // Publish the wire before handshaking so Stop can kill startup as well.
        *slot = Some(w.clone());
        drop(slot);
        let result=async {
            w.call("initialize",json!({"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"agmux","version":env!("CARGO_PKG_VERSION")}}),45).await?;
            let session=w.call("session/new",json!({"cwd":cwd,"mcpServers":[]}),45).await?;
            let id=session["sessionId"].as_str().context("Gemini returned no ACP session ID")?.to_owned();
            let mut s=self.state.lock().await;
            s.models=session.get("models").cloned().unwrap_or(Value::Null);s.session=Some(id.clone());
            Ok::<_,anyhow::Error>(id)
        }.await;
        match result {
            Ok(id) => Ok((w, id)),
            Err(e) => {
                w.stop().await;
                Err(e)
            }
        }
    }
}
impl Wire {
    async fn start(cwd: &Path, model: Option<&str>, state: Arc<Mutex<State>>) -> Result<Arc<Self>> {
        let mut command = Command::new("gemini");
        #[cfg(windows)]
        {
            let paths: Vec<_> =
                std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
            if let Some(launch) =
                agmux_core::agent_discovery::windows_gemini_launch("gemini", &paths)?
            {
                command = Command::new(launch.command);
                command.args(launch.args);
            }
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW: ACP has no terminal.
        }
        // Avoid Gemini's memory-tuning wrapper spawning an unowned second Node
        // process. The daemon supervises this process directly.
        command.env("GEMINI_CLI_NO_RELAUNCH", "1");
        #[cfg(unix)]
        command.process_group(0);
        command.arg("--acp");
        if let Some(model) = model {
            command.args(["--model", model]);
        }
        let mut child = command
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("Could not start Gemini CLI. Install it and refresh installed agents.")?;
        let input = child.stdin.take().context("Gemini stdin unavailable")?;
        let output = child.stdout.take().context("Gemini stdout unavailable")?;
        let wire = Arc::new(Self {
            input: Mutex::new(input),
            child: Mutex::new(child),
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
        });
        let reader = wire.clone();
        tokio::spawn(async move {
            let mut output = BufReader::new(output);
            let mut frame = Vec::new();
            loop {
                frame.clear();
                // Bound each provider-controlled frame before allocating more bytes.
                let read = async {
                    loop {
                        let buf = output.fill_buf().await?;
                        if buf.is_empty() {
                            return Ok::<bool, std::io::Error>(false);
                        }
                        let n = buf
                            .iter()
                            .position(|b| *b == b'\n')
                            .map_or(buf.len(), |n| n + 1);
                        if frame.len() + n > 2 * 1024 * 1024 {
                            return Err(std::io::Error::other("ACP frame too large"));
                        }
                        let done = buf[n - 1] == b'\n';
                        frame.extend_from_slice(&buf[..n]);
                        output.consume(n);
                        if done {
                            return Ok(true);
                        }
                    }
                }
                .await;
                if !matches!(read, Ok(true)) {
                    break;
                }
                let Ok(v) = serde_json::from_slice::<Value>(&frame) else {
                    break;
                };
                if let Some(method) = v["method"].as_str() {
                    if method == "session/update" {
                        let update = &v["params"]["update"];
                        let mut s = state.lock().await;
                        if s.session.as_deref() != v["params"]["sessionId"].as_str() {
                            continue;
                        }
                        if update["sessionUpdate"] == "agent_message_chunk" {
                            if let Some(text) = update["content"]["text"].as_str() {
                                if s.bytes + text.len() > 2 * 1024 * 1024 {
                                    s.error = Some(
                                        "Conversation reached its size limit. Start a new session."
                                            .into(),
                                    );
                                    break;
                                }
                                s.bytes += text.len();
                                if s.messages.last().is_none_or(|m| m["role"] != "assistant") {
                                    let id = format!("assistant-{}", s.messages.len());
                                    s.messages
                                        .push(json!({"id":id,"role":"assistant","text":""}));
                                }
                                if let Some(last) = s.messages.last_mut() {
                                    let old = last["text"].as_str().unwrap_or("");
                                    last["text"] = json!(format!("{old}{text}"));
                                }
                            }
                        } else if matches!(
                            update["sessionUpdate"].as_str(),
                            Some("tool_call" | "tool_call_update")
                        ) {
                            let id: String = update["toolCallId"]
                                .as_str()
                                .unwrap_or("tool")
                                .chars()
                                .take(200)
                                .collect();
                            let title: String = update["title"]
                                .as_str()
                                .unwrap_or("Tool operation")
                                .chars()
                                .take(1000)
                                .collect();
                            let item = json!({"id":format!("tool-{id}"),"role":"tool","text":title,"status":update["status"]});
                            if let Some(existing) =
                                s.messages.iter_mut().find(|m| m["id"] == item["id"])
                            {
                                if update["title"].is_string() {
                                    existing["text"] = item["text"].clone();
                                }
                                existing["status"] = item["status"].clone();
                            } else if s.messages.len() < 1000 {
                                s.messages.push(item);
                            }
                        }
                    } else if method == "session/request_permission" {
                        let mut s = state.lock().await;
                        if s.session.as_deref() == v["params"]["sessionId"].as_str() {
                            s.status = "permission".into();
                            if s.permissions.len() >= 32 {
                                s.error = Some("Too many pending provider approvals".into());
                                break;
                            }
                            s.permissions.push(
                                json!({"id":v["id"],"title":v["params"]["toolCall"]["title"],"options":v["params"]["options"]}),
                            );
                        }
                    } else if v.get("id").is_some() {
                        let _=reader.write(json!({"jsonrpc":"2.0","id":v["id"],"error":{"code":-32601,"message":"Client method not supported"}})).await;
                    }
                } else if let Some(id) = v["id"].as_u64() {
                    if let Some(tx) = reader.pending.lock().await.remove(&id) {
                        let result = if v.get("error").is_some() {
                            Err(v["error"]["message"]
                                .as_str()
                                .unwrap_or("Gemini request failed")
                                .to_owned())
                        } else {
                            Ok(v["result"].clone())
                        };
                        let _ = tx.send(result);
                    }
                }
            }
            for (_, tx) in reader.pending.lock().await.drain() {
                let _ = tx.send(Err("Gemini CLI disconnected".into()));
            }
            let mut s = state.lock().await;
            if s.status == "idle" {
                s.status = "failed".into();
                s.error = Some("Gemini CLI disconnected. Start a new session to reconnect.".into());
            }
            // Invalid output or EOF must also reap a still-running provider.
            drop(s);
            reader.stop().await;
        });
        Ok(wire)
    }
    async fn write(&self, value: Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(&value)?;
        bytes.push(b'\n');
        let mut input = self.input.lock().await;
        input.write_all(&bytes).await?;
        input.flush().await?;
        Ok(())
    }
    async fn call(&self, method: &str, params: Value, seconds: u64) -> Result<Value> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let result = async {
            self.write(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
                .await?;
            tokio::time::timeout(Duration::from_secs(seconds), rx)
                .await
                .context("Gemini timed out; the turn was stopped")?
                .context("Gemini connection closed")?
                .map_err(anyhow::Error::msg)
        }
        .await;
        self.pending.lock().await.remove(&id);
        result
    }
    async fn stop(&self) {
        let mut child = self.child.lock().await;
        if let Some(pid) = child.id() {
            #[cfg(unix)]
            let _ = crate::process_supervisor::kill_process_group(pid, libc::SIGKILL);
            #[cfg(windows)]
            {
                let _ = Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .creation_flags(0x08000000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
            }
        }
        let _ = child.kill().await;
    }
}

/// Authenticated daemon seam. Caller resolves actual session/project ownership.
pub async fn request(id: &str, cwd: &Path, operation: &str, payload: &str) -> Result<String> {
    if operation == "close" {
        close(id).await;
        return Ok("{}".into());
    }
    let chat = {
        let mut hub = chats().lock().await;
        ensure!(
            hub.len() < 32 || hub.contains_key(id),
            "Too many open conversations. End an unused session."
        );
        hub.entry(id.into())
            .or_insert_with(|| Arc::new(Chat::new()))
            .clone()
    };
    let value: Value = serde_json::from_str(payload)?;
    match operation {
        "state" => {}
        "configure" => {
            let mut s = chat.state.lock().await;
            ensure!(
                s.messages.is_empty(),
                "Model cannot change during a conversation"
            );
            s.model = value["model"]
                .as_str()
                .filter(|m| !m.is_empty())
                .map(str::to_owned);
        }
        "send" => {
            let text = value["message"]
                .as_str()
                .context("Missing message")?
                .to_owned();
            let request = value["request_id"]
                .as_str()
                .context("Missing request ID")?
                .to_owned();
            ensure!(
                !text.trim().is_empty()
                    && text.len() <= 65536
                    && !request.is_empty()
                    && request.len() <= 128,
                "Invalid message"
            );
            let mut s = chat.state.lock().await;
            if s.requests.contains(&request) {
                return Ok(snapshot(&s).to_string());
            }
            ensure!(
                matches!(s.status.as_str(), "idle" | "cancelled"),
                "Wait for the current turn, or start a new session after a provider error"
            );
            ensure!(
                s.messages.len() < 190 && s.bytes + text.len() <= 2 * 1024 * 1024,
                "Conversation is full. Start a new session."
            );
            s.bytes += text.len();
            s.requests.push(request.clone());
            s.messages
                .push(json!({"id":request,"role":"user","text":text}));
            s.status = "working".into();
            s.error = None;
            drop(s);
            let owned = chat.clone();
            let cwd = cwd.to_owned();
            tokio::spawn(async move {
                let result = async {
                    let (w, id) = owned.connect(&cwd).await?;
                    w.call(
                        "session/prompt",
                        json!({"sessionId":id,"prompt":[{"type":"text","text":text}]}),
                        300,
                    )
                    .await
                }
                .await;
                let mut s = owned.state.lock().await;
                s.permissions.clear();
                match result {
                    Ok(v) => {
                        s.status = if v["stopReason"] == "cancelled" || s.status == "cancelling" {
                            "cancelled"
                        } else {
                            "idle"
                        }
                        .into();
                    }
                    Err(e) => {
                        let cancelled = s.status == "cancelling";
                        s.status = if cancelled { "cancelled" } else { "failed" }.into();
                        if !cancelled {
                            s.error = Some(format!(
                                "{e}. Check Gemini sign-in in Terminal, then start a new session. Your message has not been resent."
                            ));
                        }
                        drop(s);
                        if let Some(w) = owned.wire.lock().await.take() {
                            w.stop().await;
                        }
                    }
                }
            });
        }
        "cancel" => {
            let w = chat.wire.lock().await.clone();
            let mut s = chat.state.lock().await;
            if matches!(s.status.as_str(), "working" | "permission") {
                s.status = "cancelling".into();
                let permissions = std::mem::take(&mut s.permissions);
                let session = s.session.clone();
                drop(s);
                if let Some(w) = &w {
                    for permission in permissions {
                        w.write(json!({"jsonrpc":"2.0","id":permission["id"],"result":{"outcome":{"outcome":"cancelled"}}})).await?;
                    }
                }
                if let (Some(w), Some(id)) = (w, session) {
                    w.write(json!({"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":id}})).await?;
                }
                let owned = chat.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    if owned.state.lock().await.status == "cancelling" {
                        {
                            let mut s = owned.state.lock().await;
                            s.status = "failed".into();
                            s.error=Some("Gemini did not acknowledge Stop. Its process was ended; start a new session.".into());
                        }
                        if let Some(w) = owned.wire.lock().await.take() {
                            w.stop().await;
                        }
                    }
                });
            }
        }
        "permission" => {
            let w = chat
                .wire
                .lock()
                .await
                .clone()
                .context("No active provider")?;
            let mut s = chat.state.lock().await;
            let permission = s.permissions.first().context("No pending permission")?;
            ensure!(
                permission["id"] == value["id"],
                "Permission request is no longer current"
            );
            let option = value["option_id"]
                .as_str()
                .context("Missing permission option")?;
            ensure!(
                permission["options"]
                    .as_array()
                    .is_some_and(|options| options.iter().any(|o| o["optionId"] == option)),
                "Invalid permission option"
            );
            w.write(json!({"jsonrpc":"2.0","id":permission["id"],"result":{"outcome":{"outcome":"selected","optionId":option}}})).await?;
            s.permissions.remove(0);
            s.status = if s.permissions.is_empty() {
                "working"
            } else {
                "permission"
            }
            .into();
        }
        _ => bail!("Unknown chat operation"),
    }
    Ok(snapshot(&*chat.state.lock().await).to_string())
}
/// Reap ACP subprocesses when their owning native session is ended.
pub async fn close(id: &str) {
    if let Some(chat) = chats().lock().await.remove(id) {
        if let Some(w) = chat.wire.lock().await.take() {
            w.stop().await;
        }
    }
}

/// Real ACP inventory. Cached per project; never fabricate account availability.
pub async fn models(cwd: &Path) -> Result<String> {
    type Cache = HashMap<String, (Instant, String)>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    static LIMIT: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    let key = cwd.to_string_lossy().into_owned();
    let cache = CACHE.get_or_init(Mutex::default);
    if let Some((at, data)) = cache.lock().await.get(&key) {
        if at.elapsed() < Duration::from_secs(60) {
            return Ok(data.clone());
        }
    }
    let _permit = LIMIT
        .get_or_init(|| tokio::sync::Semaphore::new(2))
        .try_acquire()
        .context("Model discovery is busy. Retry shortly.")?;
    let chat = Chat::new();
    let result = chat.connect(cwd).await;
    if let Some(w) = chat.wire.lock().await.take() {
        w.stop().await;
    }
    result?;
    let data = chat.state.lock().await.models.to_string();
    let mut cache = cache.lock().await;
    if cache.len() >= 32 {
        cache.clear();
    }
    cache.insert(key, (Instant::now(), data.clone()));
    Ok(data)
}

/// Graceful daemon shutdown must reap conversations as well as PTY sessions.
pub async fn close_all() {
    let ids: Vec<_> = chats().lock().await.keys().cloned().collect();
    for id in ids {
        close(&id).await;
    }
}

/// Lightweight status snapshot for all conversations; transcripts are fetched separately.
pub async fn statuses() -> String {
    let entries: Vec<_> = chats()
        .lock()
        .await
        .iter()
        .map(|(id, chat)| (id.clone(), chat.clone()))
        .collect();
    let mut states = serde_json::Map::new();
    for (id, chat) in entries {
        states.insert(id, json!(chat.state.lock().await.status));
    }
    Value::Object(states).to_string()
}

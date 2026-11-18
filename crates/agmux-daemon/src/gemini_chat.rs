//! Structured Chat and native Gemini PTY with one persisted provider conversation.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
type Db = Arc<std::sync::Mutex<agmux_core::storage_engine::StorageEngine>>;
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
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
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    updated_at: u64,
    messages: Vec<Value>,
    status: String,
    error: Option<String>,
    permissions: Vec<Value>,
    bytes: usize,
    models: Value,
    session: Option<String>,
    requests: Vec<String>,
    model: Option<String>,
    owner: String,
    view: String,
    notice: Option<String>,
}
struct Wire {
    input: Mutex<ChildStdin>,
    child: Mutex<Child>,
    pending: Mutex<Pending>,
    next: AtomicU64,
}
struct Chat {
    owner: Option<(Db, String)>,
    state: Arc<Mutex<State>>,
    wire: Mutex<Option<Arc<Wire>>>,
    native: Mutex<Option<NativeRuntime>>,
    handoff: Mutex<()>,
}
static CHATS: OnceLock<Mutex<HashMap<String, Arc<Chat>>>> = OnceLock::new();
fn chats() -> &'static Mutex<HashMap<String, Arc<Chat>>> {
    CHATS.get_or_init(Mutex::default)
}
fn snapshot(s: &State) -> Value {
    json!({"messages":s.messages,"status":s.status,"error":s.error,"permission":s.permissions.first(),"permissions":s.permissions,"models":s.models,"model":s.model,"provider_session_id":s.session,"updated_at":s.updated_at,"owner":if s.owner.is_empty(){"chat"}else{&s.owner},"view":if s.view.is_empty(){"chat"}else{&s.view},"notice":s.notice})
}
impl Chat {
    fn new() -> Self {
        Self {
            owner: None,
            state: Arc::new(Mutex::new(State {
                status: "idle".into(),
                ..Default::default()
            })),
            wire: Mutex::new(None),
            native: Mutex::new(None),
            handoff: Mutex::new(()),
        }
    }
    async fn save(&self) -> Result<()> {
        let Some((db, id)) = &self.owner else {
            return Ok(());
        };
        let (content, status, updated) = {
            let state = self.state.lock().await;
            (
                serde_json::to_string(&*state)?,
                state.status.clone(),
                state.updated_at,
            )
        };
        let db = db.clone();
        let id = id.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = db.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            db.save_conversation(&id, &content, &status, updated)?;
            if status == "ended" {
                db.update_session_state(&id, "DEAD")?;
            }
            Ok(())
        })
        .await??;
        Ok(())
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
        let (model, saved_session) = {
            let s = self.state.lock().await;
            (s.model.clone(), s.session.clone())
        };
        let w = Wire::start(cwd, model.as_deref(), self.state.clone()).await?;
        if let Some((db, id)) = &self.owner {
            let pid = w.child.lock().await.id().unwrap_or(0);
            db.lock()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?
                .set_session_pid(id, pid)?;
        }
        // Publish the wire before handshaking so Stop can kill startup as well.
        *slot = Some(w.clone());
        drop(slot);
        let result=async {
            let initialized=w.call("initialize",json!({"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"agmux","version":env!("CARGO_PKG_VERSION")}}),45).await?;
            let (session,id)=if let Some(id)=saved_session {
                ensure!(initialized["agentCapabilities"]["loadSession"].as_bool()==Some(true),"This Gemini CLI cannot load a saved conversation. Update Gemini CLI before switching back to Chat; history has not been replaced.");
                let before={ let mut s=self.state.lock().await; s.bytes=0; std::mem::take(&mut s.messages) };
                match w.call("session/load",json!({"sessionId":id,"cwd":cwd,"mcpServers":[]}),45).await {
                    Ok(value)=>(value,id),
                    Err(error)=>{self.state.lock().await.messages=before;return Err(error);}
                }
            } else {
                let value=w.call("session/new",json!({"cwd":cwd,"mcpServers":[]}),45).await?;
                let id=value["sessionId"].as_str().context("Gemini returned no ACP session ID")?.to_owned(); (value,id)
            };
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
                        s.updated_at = now();
                        if matches!(
                            update["sessionUpdate"].as_str(),
                            Some("agent_message_chunk" | "user_message_chunk")
                        ) {
                            let role = if update["sessionUpdate"] == "user_message_chunk" {
                                "user"
                            } else {
                                "assistant"
                            };
                            if let Some(text) = update["content"]["text"].as_str() {
                                if s.bytes + text.len() > 2 * 1024 * 1024 {
                                    s.error = Some(
                                        "Conversation reached its size limit. Start a new session."
                                            .into(),
                                    );
                                    break;
                                }
                                s.updated_at = now();
                                s.bytes += text.len();
                                if s.messages.last().is_none_or(|m| m["role"] != role) {
                                    let id = format!("{role}-{}", s.messages.len());
                                    s.messages.push(json!({"id":id,"role":role,"text":""}));
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
                            let item = json!({"id":format!("tool-{id}"),"role":"tool","text":title,"status":update["status"],"content":update["content"],"tool_call_id":update["toolCallId"]});
                            if let Some(existing) =
                                s.messages.iter_mut().find(|m| m["id"] == item["id"])
                            {
                                if update["title"].is_string() {
                                    existing["text"] = item["text"].clone();
                                }
                                if !item["status"].is_null() {
                                    existing["status"] = item["status"].clone();
                                }
                                if item["content"].is_array() {
                                    existing["content"] = item["content"].clone();
                                }
                            } else if s.messages.len() < 1000 {
                                s.messages.push(item);
                            }
                        }
                    } else if method == "session/request_permission" {
                        let mut s = state.lock().await;
                        if s.session.as_deref() == v["params"]["sessionId"].as_str() {
                            s.updated_at = now();
                            s.status = "permission".into();
                            if s.permissions.len() >= 32 {
                                s.error = Some("Too many pending provider approvals".into());
                                break;
                            }
                            s.permissions.push(
                                json!({"id":v["id"],"title":v["params"]["toolCall"]["title"],"options":v["params"]["options"],"tool_call":v["params"]["toolCall"]}),
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
pub async fn request(
    id: &str,
    cwd: &Path,
    operation: &str,
    payload: &str,
    db: Db,
    ended: bool,
) -> Result<String> {
    if operation == "close" {
        close(id).await;
        return Ok("{}".into());
    }
    let chat = {
        let mut hub = chats().lock().await;
        ensure!(
            ended || hub.len() < 32 || hub.contains_key(id),
            "Too many open conversations. End an unused session."
        );
        if let Some(chat) = hub.get(id) {
            chat.clone()
        } else {
            let saved=db.lock().map_err(|e|anyhow::anyhow!(e.to_string()))?.conversation(id)?.context("This is a native CLI session. Start a managed Gemini session for shared views.")?;
            let mut state: State = serde_json::from_str(&saved)?;
            if ended {
                state.status = "ended".into();
                state.permissions.clear();
            } else if matches!(
                state.status.as_str(),
                "working" | "permission" | "cancelling"
            ) {
                state.status = "failed".into();
                state.permissions.clear();
                state.error = Some(
                    "The previous turn was interrupted. Start a new session to continue safely."
                        .into(),
                );
            }
            let chat = Arc::new(Chat {
                owner: Some((db.clone(), id.into())),
                state: Arc::new(Mutex::new(state)),
                wire: Mutex::new(None),
                native: Mutex::new(None),
                handoff: Mutex::new(()),
            });
            // Retained inactive records do not consume the live-session limit.
            if !ended {
                hub.insert(id.into(), chat.clone());
            }
            chat
        }
    };
    let value: Value = serde_json::from_str(payload)?;
    ensure!(
        !ended || operation == "state",
        "This session has ended; its history is read-only"
    );
    if !ended {
        chat.refresh_native(cwd).await?;
    }
    match operation {
        "state" => {}
        "view" => {
            let view = value["view"].as_str().context("Missing view")?;
            ensure!(matches!(view, "chat" | "terminal"), "Unknown session view");
            chat.state.lock().await.view = view.into();
            chat.apply_view(cwd).await?;
        }
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
            if chat.state.lock().await.owner == "terminal" {
                chat.send_native_prompt(&value).await?;
                chat.save().await?;
                return Ok(snapshot(&*chat.state.lock().await).to_string());
            }
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
            s.updated_at = now();
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
                if s.status == "ended" {
                    return;
                }
                s.updated_at = now();
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
                                "{e}. Check Gemini CLI sign-in on this machine, then start a new session. Your message has not been resent."
                            ));
                        }
                        drop(s);
                        if let Some(w) = owned.wire.lock().await.take() {
                            w.stop().await;
                        }
                        if let Err(e) = owned.save().await {
                            owned.state.lock().await.error =
                                Some(format!("Could not retain conversation: {e}"));
                        }
                        return;
                    }
                }
                drop(s);
                if let Err(e) = owned.save().await {
                    owned.state.lock().await.error =
                        Some(format!("Could not retain conversation: {e}"));
                }
            });
        }
        "cancel" => {
            if let Some(native) = chat.native.lock().await.as_ref() {
                native.session.send_input(&[3])?;
                return Ok(snapshot(&*chat.state.lock().await).to_string());
            }
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
    if !ended && operation == "state" {
        chat.apply_view(cwd).await?;
    }
    if operation != "state" {
        chat.save().await?;
    }
    Ok(snapshot(&*chat.state.lock().await).to_string())
}
/// Reap ACP subprocesses when their owning native session is ended.
pub async fn close(id: &str) {
    if let Some(chat) = chats().lock().await.remove(id) {
        if let Err(e) = chat.refresh_native(std::path::Path::new(".")).await {
            tracing::warn!(%e,"Reading native history on end failed");
        }
        if let Err(e) = chat.stop_native(true).await {
            tracing::error!(%e,"Stopping native Gemini failed");
        }
        {
            let mut s = chat.state.lock().await;
            s.status = "ended".into();
            s.permissions.clear();
            s.updated_at = now();
        }
        if let Some(w) = chat.wire.lock().await.take() {
            w.stop().await;
        }
        if let Err(e) = chat.save().await {
            tracing::error!(%e,"Retaining ended conversation failed");
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
pub async fn statuses(db: Db) -> Result<String> {
    let records = db
        .lock()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .conversation_index()?;
    let mut states = serde_json::Map::new();
    let mut activity = serde_json::Map::new();
    let mut managed = Vec::new();
    for (id, status, at) in records {
        managed.push(id.clone());
        states.insert(id.clone(), json!(status));
        activity.insert(id, json!(at));
    }
    let entries: Vec<_> = chats()
        .lock()
        .await
        .iter()
        .map(|(id, c)| (id.clone(), c.clone()))
        .collect();
    for (id, chat) in entries {
        let state = chat.state.lock().await;
        states.insert(id.clone(), json!(state.status));
        activity.insert(id, json!(state.updated_at));
    }
    Ok(json!({"states":states,"activity":activity,"managed":managed}).to_string())
}

pub fn create(db: &Db, id: &str, model: Option<String>) -> Result<()> {
    let state = State {
        status: "idle".into(),
        model,
        updated_at: now(),
        ..Default::default()
    };
    db.lock()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .save_conversation(
            id,
            &serde_json::to_string(&state)?,
            &state.status,
            state.updated_at,
        )?;
    Ok(())
}

/// End without deleting retained history, including a session never prompted.
pub async fn end(db: Db, id: &str) -> Result<()> {
    close(id).await;
    let mut state: State = {
        let db = db.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        serde_json::from_str(&db.conversation(id)?.context("Conversation not found")?)?
    };
    state.status = "ended".into();
    state.permissions.clear();
    state.updated_at = now();
    let db = db.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    db.save_conversation(
        id,
        &serde_json::to_string(&state)?,
        &state.status,
        state.updated_at,
    )?;
    db.update_session_state(id, "DEAD")?;
    Ok(())
}

struct NativeRuntime {
    session: Arc<agmux_core::pty_manager::ActivePtySession>,
    dir: std::path::PathBuf,
    last_input: u64,
    idle: bool,
}

/// Private process-scoped Gemini lifecycle hook. No daemon startup or IPC token.
pub fn record_native_hook() -> Result<()> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .take(1_000_001)
        .read_to_string(&mut input)?;
    ensure!(input.len() <= 1_000_000, "Gemini hook input too large");
    let v: Value = serde_json::from_str(&input)?;
    let path = std::path::PathBuf::from(
        std::env::var_os("AGMUX_NATIVE_EVENT").context("Missing native event path")?,
    );
    let event = json!({"event":v["hook_event_name"],"session":v["session_id"],"transcript":v["transcript_path"],"at":now()});
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, event.to_string())?;
    std::fs::rename(temporary, path)?;
    println!("{{}}");
    Ok(())
}

fn native_settings(dir: &Path) -> Result<std::path::PathBuf> {
    let system = std::env::var_os("GEMINI_CLI_SYSTEM_SETTINGS_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(windows) {
                r"C:\ProgramData\gemini-cli\settings.json".into()
            } else if cfg!(target_os = "macos") {
                "/Library/Application Support/GeminiCli/settings.json".into()
            } else {
                "/etc/gemini-cli/settings.json".into()
            }
        });
    let defaults = std::env::var_os("GEMINI_CLI_SYSTEM_DEFAULTS_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| system.with_file_name("system-defaults.json"));
    let mut settings: Value = match std::fs::read_to_string(defaults) {
        Ok(text) => jsonc_parser::parse_to_serde_value(&text, &Default::default())
            .context("Could not read Gemini system defaults")?
            .context("Gemini system defaults are empty")?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(e.into()),
    };
    ensure!(settings.is_object(), "Invalid Gemini system defaults");
    if settings.get("hooks").is_none() {
        settings["hooks"] = json!({});
    }
    let command = if cfg!(windows) {
        r#"& "$env:AGMUX_DAEMON_EXE" --gemini-session-hook"#
    } else {
        r#""$AGMUX_DAEMON_EXE" --gemini-session-hook"#
    };
    for event in ["SessionStart", "BeforeAgent", "AfterAgent"] {
        if settings["hooks"].get(event).is_none() {
            settings["hooks"][event] = json!([]);
        }
        settings["hooks"][event].as_array_mut().context("Invalid Gemini lifecycle hooks")?.push(json!({"hooks":[{"name":"agmux-session-sync","type":"command","command":command,"timeout":5000,"env":{"AGMUX_DAEMON_EXE":std::env::current_exe()?.to_string_lossy(),"AGMUX_NATIVE_EVENT":dir.join("event.json").to_string_lossy()}}]}));
    }
    let file = dir.join("settings.json");
    std::fs::write(&file, settings.to_string())?;
    Ok(file)
}

/// Import only actual provider message records, never terminal screens or banners.
fn native_messages(text: &str, id: &str) -> Result<Vec<Value>> {
    let (session, records) = if let Ok(v) = serde_json::from_str::<Value>(text) {
        (
            v["sessionId"].as_str().unwrap_or("").to_owned(),
            v["messages"].as_array().cloned().unwrap_or_default(),
        )
    } else {
        let mut lines = text.lines();
        let metadata: Value =
            serde_json::from_str(lines.next().context("Empty provider transcript")?)?;
        let records = lines
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .collect();
        (
            metadata["sessionId"].as_str().unwrap_or("").to_owned(),
            records,
        )
    };
    ensure!(
        session == id,
        "Gemini transcript belongs to a different conversation; synchronization stopped"
    );
    let mut messages: Vec<Value> = Vec::new();
    for item in records {
        let role = match item["type"].as_str() {
            Some("user") => "user",
            Some("gemini") => "assistant",
            _ => continue,
        };
        let Some(key) = item["id"].as_str() else {
            continue;
        };
        let text = if let Some(text) = item["content"].as_str() {
            text.to_owned()
        } else {
            item["content"]
                .as_array()
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p["text"].as_str())
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default()
        };
        let value = json!({"id":key,"role":role,"text":text});
        if let Some(previous) = messages.iter_mut().find(|m| m["id"] == key) {
            *previous = value;
        } else {
            messages.push(value);
        }
        if let Some(calls) = item["toolCalls"].as_array() {
            for call in calls {
                let Some(id) = call["id"].as_str() else {
                    continue;
                };
                let value = json!({"id":format!("tool-{id}"),"role":"tool","text":call["displayName"].as_str().or_else(||call["name"].as_str()).unwrap_or("Tool"),"status":call["status"],"content":call});
                if let Some(previous) = messages.iter_mut().find(|m| m["id"] == value["id"]) {
                    *previous = value;
                } else {
                    messages.push(value);
                }
            }
        }
    }
    Ok(messages)
}

impl Chat {
    async fn stop_native(&self, force: bool) -> Result<bool> {
        let native = {
            let mut slot = self.native.lock().await;
            // Recheck readiness atomically with detaching input ownership: a
            // received key must not race the earlier view-switch snapshot.
            if !force
                && slot
                    .as_ref()
                    .is_some_and(|n| !n.idle && n.session.subscribe_exit().borrow().is_none())
            {
                return Ok(false);
            }
            slot.take()
        };
        if let Some(native) = native {
            let mut exit = native.session.subscribe_exit();
            let already_exited = exit.borrow().is_some();
            if !already_exited {
                native.session.kill();
            }
            let confirmed = already_exited
                || (matches!(
                    tokio::time::timeout(Duration::from_secs(5), exit.changed()).await,
                    Ok(Ok(()))
                ) && exit.borrow().is_some());
            if !confirmed {
                *self.native.lock().await = Some(native);
                bail!("Native Gemini has not exited. Chat was not started; continue in Terminal.");
            }
            let _ = tokio::fs::remove_dir_all(native.dir).await;
        }
        Ok(true)
    }
    async fn refresh_native(&self, _cwd: &Path) -> Result<()> {
        use std::io::Read;
        let mut slot = self.native.lock().await;
        let Some(native) = slot.as_mut() else {
            return Ok(());
        };
        let path = native.dir.join("event.json");
        let event: Value = match tokio::fs::read_to_string(path).await {
            Ok(text) => serde_json::from_str(&text)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let mut state = self.state.lock().await;
        let previous_at = state.updated_at;
        let previous_status = state.status.clone();
        let mut changed = false;
        let id = state
            .session
            .clone()
            .context("Missing native session identity")?;
        ensure!(
            event["session"].as_str() == Some(id.as_str()),
            "Gemini changed its native session identity. End this Agmux session before opening a different provider session."
        );
        let at = event["at"].as_u64().unwrap_or(0);
        native.idle = matches!(event["event"].as_str(), Some("SessionStart" | "AfterAgent"))
            && at > native.last_input
            && now().saturating_sub(at) >= 250;
        state.status = if native.idle { "idle" } else { "working" }.into();
        state.updated_at = at;
        if let Some(path) = event["transcript"].as_str().filter(|s| !s.is_empty()) {
            let path = std::path::PathBuf::from(path);
            let id = id.to_owned();
            let messages=tokio::task::spawn_blocking(move||->Result<Vec<Value>>{
                let mut text=String::new();std::fs::File::open(path)?.take(2*1024*1024+1).read_to_string(&mut text)?;
                ensure!(text.len()<=2*1024*1024,"Provider transcript exceeds the synchronized preview limit; continue in Terminal");
                native_messages(&text,&id)
            }).await??;
            if state.messages != messages {
                state.messages = messages;
                changed = true;
            }
        }
        state.notice = if native.idle {
            None
        } else {
            Some("Terminal owns this conversation. Finish its current turn or draft there; Chat will synchronize when it is ready.".into())
        };
        changed |= previous_at != state.updated_at || previous_status != state.status;
        drop(state);
        drop(slot);
        if changed {
            self.save().await?;
        }
        Ok(())
    }
    async fn apply_view(&self, cwd: &Path) -> Result<()> {
        if let Err(error) = self.reconcile_view(cwd).await {
            let native = self.native.lock().await.is_some();
            let mut state = self.state.lock().await;
            state.owner = if native { "terminal" } else { "chat" }.into();
            state.view = state.owner.clone();
            state.status = "failed".into();
            let message = format!(
                "Could not switch Gemini interfaces: {error}. The saved conversation was not replaced."
            );
            state.notice = Some(message.clone());
            state.error = Some(message);
            drop(state);
            self.save().await?;
        }
        Ok(())
    }
    async fn reconcile_view(&self, cwd: &Path) -> Result<()> {
        let _guard = self.handoff.lock().await;
        let (view, owner, status) = {
            let s = self.state.lock().await;
            (s.view.clone(), s.owner.clone(), s.status.clone())
        };
        if view == "terminal" && owner != "terminal" {
            if !matches!(status.as_str(), "idle" | "cancelled" | "failed") {
                self.state.lock().await.notice=Some("Terminal will open after the current Chat turn finishes. Resolve any Chat approval first, or stop the turn.".into());
                return Ok(());
            }
            self.state.lock().await.status = "switching".into();
            if let Some(w) = self.wire.lock().await.take() {
                w.stop().await;
            }
            let (id, fresh, model) = {
                let s = self.state.lock().await;
                let fresh = s.session.is_none();
                let id = s
                    .session
                    .clone()
                    .unwrap_or_else(agmux_core::session_tracker::SessionTracker::mint_uuid_v4);
                (id, fresh, s.model.clone())
            };
            let dir = std::env::temp_dir().join(format!(
                "agmux-native-{}",
                agmux_core::session_tracker::SessionTracker::mint_uuid_v4()
            ));
            tokio::fs::create_dir(&dir).await?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
            }
            let settings = native_settings(&dir)?;
            let args = if fresh {
                vec!["--session-id".into(), id.clone()]
            } else {
                vec!["--resume".into(), id.clone()]
            };
            let mut args = args;
            if let Some(model) = model {
                args.extend(["--model".into(), model]);
            }
            let manager = agmux_core::pty_manager::PtyManager::new();
            let session = manager
                .spawn_with_env(
                    self.owner
                        .as_ref()
                        .context("Native session has no owner")?
                        .1
                        .clone(),
                    "gemini".into(),
                    args,
                    Some(dunce::simplified(cwd).to_path_buf()),
                    None,
                    vec![
                        ("GEMINI_CLI_NO_RELAUNCH".into(), "1".into()),
                        (
                            "GEMINI_CLI_SYSTEM_DEFAULTS_PATH".into(),
                            settings.to_string_lossy().into_owned(),
                        ),
                        (
                            "AGMUX_DAEMON_EXE".into(),
                            std::env::current_exe()?.to_string_lossy().into_owned(),
                        ),
                        (
                            "AGMUX_NATIVE_EVENT".into(),
                            dir.join("event.json").to_string_lossy().into_owned(),
                        ),
                    ],
                )
                .await?;
            if let Some((db, id)) = &self.owner {
                db.lock()
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .set_session_pid(id, session.pid)?;
            }
            *self.native.lock().await = Some(NativeRuntime {
                session,
                dir,
                last_input: 0,
                idle: false,
            });
            let mut s = self.state.lock().await;
            s.owner = "terminal".into();
            s.session = Some(id);
            s.error = None;
            s.status = "working".into();
            s.notice =
                Some("Native Gemini is starting. Complete any CLI setup in Terminal.".into());
            drop(s);
            self.save().await?;
        } else if view != "terminal" && owner == "terminal" {
            let (ready, exited) = {
                let native = self.native.lock().await;
                let exited = native
                    .as_ref()
                    .is_some_and(|n| n.session.subscribe_exit().borrow().is_some());
                (exited || native.as_ref().is_some_and(|n| n.idle), exited)
            };
            if !ready {
                return Ok(());
            }
            // An empty native session is not resumable by the native CLI. Keep
            // that single owner until the first prompt has created history.
            if self.state.lock().await.messages.is_empty() && !exited {
                return Ok(());
            }
            self.state.lock().await.status = "switching".into();
            if !self.stop_native(false).await? {
                self.state.lock().await.status = "working".into();
                return Ok(());
            }
            if let Err(e) = self.connect(cwd).await {
                let mut s = self.state.lock().await;
                s.owner = "chat".into();
                s.status = "failed".into();
                s.error = Some(format!(
                    "Could not resume this Gemini session in Chat: {e}. No replacement conversation was created."
                ));
                return Ok(());
            }
            let mut s = self.state.lock().await;
            s.owner = "chat".into();
            s.status = "idle".into();
            s.notice = None;
            drop(s);
            self.save().await?;
        }
        Ok(())
    }
    async fn send_native_prompt(&self, value: &Value) -> Result<()> {
        let text = value["message"].as_str().context("Missing message")?;
        let request = value["request_id"].as_str().context("Missing request ID")?;
        ensure!(
            !text.trim().is_empty()
                && text.len() <= 65536
                && !request.is_empty()
                && request.len() <= 128,
            "Invalid message"
        );
        let mut native = self.native.lock().await;
        let native = native.as_mut().context("Native terminal is unavailable")?;
        let mut state = self.state.lock().await;
        if state.requests.iter().any(|r| r == request) {
            return Ok(());
        }
        ensure!(
            native.idle,
            "Wait for the native Gemini turn or answer its approval in Terminal"
        );
        native.last_input = now();
        native.idle = false;
        native.session.submit_prompt(text)?;
        state.requests.push(request.into());
        state.status = "working".into();
        Ok(())
    }
}

pub async fn native_session(id: &str) -> Option<Arc<agmux_core::pty_manager::ActivePtySession>> {
    let chat = chats().lock().await.get(id)?.clone();
    let native = chat.native.lock().await;
    native.as_ref().map(|n| n.session.clone())
}
pub async fn native_input(id: &str, data: &[u8]) -> Result<bool> {
    let Some(chat) = chats().lock().await.get(id).cloned() else {
        return Ok(false);
    };
    let mut native = chat.native.lock().await;
    let Some(native) = native.as_mut() else {
        return Ok(false);
    };
    // Terminal protocol replies are not a user draft or model submission.
    // Deliver them unchanged without revoking the last confirmed idle boundary.
    if !terminal_report(data) {
        native.last_input = now();
        native.idle = false;
    }
    native.session.send_input(data)?;
    Ok(true)
}

fn terminal_report(data: &[u8]) -> bool {
    if matches!(data, b"\x1b[I" | b"\x1b[O") {
        return true;
    }
    if data.len() > 3
        && data.starts_with(b"\x1b[")
        && matches!(data.last(), Some(b'R' | b'c' | b't' | b'y'))
    {
        return data[2..data.len() - 1]
            .iter()
            .all(|b| b.is_ascii_digit() || b";?=>$:".contains(b));
    }
    if data.starts_with(b"\x1b[?")
        && data.ends_with(b"u")
        && data[3..data.len() - 1]
            .iter()
            .all(|b| b.is_ascii_digit() || *b == b';')
    {
        return true;
    }
    let color = data.starts_with(b"\x1b]10;")
        || data.starts_with(b"\x1b]11;")
        || data.starts_with(b"\x1b]12;");
    color && (data.ends_with(b"\x07") || data.ends_with(b"\x1b\\"))
}

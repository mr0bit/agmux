//! Production browser boundary: authenticated HTTP, SSE and lossless PTY WebSockets.
use agmux_proto::{CompanionRequest, HandshakeRequest, IpcMessage};
use anyhow::{Context, Result, bail};
use axum::{
    Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, OriginalUri, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, Request, StatusCode, header},
    middleware::{self, Next},
    response::{
        Html, IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{convert::Infallible, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Semaphore;
// Generated from the production frontend during compilation.
include!(concat!(env!("OUT_DIR"), "/companion_assets.rs"));

type Ctx = Arc<Bridge>;
struct Bridge {
    socket: PathBuf,
    daemon_token: String,
    secret: String,
    authority: String,
    index: String,
    streams: Arc<Semaphore>,
    directory: Option<PathBuf>,
}
struct ApiError(StatusCode, String);
impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self(StatusCode::FORBIDDEN, error.to_string())
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, axum::Json(json!({"error":self.1}))).into_response()
    }
}
type ApiResult<T> = std::result::Result<T, ApiError>;

/// Serve the embedded application; an explicit directory override is for development.
pub async fn serve(socket: PathBuf, daemon_token: String) -> Result<()> {
    let directory = std::env::var_os("AGMUX_UI_DIST").map(PathBuf::from);
    let port: u16 = std::env::var("AGMUX_GUI_PORT")
        .unwrap_or_else(|_| "0".into())
        .parse()
        .context("Invalid AGMUX_GUI_PORT")?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let authority = listener.local_addr()?.to_string();
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|e| anyhow::anyhow!("Browser token: {e}"))?;
    let secret: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let html = match &directory {
        Some(path) => tokio::fs::read_to_string(path.join("index.html"))
            .await
            .context("AGMUX_UI_DIST must contain the built frontend index.html")?,
        None => std::str::from_utf8(
            embedded_asset("index.html").context("Embedded application index missing")?,
        )?
        .to_owned(),
    };
    let bootstrap =
        format!("<script>const token='{secret}';window.__AGMUX__={{token,version:1}};</script>");
    let ctx = Arc::new(Bridge {
        socket,
        daemon_token,
        secret,
        authority: authority.clone(),
        index: html.replace("<head>", &format!("<head>{bootstrap}")),
        streams: Arc::new(Semaphore::new(32)),
        directory,
    });
    let router = Router::new()
        .route("/", get(index))
        .route("/api/snapshot", get(snapshot))
        .route("/api/info", get(info))
        .route("/api/events", get(events))
        .route("/api/terminal", get(terminal))
        .route("/api/{*path}", post(api))
        .route("/assets/{*path}", get(asset))
        .layer(DefaultBodyLimit::max(1_000_000))
        .layer(middleware::from_fn_with_state(ctx.clone(), authorize))
        .with_state(ctx);
    let url = format!("http://{authority}/");
    println!("{url}");
    if std::env::var_os("AGMUX_GUI_NO_OPEN").is_none() {
        open_browser(&url);
    }
    axum::serve(listener, router)
        .await
        .context("Companion HTTP server")
}
fn embedded_asset(path: &str) -> Option<&'static [u8]> {
    COMPANION_ASSETS
        .iter()
        .find_map(|(name, bytes)| (*name == path).then_some(*bytes))
}

async fn asset(State(ctx): State<Ctx>, OriginalUri(uri): OriginalUri) -> Response {
    let path = uri.path().trim_start_matches('/');
    // Only files in the embedded manifest can be served, including in override mode.
    let Some(embedded) = embedded_asset(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let body = match &ctx.directory {
        Some(directory) => match tokio::fs::read(directory.join(path)).await {
            Ok(bytes) => Bytes::from(bytes),
            Err(_) => return StatusCode::NOT_FOUND.into_response(),
        },
        None => Bytes::from_static(embedded),
    };
    let content_type = if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    };
    ([(header::CONTENT_TYPE, content_type)], body).into_response()
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    if let Err(error) = result {
        tracing::warn!(%error, "Open the printed companion URL in your browser");
    }
}

async fn authorize(
    State(ctx): State<Ctx>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let headers = request.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let path = request.uri().path();
    let query = request.uri().query().unwrap_or("");
    let query_token = query
        .split('&')
        .filter_map(|p| p.split_once('='))
        .find_map(|(key, value)| (key == "token").then_some(value))
        .unwrap_or("");
    let token = headers
        .get("x-agmux-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let api = path.starts_with("/api/");
    let authenticated = agmux_proto::constant_time_eq(token, &ctx.secret)
        || (matches!(path, "/api/events" | "/api/terminal")
            && agmux_proto::constant_time_eq(query_token, &ctx.secret));
    let same_origin = origin == Some(format!("http://{}", ctx.authority).as_str());
    let write = request.method() != axum::http::Method::GET || path == "/api/terminal";
    if host != ctx.authority || (api && (!authenticated || write && !same_origin)) {
        return ApiError(
            StatusCode::FORBIDDEN,
            "Invalid companion authentication or origin".into(),
        )
        .into_response();
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().expect("static header"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "no-store".parse().expect("static header"),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        "no-referrer".parse().expect("static header"),
    );
    response.headers_mut().insert(
        header::X_FRAME_OPTIONS,
        "DENY".parse().expect("static header"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        "frame-ancestors 'none'; base-uri 'self'; object-src 'none'"
            .parse()
            .expect("static header"),
    );
    response
}
async fn index(State(ctx): State<Ctx>) -> Html<String> {
    Html(ctx.index.clone())
}
async fn rpc(ctx: &Bridge, request: IpcMessage) -> Result<IpcMessage> {
    tokio::time::timeout(
        Duration::from_secs(10),
        crate::ipc_client::review_request(&ctx.socket, &ctx.daemon_token, request),
    )
    .await
    .context("Daemon request timed out")?
}
async fn companion(
    ctx: &Bridge,
    request: CompanionRequest,
) -> Result<agmux_proto::CompanionResponse> {
    match rpc(ctx, IpcMessage::CompanionRequest(request)).await? {
        IpcMessage::CompanionResponse(response) => {
            if let Some(error) = &response.error {
                bail!("{error}");
            }
            Ok(response)
        }
        _ => bail!("Unexpected companion response"),
    }
}
#[derive(Default, Deserialize)]
struct Selection {
    session_id: Option<String>,
}
async fn current_snapshot(ctx: &Bridge, selected: Option<&str>) -> Result<Value> {
    let panels = companion(ctx, CompanionRequest::List).await?;
    let workspace = match rpc(
        ctx,
        IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::List),
    )
    .await?
    {
        IpcMessage::WorkspaceResponse(value) => {
            if let Some(error) = &value.error {
                bail!("{error}");
            }
            value
        }
        _ => bail!("Unexpected workspace response"),
    };
    let attention = workspace
        .sessions
        .iter()
        .filter(|session| {
            matches!(
                session.state.as_str(),
                "WAITING" | "WAITING_INPUT" | "FAILED" | "ERRORED" | "STUCK"
            )
        })
        .count();
    let mut value = json!({"sessions":workspace.sessions,"metadata":workspace.metadata,"panels":panels.panels,"notifications":panels.notifications,"selected_panel":panels.selected_panel,"selected":panels.selected,"attention":attention,"screen":""});
    let id = selected
        .map(str::to_owned)
        .or_else(|| value["selected"].as_str().map(str::to_owned))
        .or_else(|| {
            value["sessions"].as_array()?.first()?["session_id"]
                .as_str()
                .map(str::to_owned)
        });
    if let Some(id) = id {
        if !value["sessions"].as_array().is_some_and(|rows| {
            rows.iter()
                .any(|row| row["session_id"].as_str() == Some(&id))
        }) {
            bail!("Session not found");
        }
        if let IpcMessage::ReadSessionResponse(screen) = rpc(
            ctx,
            IpcMessage::ReadSessionRequest(agmux_proto::ReadSessionRequest {
                session_id: id.clone(),
            }),
        )
        .await?
        {
            if screen.success {
                let mut history = agmux_core::fts_indexer::TerminalHistory::new(1_000_000);
                history.ingest(screen.screen.as_bytes());
                value["screen"] = String::from_utf8_lossy(&history.clean())
                    .into_owned()
                    .into();
            } else {
                value["screen"] = Value::String(String::new());
            }
        }
        value["selected"] = id.into();
    }
    Ok(value)
}
async fn snapshot(
    State(ctx): State<Ctx>,
    Query(query): Query<Selection>,
) -> ApiResult<axum::Json<Value>> {
    Ok(axum::Json(
        current_snapshot(&ctx, query.session_id.as_deref()).await?,
    ))
}
async fn info() -> ApiResult<axum::Json<Value>> {
    let info = tokio::task::spawn_blocking(machine_info)
        .await
        .context("Agent discovery task failed")?;
    Ok(axum::Json(info))
}
fn machine_info() -> Value {
    let dirs: Vec<_> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    let plugins = agmux_core::builtin_plugins::all_builtin_plugins();
    let discovered =
        agmux_core::agent_discovery::discover_all_builtin_agents_in_paths(&dirs, cfg!(windows));
    let mut agents: Vec<Value> = plugins.iter().zip(discovered).map(|(p,d)| json!({"id":p.id(),"name":d.name,"command":d.binary_name,"available":d.available})).collect();
    agents.insert(
        0,
        json!({"id":"shell","name":"Shell","command":shell(),"available":true}),
    );
    json!({"version":env!("CARGO_PKG_VERSION"),"machine":std::env::var("HOSTNAME").unwrap_or_else(|_|"Local machine".into()),"cwd":std::env::current_dir().ok(),"agents":agents,"capabilities":{"terminal":true,"files":true,"diff":true,"structured_chat":false,"permission_decisions":false,"agent_handoff":false}})
}
fn shell() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }
}
fn field(value: &Value, key: &str) -> Result<String> {
    Ok(value[key]
        .as_str()
        .with_context(|| format!("Missing {key}"))?
        .to_owned())
}
fn number(value: &Value, key: &str) -> Result<i64> {
    value[key]
        .as_i64()
        .with_context(|| format!("Missing {key}"))
}
fn fingerprint(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}
async fn session_root(ctx: &Bridge, id: &str) -> Result<PathBuf> {
    if id.starts_with('[') {
        bail!("Remote file access is not available through this local companion");
    }
    let sessions = match rpc(
        ctx,
        IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::List),
    )
    .await?
    {
        IpcMessage::WorkspaceResponse(response) if response.error.is_none() => response.sessions,
        _ => bail!("Cannot load workspace sessions"),
    };
    let session = sessions
        .into_iter()
        .find(|s| s.session_id == id)
        .context("Session not found")?;
    tokio::fs::canonicalize(session.cwd)
        .await
        .context("Session directory is unavailable")
}
async fn api(
    State(ctx): State<Ctx>,
    OriginalUri(uri): OriginalUri,
    body: Bytes,
) -> ApiResult<axum::Json<Value>> {
    let value: Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let result: Result<Value> = async { Ok(match uri.path() {
        "/api/select" => serde_json::to_value(companion(&ctx,CompanionRequest::Select{session_id:field(&value,"session_id")?}).await?)?,
        "/api/reply" => { let id=field(&value,"session_id")?; let message=field(&value,"message")?; if message.is_empty() || message.len()>65536 { bail!("Message must be between 1 and 65536 bytes"); } serde_json::to_value(companion(&ctx,CompanionRequest::Reply{session_id:id,message}).await?)? },
        "/api/send" => { match rpc(&ctx,IpcMessage::SendMessageRequest(agmux_proto::SendMessageRequest{target_session_id:field(&value,"session_id")?,message:field(&value,"message")?})).await? { IpcMessage::SendMessageResponse(r) => { if !r.success { bail!("{}",r.error.unwrap_or_default()); } serde_json::to_value(r)? }, _=>bail!("Unexpected send response") } },
        "/api/spawn" => {
            let agent=field(&value,"agent")?; let cwd=tokio::fs::canonicalize(field(&value,"cwd")?).await.context("Project directory does not exist")?; if !cwd.is_dir() { bail!("Project must be a directory"); }
            let extra: Vec<String>=serde_json::from_value(value.get("args").cloned().unwrap_or_else(||json!([])))?;
            if extra.len()>128 || extra.iter().any(|a|a.len()>65536 || a.contains('\0')) { bail!("Invalid command arguments"); }
            let (command,args)=if agent=="shell" {(shell(),extra)} else if agent=="custom" {(field(&value,"command")?,extra)} else { let plugin=agmux_core::builtin_plugins::all_builtin_plugins().into_iter().find(|p|p.id()==agent).context("Unknown agent")?; (plugin.binary_name().to_owned(),plugin.spawn_args(None,&extra)) };
            if command.is_empty() || command.contains('\0') { bail!("Invalid executable"); }
            let mut random=[0u8;16]; getrandom::fill(&mut random).map_err(|e|anyhow::anyhow!("Session ID: {e}"))?; let id=format!("ui-{}",random.iter().map(|b|format!("{b:02x}")).collect::<String>());
            let req=agmux_proto::SpawnSessionRequest{session_id:id.clone(),command,args,cwd:Some(cwd.to_string_lossy().into_owned()),detached:true,cols:Some(100),rows:Some(30),worktree_branch:None};
            match rpc(&ctx,IpcMessage::SpawnSessionRequest(req)).await? { IpcMessage::SpawnSessionResponse(r) if r.success=>{}, IpcMessage::SpawnSessionResponse(r)=>bail!("{}",r.error.unwrap_or_default()), _=>bail!("Unexpected spawn response") }
            let title=value["title"].as_str().filter(|s|!s.trim().is_empty()).unwrap_or(&agent).chars().take(120).collect();
            match rpc(&ctx,IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::Set{session_id:id.clone(),metadata:agmux_proto::WorkspaceMetadata{project:title,..Default::default()}})).await? { IpcMessage::WorkspaceResponse(r) if r.error.is_none()=>{}, _=>bail!("Session started but metadata could not be saved") }
            companion(&ctx,CompanionRequest::Select{session_id:id.clone()}).await?; json!({"session_id":id})
        },
        "/api/interrupt" => { let id=field(&value,"session_id")?; let mut stream=authenticated(&ctx).await?; stream.send(IpcMessage::TerminalInputFrame(agmux_proto::TerminalInputFrame{session_id:id,data:vec![3]})).await?; json!({"success":true}) },
        "/api/freeze" => { match rpc(&ctx,IpcMessage::FreezeSessionRequest(agmux_proto::FreezeSessionRequest{session_id:field(&value,"session_id")?})).await? { IpcMessage::FreezeSessionResponse(r) if r.success=>json!({"success":true}), _=>bail!("Session could not be ended") } },
        "/api/workspace" => { let metadata=serde_json::from_value(value["metadata"].clone())?; match rpc(&ctx,IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::Set{session_id:field(&value,"session_id")?,metadata})).await? { IpcMessage::WorkspaceResponse(r) if r.error.is_none()=>json!({"success":true}), _=>bail!("Metadata could not be saved") } },
        "/api/panels/open" => {
            let id=field(&value,"session_id")?; let path=field(&value,"path")?;
            let target=if path.starts_with("http://") || path.starts_with("https://") {path.clone()} else {tokio::fs::canonicalize(session_root(&ctx,&id).await?.join(&path)).await?.to_string_lossy().into_owned()};
            let response=companion(&ctx,CompanionRequest::Open{session_id:id.clone(),target:path}).await?;
            let panel=response.panels.iter().find(|panel|panel.session_id==id && panel.target==target).context("Registered panel not returned")?;
            json!({"panel_id":panel.id,"panels":response.panels})
        },
        "/api/file/read" => { let r=companion(&ctx,CompanionRequest::Read{panel_id:number(&value,"panel_id")?}).await?; let content=r.content.unwrap_or_default(); json!({"version":fingerprint(&content),"content":content}) },
        "/api/file/write" => {
            let panel_id=number(&value,"panel_id")?; let content=field(&value,"content")?;
            if let Some(expected)=value["version"].as_str() { let current=companion(&ctx,CompanionRequest::Read{panel_id}).await?.content.unwrap_or_default(); if fingerprint(&current)!=expected { bail!("File changed on disk. Reload before saving to avoid overwriting changes."); } }
            companion(&ctx,CompanionRequest::Write{panel_id,content:content.clone()}).await?; json!({"success":true,"version":fingerprint(&content)})
        },
        "/api/files" => {
            let root=session_root(&ctx,&field(&value,"session_id")?).await?; let relative=value["path"].as_str().unwrap_or(""); let path=tokio::fs::canonicalize(root.join(relative)).await?;
            if !path.starts_with(&root) { bail!("Directory is outside the session workspace"); }
            let mut reader=tokio::fs::read_dir(path).await?; let mut entries=Vec::new();
            while let Some(entry)=reader.next_entry().await? { let name=entry.file_name().to_string_lossy().into_owned(); if matches!(name.as_str(),".git"|"node_modules"|"target") { continue; } let resolved=match tokio::fs::canonicalize(entry.path()).await {Ok(p) if p.starts_with(&root)=>p,_=>continue}; let metadata=tokio::fs::metadata(&resolved).await?; entries.push(json!({"name":name,"path":resolved.strip_prefix(&root)?.to_string_lossy(),"is_dir":metadata.is_dir()})); if entries.len()>=512 {break;} }
            entries.sort_by_key(|e|(!e["is_dir"].as_bool().unwrap_or(false),e["name"].as_str().unwrap_or("").to_lowercase())); json!({"entries":entries})
        },
        "/api/diff" => { let scope=serde_json::from_value(value.get("scope").cloned().unwrap_or_else(||json!("FullUncommitted")))?; match rpc(&ctx,IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest{session_id:field(&value,"session_id")?,scope,target:None})).await? {IpcMessage::GetDiffResponse(r)=>{if let Some(e)=r.error {bail!("{e}");} json!({"files":r.files})}, _=>bail!("Unexpected diff response")} },
        _=>bail!("Unknown endpoint"),
    }) }.await;
    match result {
        Err(error) if error.to_string().starts_with("File changed on disk") => {
            Err(ApiError(StatusCode::CONFLICT, error.to_string()))
        }
        value => Ok(axum::Json(value?)),
    }
}
async fn authenticated(
    ctx: &Bridge,
) -> Result<agmux_proto::FramedStream<agmux_proto::IpcClientStream>> {
    let mut stream = agmux_proto::new_framed(agmux_proto::connect_ipc(&ctx.socket).await?);
    stream
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: ctx.daemon_token.clone(),
        }))
        .await?;
    match tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await?
        .context("Daemon closed handshake")??
    {
        IpcMessage::HandshakeResponse(r) if r.success => Ok(stream),
        _ => bail!("Daemon authentication failed"),
    }
}
async fn events(
    State(ctx): State<Ctx>,
    Query(query): Query<Selection>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let permit = ctx.streams.clone().try_acquire_owned().map_err(|_| {
        ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many live streams".into(),
        )
    })?;
    let initial = current_snapshot(&ctx, query.session_id.as_deref()).await?;
    let mut journal = authenticated(&ctx).await?;
    let since = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    journal
        .send(IpcMessage::WatchEventsRequest(
            agmux_proto::WatchEventsRequest {
                cursor_id: None,
                since_seq: Some(since),
                follow: true,
            },
        ))
        .await
        .map_err(anyhow::Error::from)?;
    let mut viewport = authenticated(&ctx).await?;
    if let Some(id) = initial["selected"].as_str() {
        viewport
            .send(IpcMessage::SubscribeViewport {
                session_id: id.into(),
            })
            .await
            .map_err(anyhow::Error::from)?;
    }
    let stream = async_stream::stream! {
        let _permit=permit; let mut previous=initial.to_string();
        yield Ok::<Event,Infallible>(Event::default().event("snapshot").data(previous.clone()));
        let mut tick=tokio::time::interval(Duration::from_millis(20)); tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip); let mut dirty=false;
        loop {
            tokio::select! {
                frame=journal.next()=>match frame {Some(Ok(IpcMessage::WatchEventFrame(frame)))=>{if let Some(event)=frame.event {dirty=true; yield Ok(Event::default().event("journal").id(event.seq.to_string()).data(serde_json::to_string(&event).unwrap_or_default()));}},Some(Ok(_))=>{dirty=true;},_=>{yield Ok(Event::default().event("offline").data("Daemon disconnected"));break;}},
                frame=viewport.next()=>match frame {Some(Ok(_))=>dirty=true,_=>{yield Ok(Event::default().event("offline").data("Viewport disconnected"));break;}},
                _=tick.tick(), if dirty=>{dirty=false; match current_snapshot(&ctx,query.session_id.as_deref()).await {Ok(value)=>{let current=value.to_string();if current!=previous {previous=current.clone();yield Ok(Event::default().event("update").data(current));}},Err(error)=>{yield Ok(Event::default().event("offline").data(error.to_string()));break;}}},
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::default().interval(Duration::from_secs(10))))
}
#[derive(Deserialize)]
struct TerminalQuery {
    session_id: String,
}
async fn terminal(
    State(ctx): State<Ctx>,
    Query(query): Query<TerminalQuery>,
    upgrade: WebSocketUpgrade,
) -> ApiResult<Response> {
    let permit = ctx.streams.clone().try_acquire_owned().map_err(|_| {
        ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many live streams".into(),
        )
    })?;
    let mut ipc = authenticated(&ctx).await?;
    ipc.send(IpcMessage::SubscribeViewport {
        session_id: query.session_id.clone(),
    })
    .await
    .map_err(anyhow::Error::from)?;
    Ok(upgrade
        .max_message_size(65536)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            if let Err(error) = terminal_socket(socket, ipc, query.session_id).await {
                tracing::debug!("Terminal disconnected: {error}");
            }
        }))
}
async fn terminal_socket(
    mut socket: WebSocket,
    mut ipc: agmux_proto::FramedStream<agmux_proto::IpcClientStream>,
    id: String,
) -> Result<()> {
    loop {
        tokio::select! {
            frame=ipc.next()=>{let frame=frame.context("Daemon closed terminal")??;let data=match frame {IpcMessage::ScreenSyncFrame(frame) if frame.session_id==id=>{socket.send(Message::Text(json!({"type":"sync","cols":frame.cols,"rows":frame.rows}).to_string().into())).await?;let mut data=b"\x1b[2J\x1b[H".to_vec();data.extend(frame.formatted_screen);Some(data)},IpcMessage::TerminalOutputFrame(frame) if frame.session_id==id=>Some(frame.data),IpcMessage::SessionExitEvent(event) if event.session_id==id=>{socket.send(Message::Text(json!({"type":"exit","code":event.exit_code}).to_string().into())).await?;break;},_=>None};if let Some(data)=data {tokio::time::timeout(Duration::from_secs(10),socket.send(Message::Binary(data.into()))).await??;}},
            frame=socket.recv()=>match frame {Some(Ok(Message::Binary(data)))=>ipc.send(IpcMessage::TerminalInputFrame(agmux_proto::TerminalInputFrame{session_id:id.clone(),data:data.to_vec()})).await?,Some(Ok(Message::Text(text)))=>{let value:Value=serde_json::from_str(&text)?;if value["type"]=="resize" {let cols=value["cols"].as_u64().filter(|n| (2..=500).contains(n)).context("Invalid terminal columns")? as u16;let rows=value["rows"].as_u64().filter(|n|(2..=200).contains(n)).context("Invalid terminal rows")? as u16;ipc.send(IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest{session_id:id.clone(),cols,rows})).await?;}},Some(Ok(Message::Close(_)))|None=>break,Some(Err(error))=>return Err(error.into()),_=>{}},
        }
    }
    Ok(())
}

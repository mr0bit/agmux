//! Local companion web bridge over authenticated daemon IPC.
use agmux_proto::{CompanionRequest, CompanionResponse, IpcMessage};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn request(
    socket: &Path,
    token: &str,
    req: CompanionRequest,
) -> Result<CompanionResponse> {
    match crate::ipc_client::review_request(socket, token, IpcMessage::CompanionRequest(req))
        .await?
    {
        IpcMessage::CompanionResponse(r) => {
            if let Some(e) = &r.error {
                bail!("{e}");
            }
            Ok(r)
        }
        other => bail!("Unexpected companion response: {other:?}"),
    }
}

pub async fn snapshot(socket: &Path, token: &str) -> Result<Value> {
    let panels = request(socket, token, CompanionRequest::List).await?;
    let workspace = crate::ipc_client::review_request(
        socket,
        token,
        IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::List),
    )
    .await?;
    let IpcMessage::WorkspaceResponse(workspace) = workspace else {
        bail!("Unexpected workspace response");
    };
    let attention = workspace
        .sessions
        .iter()
        .filter(|s| matches!(s.state.as_str(), "WAITING_INPUT" | "FAILED" | "STUCK"))
        .count();
    let screen = if let Some(id) = &panels.selected {
        match crate::ipc_client::review_request(
            socket,
            token,
            IpcMessage::ReadSessionRequest(agmux_proto::ReadSessionRequest {
                session_id: id.clone(),
            }),
        )
        .await
        {
            Ok(IpcMessage::ReadSessionResponse(r)) => {
                let mut history = agmux_core::fts_indexer::TerminalHistory::new(1_000_000);
                history.ingest(r.screen.as_bytes());
                String::from_utf8_lossy(&history.clean()).into_owned()
            }
            _ => String::new(),
        }
    } else {
        String::new()
    };
    Ok(
        json!({"sessions":workspace.sessions,"metadata":workspace.metadata,"panels":panels.panels,"notifications":panels.notifications,"selected_panel":panels.selected_panel,"selected":panels.selected,"attention":attention,"screen":screen}),
    )
}

/// Resolve strictly structured session and panel links through the daemon registry.
pub async fn open_link(socket: &Path, token: &str, uri: &str) -> Result<()> {
    let rest = uri
        .strip_prefix("agmux://session/")
        .context("Invalid agmux link")?;
    let parts: Vec<_> = rest.split('/').collect();
    if parts.is_empty()
        || parts[0].is_empty()
        || !parts[0]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
    {
        bail!("Invalid session ID");
    }
    match parts.as_slice() {
        [id] => {
            request(
                socket,
                token,
                CompanionRequest::Select {
                    session_id: (*id).into(),
                },
            )
            .await?;
        }
        [id, "panel", panel] => {
            let panel: i64 = panel.parse()?;
            request(
                socket,
                token,
                CompanionRequest::SelectPanel {
                    session_id: (*id).into(),
                    panel_id: panel,
                },
            )
            .await?;
        }
        _ => bail!("Invalid agmux link"),
    }
    Ok(())
}

/// Create a desktop entry for the `agmux://` URI scheme.
#[cfg(target_os = "linux")]
pub fn register_desktop_protocol(app_path: Option<&Path>) -> Result<PathBuf> {
    let executable = match app_path {
        Some(path) => path.to_path_buf(),
        None => std::env::current_exe()?,
    };
    let executable = executable
        .canonicalize()
        .context("Invalid executable path")?;
    let executable = executable
        .to_str()
        .context("Executable path is not UTF-8")?;
    let escaped = executable
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('$', "\\$")
        .replace('`', "\\`");
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    let dir = PathBuf::from(home).join(".local/share/applications");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("agmux-scheme-handler.desktop");
    std::fs::write(
        &path,
        format!(
            "[Desktop Entry]\nType=Application\nName=Agmux\nExec=\"{escaped}\" open %u\nStartupNotify=false\nTerminal=false\nMimeType=x-scheme-handler/agmux;\n"
        ),
    )?;
    let status = std::process::Command::new("xdg-mime")
        .args([
            "default",
            "agmux-scheme-handler.desktop",
            "x-scheme-handler/agmux",
        ])
        .status()
        .context("Failed to run xdg-mime")?;
    if !status.success() {
        bail!("xdg-mime failed: {status}");
    }
    Ok(path)
}

/// Registering a desktop URI handler is currently supported on Linux.
#[cfg(not(target_os = "linux"))]
pub fn register_desktop_protocol(_app_path: Option<&Path>) -> Result<PathBuf> {
    bail!("Desktop protocol registration is only supported on Linux")
}

const PAGE: &str = r##"<!doctype html><html><head><meta charset="utf-8"><title>agmux Desktop Mode — Companion GUI</title><style>
:root{--bg:#0b0f19;--panel:#131b2e;--card:#1a233a;--border:#22304d;--accent:#38bdf8;--text:#e2e8f0;--dim:#8492a6;--red:#ef4444;--green:#22c55e}
*{box-sizing:border-box;margin:0;padding:0}body{font:14px/1.4 -apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:var(--bg);color:var(--text);display:flex;flex-direction:column;height:100vh;overflow:hidden}
header{background:var(--panel);border-bottom:1px solid var(--border);padding:10px 18px;display:flex;align-items:center;justify-content:space-between;user-select:none}
.win-ctrl{display:flex;gap:7px;align-items:center}.dot{width:11px;height:11px;border-radius:50%;display:inline-block}.d-r{background:#ff5f56}.d-y{background:#ffbd2e}.d-g{background:#27c93f}
.title-wrap{margin-left:12px;font-weight:600;font-size:14px;display:flex;align-items:center;gap:8px}
.title-wrap strong{background:linear-gradient(135deg,#38bdf8,#818cf8);-webkit-background-clip:text;-webkit-text-fill-color:transparent;font-weight:800;font-size:15px}
.hdr-right{display:flex;align-items:center;gap:12px}
.badge-sse{background:rgba(34,197,94,0.15);color:#86efac;border:1px solid rgba(34,197,94,0.3);padding:3px 10px;border-radius:20px;font-size:11px;font-weight:600;display:flex;align-items:center;gap:6px}
.pulse{width:6px;height:6px;background:var(--green);border-radius:50%;box-shadow:0 0 6px var(--green)}
.badge{background:var(--red);color:white;border-radius:12px;padding:2px 9px;font-size:11px;font-weight:700}
main{display:grid;grid-template-columns:270px 1fr 1fr;gap:0;flex:1;height:calc(100vh - 45px);overflow:hidden}
.col{display:flex;flex-direction:column;border-right:1px solid var(--border);overflow:hidden}
.col:last-child{border-right:none}
.col-hdr{background:var(--panel);border-bottom:1px solid var(--border);padding:9px 14px;font-size:11px;text-transform:uppercase;letter-spacing:1px;font-weight:700;color:var(--dim);display:flex;justify-content:space-between}
#sessions{padding:10px;display:flex;flex-direction:column;gap:6px;overflow-y:auto;flex:1}
#sessions button{background:var(--card);border:1px solid var(--border);color:var(--text);padding:8px 12px;border-radius:6px;cursor:pointer;text-align:left;font-size:12px;font-weight:600;display:block;width:100%}
#sessions button:hover{border-color:var(--accent)}
#screen{background:#06090e;color:#cbd5e1;padding:14px;font:13px/1.5 'DejaVu Sans Mono',Consolas,monospace;white-space:pre-wrap;flex:1;overflow-y:auto;border:none}
#reply{background:var(--panel);border-top:1px solid var(--border);padding:10px 14px;display:flex;gap:8px}
#text{flex:1;background:#06090e;border:1px solid var(--border);border-radius:5px;padding:8px 12px;color:white;font-size:13px}
#reply button,button.save-btn{background:#2563eb;color:white;border:none;padding:7px 14px;border-radius:5px;cursor:pointer;font-weight:600;font-size:12px}
#panels{padding:10px;overflow-y:auto;flex:1}
.panel{background:var(--card);border:1px solid var(--border);border-radius:6px;padding:12px;margin-bottom:12px}
.panel strong{display:block;font-size:12px;color:var(--accent);margin-bottom:8px}
.editor{display:flex;background:#06090e;border:1px solid var(--border);border-radius:4px;overflow:hidden}
.lines{width:3.2em;text-align:right;color:#475569;padding:10px 6px;font:12px/1.4 monospace;white-space:pre;overflow:hidden;background:#04060a;border-right:1px solid var(--border);user-select:none}
.code{position:relative;flex:1;min-width:0}
.code pre,.editor textarea{box-sizing:border-box;font:12px/1.4 'DejaVu Sans Mono',Consolas,monospace;white-space:pre;overflow:auto;tab-size:4;margin:0;width:100%;height:220px;min-height:0;padding:10px}
.code pre{position:absolute;inset:0;pointer-events:none;color:#cbd5e1}
.editor textarea{position:relative;background:transparent;color:transparent;caret-color:white;resize:none}
.kw{color:#c084fc;font-weight:600}.str{color:#86efac}.comment{color:#64748b;font-style:italic}
iframe{width:100%;height:320px;background:white;border:0;border-radius:4px}
#notifications{max-height:160px;overflow-y:auto;padding:8px 12px;background:rgba(0,0,0,0.2);border-top:1px solid var(--border)}
#notifications p{font-size:12px;padding:5px 8px;margin-bottom:4px;background:var(--card);border-left:3px solid #f59e0b;border-radius:3px}
#status{padding:4px 12px;font-size:11px;color:var(--dim);background:var(--panel)}
</style></head><body>
<header>
  <div class="win-ctrl"><span class="dot d-r"></span><span class="dot d-y"></span><span class="dot d-g"></span><div class="title-wrap"><strong>agmux</strong> · Desktop Mode (Companion GUI)</div></div>
  <div class="hdr-right"><div class="badge-sse"><span class="pulse"></span><span>Live SSE (<15ms)</span></div><div>Attention <span id="badge" class="badge">0</span></div></div>
</header>
<main>
  <section class="col"><div class="col-hdr"><span>Sessions</span><span>Workspace</span></div><div id="sessions"></div><div class="col-hdr"><span>Notifications</span></div><div id="notifications"></div></section>
  <section class="col"><div class="col-hdr"><span>Live Terminal Viewport</span><span>SSE Stream</span></div><pre id="screen"></pre><form id="reply"><input id="text" placeholder="Reply or enter command for session..."><button>Send reply</button></form></section>
  <section class="col"><div class="col-hdr"><span>Docked Panels (Monaco / Web)</span><span id="status"></span></div><div id="panels"></div></section>
</main>
<script>
const token='__TOKEN__'; async function api(path,body){let r=await fetch(path,{method:body===undefined?'GET':'POST',headers:{'Content-Type':'application/json','X-Agmux-Token':token},body:body===undefined?undefined:JSON.stringify(body)});let j=await r.json();if(!r.ok)throw Error(j.error);return j} let state, signature='';
function render(s){try{state=s;document.querySelector('#badge').textContent=s.attention;document.querySelector('#screen').textContent=s.screen;let notices=document.querySelector('#notifications');notices.replaceChildren();for(let n of s.notifications.slice(0,10)){let p=document.createElement('p');p.textContent=n.title+' · '+n.body;notices.append(p)}let list=document.querySelector('#sessions');list.replaceChildren();for(let x of s.sessions.filter(x=>x.state!=='DEAD'&&x.state!=='COMPLETED').slice(0,8)){let b=document.createElement('button');b.textContent=(s.metadata[x.session_id]?.project||x.session_id)+' · '+x.state;if(x.session_id===s.selected){b.style.borderColor='var(--accent)';b.style.background='rgba(56,189,248,0.12)'}b.onclick=async()=>{await api('/api/select',{session_id:x.session_id});signature='';refresh()};list.append(b)}let sig=JSON.stringify([s.selected,s.selected_panel,s.panels]);if(sig!==signature){signature=sig;let host=document.querySelector('#panels');host.replaceChildren();for(let p of s.panels.filter(p=>p.session_id===s.selected)){let box=document.createElement('div');box.className='panel';if(p.id===s.selected_panel)box.style.outline='2px solid #38bdf8';let title=document.createElement('strong');title.textContent=p.target;box.append(title);if(p.kind==='web'){let f=document.createElement('iframe');f.setAttribute('sandbox','allow-scripts allow-forms');f.src=p.target;box.append(f)}else{let editor=document.createElement('div');editor.className='editor';let lines=document.createElement('div');lines.className='lines';let code=document.createElement('div');code.className='code';let highlight=document.createElement('pre');let area=document.createElement('textarea');area.spellcheck=false;let numbers=()=>{lines.textContent=area.value.split('\n').map((_,i)=>i+1).join('\n');let escaped=area.value.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');highlight.innerHTML=escaped.replace(/(\/\/[^\n]*|#[^\n]*|&quot;[^\n]*?&quot;|'[^'\n]*'|\b(?:fn|let|mut|pub|struct|impl|use|const|async|await|function|return|if|else|class|def|import)\b)/g,m=>'<span class="'+(m.startsWith('//')||m.startsWith('#')?'comment':m.startsWith("'")||m.startsWith('&quot;')?'str':'kw')+'">'+m+'</span>')};area.oninput=numbers;area.onscroll=()=>{lines.scrollTop=area.scrollTop;highlight.scrollTop=area.scrollTop;highlight.scrollLeft=area.scrollLeft};code.append(highlight,area);editor.append(lines,code);let save=document.createElement('button');save.className='save-btn';save.style.marginTop='8px';save.textContent='Save File (Ctrl+S)';api('/api/file/read',{panel_id:p.id}).then(x=>{area.value=x.content;numbers()}).catch(show);save.onclick=()=>api('/api/file/write',{panel_id:p.id,content:area.value}).then(()=>show('Saved')).catch(show);area.onkeydown=e=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='s'){e.preventDefault();save.click()}};box.append(editor,save)}host.append(box)}}}catch(e){show(e)}}async function refresh(){try{render(await api('/api/snapshot'))}catch(e){show(e)}}function show(e){document.querySelector('#status').textContent=String(e)}document.querySelector('#reply').onsubmit=async e=>{e.preventDefault();try{await api('/api/reply',{session_id:state.selected,message:document.querySelector('#text').value});document.querySelector('#text').value='';show('Reply sent')}catch(e){show(e)}};let fallback;function poll(){if(!fallback){refresh();fallback=setInterval(refresh,2000)}}if(window.EventSource){let events=new EventSource('/api/events?token='+encodeURIComponent(token));events.addEventListener('snapshot',e=>render(JSON.parse(e.data)));events.addEventListener('update',e=>render(JSON.parse(e.data)));events.onopen=()=>{clearInterval(fallback);fallback=null};events.onerror=poll}else poll();
</script></body></html>"##;

pub async fn serve(socket: PathBuf, daemon_token: String) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|e| anyhow::anyhow!("random token: {e}"))?;
    let secret: String = random.iter().map(|b| format!("{b:02x}")).collect();
    println!("http://{addr}/");
    let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(32));
    loop {
        let (stream, _) = listener.accept().await?;
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            continue;
        };
        let s = socket.clone();
        let t = daemon_token.clone();
        let key = secret.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let _ = handle(stream, s, t, key, addr).await;
        });
    }
}
async fn handle(
    mut stream: tokio::net::TcpStream,
    socket: PathBuf,
    daemon_token: String,
    secret: String,
    addr: std::net::SocketAddr,
) -> Result<()> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    let end = loop {
        let n = tokio::time::timeout(std::time::Duration::from_secs(5), stream.read(&mut chunk))
            .await??;
        if n == 0 {
            bail!("closed");
        }
        data.extend_from_slice(&chunk[..n]);
        if data.len() > 1_100_000 {
            bail!("request too large");
        }
        if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let header = std::str::from_utf8(&data[..end])?;
    let mut lines = header.split("\r\n");
    let first = lines.next().context("request")?;
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let path = parts.next().unwrap_or("").to_owned();
    let mut host = "";
    let mut origin = None;
    let mut auth = "";
    let mut length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let v = v.trim();
            if k.eq_ignore_ascii_case("host") {
                host = v
            } else if k.eq_ignore_ascii_case("origin") {
                origin = Some(v)
            } else if k.eq_ignore_ascii_case("x-agmux-token") {
                auth = v
            } else if k.eq_ignore_ascii_case("content-length") {
                length = v.parse()?
            }
        }
    }
    let valid_host = host == addr.to_string();
    let valid_origin = origin
        .map(|o| o == format!("http://{addr}"))
        .unwrap_or(false);
    let (route, query) = path.split_once('?').unwrap_or((&path, ""));
    let query_token = query
        .split('&')
        .filter_map(|part| part.split_once('='))
        .find_map(|(key, value)| (key == "token").then_some(value))
        .unwrap_or("");
    let authorized = agmux_proto::constant_time_eq(auth, &secret)
        || (route == "/api/events" && agmux_proto::constant_time_eq(query_token, &secret));
    let permitted = valid_host
        && (method == "GET" && path == "/" || authorized && (method == "GET" || valid_origin));
    if length > 1_000_000 {
        bail!("body too large")
    };
    while data.len() < end + length {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            bail!("closed")
        };
        data.extend_from_slice(&chunk[..n]);
    }
    if permitted && method == "GET" && route == "/api/events" {
        let mut previous = snapshot(&socket, &daemon_token).await?.to_string();
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n").await?;
        stream
            .write_all(format!("event: snapshot\ndata: {previous}\n\n").as_bytes())
            .await?;
        let mut ticks = tokio::time::interval(std::time::Duration::from_millis(20));
        let mut idle = 0u32;
        loop {
            tokio::select! {
                _ = ticks.tick() => {},
                result = stream.read(&mut chunk) => {
                    if result? == 0 { return Ok(()); }
                }
            }
            let current = snapshot(&socket, &daemon_token).await?.to_string();
            if current != previous {
                stream
                    .write_all(format!("event: update\ndata: {current}\n\n").as_bytes())
                    .await?;
                previous = current;
                idle = 0;
            } else {
                idle += 1;
                if idle >= 500 {
                    stream.write_all(b": keepalive\n\n").await?;
                    idle = 0;
                }
            }
        }
    }
    let result: Result<(String, String)> = async {
        if !permitted {
            bail!("forbidden")
        }
        if method == "GET" && path == "/" {
            return Ok((
                "text/html; charset=utf-8".into(),
                PAGE.replace("__TOKEN__", &secret),
            ));
        }
        if method == "GET" && path == "/api/snapshot" {
            return Ok((
                "application/json".into(),
                snapshot(&socket, &daemon_token).await?.to_string(),
            ));
        }
        if method != "POST" {
            bail!("unknown endpoint")
        };
        let v: Value = serde_json::from_slice(&data[end..end + length])?;
        let string = |name: &str| -> Result<String> {
            Ok(v.get(name)
                .and_then(Value::as_str)
                .context("missing field")?
                .to_owned())
        };
        let output = match path.as_str() {
            "/api/select" => serde_json::to_value(
                request(
                    &socket,
                    &daemon_token,
                    CompanionRequest::Select {
                        session_id: string("session_id")?,
                    },
                )
                .await?,
            )?,
            "/api/file/read" => serde_json::to_value(
                request(
                    &socket,
                    &daemon_token,
                    CompanionRequest::Read {
                        panel_id: v["panel_id"].as_i64().context("panel id")?,
                    },
                )
                .await?,
            )?,
            "/api/file/write" => serde_json::to_value(
                request(
                    &socket,
                    &daemon_token,
                    CompanionRequest::Write {
                        panel_id: v["panel_id"].as_i64().context("panel id")?,
                        content: string("content")?,
                    },
                )
                .await?,
            )?,
            "/api/reply" => serde_json::to_value(
                request(
                    &socket,
                    &daemon_token,
                    CompanionRequest::Reply {
                        session_id: string("session_id")?,
                        message: string("message")?,
                    },
                )
                .await?,
            )?,
            _ => bail!("unknown endpoint"),
        };
        Ok(("application/json".into(), output.to_string()))
    }
    .await;
    let (status, kind, body) = match result {
        Ok((k, b)) => ("200 OK", k, b),
        Err(e) => (
            "403 Forbidden",
            "application/json".into(),
            json!({"error":e.to_string()}).to_string(),
        ),
    };
    stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",body.len()).as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    Ok(())
}

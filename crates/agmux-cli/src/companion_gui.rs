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

const PAGE: &str = r##"<!doctype html><html><head><meta charset="utf-8"><title>Agmux companion</title><style>body{font:15px system-ui;background:#101827;color:#e2e8f0;margin:0}header{padding:18px;background:#1e293b}main{display:grid;grid-template-columns:220px 1fr;gap:16px;padding:16px}button{padding:8px;cursor:pointer}pre,textarea{background:#020617;color:#cbd5e1;padding:12px;white-space:pre-wrap;min-height:150px;width:95%}iframe{width:100%;height:360px;background:white;border:0}.panel{background:#1e293b;padding:14px;margin:12px 0}.badge{background:#ef4444;border-radius:20px;padding:3px 10px}</style></head><body><header><strong>Agmux companion</strong> · Attention <span id="badge" class="badge">0</span></header><main><aside><h3>Sessions</h3><div id="sessions"></div></aside><section><h3>Session viewport</h3><pre id="screen"></pre><form id="reply"><input id="text" placeholder="Reply to session"><button>Send reply</button></form><h3>Docked panels</h3><div id="panels"></div><h3>Notifications</h3><div id="notifications"></div><p id="status"></p></section></main><script>
const token='__TOKEN__'; async function api(path,body){let r=await fetch(path,{method:body===undefined?'GET':'POST',headers:{'Content-Type':'application/json','X-Agmux-Token':token},body:body===undefined?undefined:JSON.stringify(body)});let j=await r.json();if(!r.ok)throw Error(j.error);return j} let state, signature='';
async function refresh(){try{let s=await api('/api/snapshot');state=s;document.querySelector('#badge').textContent=s.attention;document.querySelector('#screen').textContent=s.screen;let notices=document.querySelector('#notifications');notices.replaceChildren();for(let n of s.notifications.slice(0,10)){let p=document.createElement('p');p.textContent=n.title+' · '+n.body;notices.append(p)}let list=document.querySelector('#sessions');list.replaceChildren();for(let x of s.sessions){let b=document.createElement('button');b.textContent=(s.metadata[x.session_id]?.project||x.session_id)+' · '+x.state;b.onclick=async()=>{await api('/api/select',{session_id:x.session_id});signature='';refresh()};list.append(b,document.createElement('br'))}let sig=JSON.stringify([s.selected,s.selected_panel,s.panels]);if(sig!==signature){signature=sig;let host=document.querySelector('#panels');host.replaceChildren();for(let p of s.panels.filter(p=>p.session_id===s.selected)){let box=document.createElement('div');box.className='panel';if(p.id===s.selected_panel)box.style.outline='2px solid #38bdf8';let title=document.createElement('strong');title.textContent=p.target;box.append(title);if(p.kind==='web'){let f=document.createElement('iframe');f.setAttribute('sandbox','allow-scripts allow-forms');f.src=p.target;box.append(f)}else{let area=document.createElement('textarea');let save=document.createElement('button');save.textContent='Save';api('/api/file/read',{panel_id:p.id}).then(x=>area.value=x.content).catch(show);save.onclick=()=>api('/api/file/write',{panel_id:p.id,content:area.value}).then(()=>show('Saved')).catch(show);box.append(area,save)}host.append(box)}}}catch(e){show(e)}}function show(e){document.querySelector('#status').textContent=String(e)}document.querySelector('#reply').onsubmit=async e=>{e.preventDefault();try{await api('/api/reply',{session_id:state.selected,message:document.querySelector('#text').value});document.querySelector('#text').value='';show('Reply sent')}catch(e){show(e)}};refresh();setInterval(refresh,2000);
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
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                handle(stream, s, t, key, addr),
            )
            .await;
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
    let authorized = agmux_proto::constant_time_eq(auth, &secret);
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

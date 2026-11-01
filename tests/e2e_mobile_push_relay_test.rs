//! TEST-VER-GUI-004: Mobile push relays (ntfy.sh & webhooks with actionable inline replies).
#![cfg(unix)]

use agmux_proto::{IpcMessage, SpawnSessionRequest};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn ensure_binaries() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if Path::new("target/debug/agmuxd").exists() && Path::new("target/debug/agmux").exists() {
            return;
        }
        assert!(
            Command::new("cargo")
                .args(["build", "-p", "agmux-daemon", "-p", "agmux-cli"])
                .status()
                .unwrap()
                .success()
        );
    });
}

fn daemon(root: &Path) -> Process {
    let child = Command::new("target/debug/agmuxd")
        .env("AGMUX_SOCK", root.join("socket"))
        .env("AGMUX_AUTH_TOKEN_PATH", root.join("token"))
        .env("AGMUX_DB_PATH", root.join("db"))
        .env("AGMUX_CONFIG_PATH", root.join("config"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..300 {
        if root.join("socket").exists() {
            return Process(child);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("daemon did not start")
}

#[derive(Clone, Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

struct MockHttpServer {
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockHttpServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let reqs = requests.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accept_res = listener.accept() => {
                        let Ok((mut socket, _)) = accept_res else { break };
                        let reqs = reqs.clone();
                        tokio::spawn(async move {
                            use tokio::io::{AsyncReadExt, AsyncWriteExt};
                            let mut buf = Vec::new();
                            let mut chunk = [0u8; 4096];
                            let header_end;
                            loop {
                                let n = match socket.read(&mut chunk).await {
                                    Ok(0) | Err(_) => return,
                                    Ok(n) => n,
                                };
                                buf.extend_from_slice(&chunk[..n]);
                                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                                    header_end = pos + 4;
                                    break;
                                }
                            }
                            let header_str = String::from_utf8_lossy(&buf[..header_end]);
                            let mut lines = header_str.split("\r\n");
                            let req_line = lines.next().unwrap_or("");
                            let mut parts = req_line.split_whitespace();
                            let method = parts.next().unwrap_or("").to_string();
                            let path = parts.next().unwrap_or("").to_string();

                            let mut headers = HashMap::new();
                            let mut content_length = 0usize;
                            for line in lines {
                                if let Some((k, v)) = line.split_once(':') {
                                    let key = k.trim().to_lowercase();
                                    let val = v.trim().to_string();
                                    if key == "content-length" {
                                        content_length = val.parse().unwrap_or(0);
                                    }
                                    headers.insert(key, val);
                                }
                            }

                            while buf.len() < header_end + content_length {
                                let n = match socket.read(&mut chunk).await {
                                    Ok(0) | Err(_) => break,
                                    Ok(n) => n,
                                };
                                buf.extend_from_slice(&chunk[..n]);
                            }

                            let body = String::from_utf8_lossy(&buf[header_end..header_end + content_length]).to_string();
                            reqs.lock().unwrap().push(RecordedRequest {
                                method,
                                path,
                                headers,
                                body,
                            });

                            let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                            let _ = socket.write_all(response.as_bytes()).await;
                        });
                    }
                }
            }
        });

        Self {
            addr,
            requests,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    async fn wait_for_request(&self, timeout: Duration) -> Option<RecordedRequest> {
        let deadline = tokio::time::Instant::now() + timeout;
        while tokio::time::Instant::now() < deadline {
            if let Ok(guard) = self.requests.lock() {
                if let Some(req) = guard.last() {
                    return Some(req.clone());
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        None
    }
}

impl Drop for MockHttpServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

fn http_companion(
    addr: &str,
    method: &str,
    path: &str,
    key: &str,
    origin: &str,
    body: &str,
) -> (String, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nOrigin: {origin}\r\nX-Agmux-Token: {key}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
    (head.to_owned(), body.to_owned())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ntfy_push_relay_dispatch_on_waiting() {
    ensure_binaries();
    let root = std::env::temp_dir().join(format!(
        "agmux-push-ntfy-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "ntfy-test-branch"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );

    let mock = MockHttpServer::start().await;
    let mock_addr = mock.addr;

    let config_content = format!(
        r#"
[notifications.push]
provider = "ntfy"
url = "http://{mock_addr}"
topic = "alerts-topic"
token = "ntfy-auth-token"
reply_base_url = "http://127.0.0.1:4040"
"#
    );
    std::fs::write(root.join("config"), config_content).unwrap();

    let d = daemon(&root);
    let token = agmux_core::get_or_create_token(&root.join("token")).unwrap();
    let sock = root.join("socket");

    let spawn = IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
        session_id: "session-ntfy".into(),
        command: "sh".into(),
        args: vec![
            "-c".into(),
            "printf 'agent waiting: do you want to continue (y/n)?\\n'; sleep 30".into(),
        ],
        cwd: Some(root.to_string_lossy().into()),
        worktree_branch: None,
        ..Default::default()
    });
    assert!(matches!(
        agmux_cli::ipc_client::review_request(&sock, &token, spawn).await.unwrap(),
        IpcMessage::SpawnSessionResponse(r) if r.success
    ));
    tokio::time::sleep(Duration::from_millis(250)).await;

    let hook_resp = agmux_cli::ipc_client::send_internal_hook(
        &sock,
        &token,
        "session-ntfy".into(),
        "wait".into(),
    )
    .await
    .unwrap();
    assert!(hook_resp.success);

    let req = mock
        .wait_for_request(Duration::from_secs(5))
        .await
        .expect("ntfy push request should be received");

    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/alerts-topic");

    let title = req.headers.get("title").expect("title header");
    assert!(title.contains("session-ntfy") && title.contains("WAITING"));

    assert_eq!(
        req.headers.get("priority").map(|s| s.as_str()),
        Some("default")
    );
    assert_eq!(req.headers.get("tags").map(|s| s.as_str()), Some("warning"));
    assert_eq!(
        req.headers.get("authorization").map(|s| s.as_str()),
        Some("Bearer ntfy-auth-token")
    );

    let actions = req.headers.get("actions").expect("actions header");
    assert!(actions.contains("http, Reply Yes, http://127.0.0.1:4040/api/reply"));
    assert!(actions.contains("headers.Content-Type=application/json"));
    assert!(actions.contains(r#"body={"session_id":"session-ntfy","message":"y\n"}"#));
    assert!(actions.contains("http, Reply No, http://127.0.0.1:4040/api/reply"));
    assert!(actions.contains(r#"body={"session_id":"session-ntfy","message":"n\n"}"#));
    assert!(actions.contains("view, Open agmux, agmux://session/session-ntfy"));

    assert!(
        req.body
            .contains("agent waiting: do you want to continue (y/n)?")
    );

    drop(d);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_webhook_push_relay_dispatch_on_errored() {
    ensure_binaries();
    let root = std::env::temp_dir().join(format!(
        "agmux-push-webhook-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "webhook-test-branch"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );

    let mock = MockHttpServer::start().await;
    let mock_addr = mock.addr;

    let config_content = format!(
        r#"
[notifications.push]
provider = "webhook"
url = "http://{mock_addr}/events/webhook"
token = "webhook-secret-token"
reply_base_url = "http://127.0.0.1:5050"
"#
    );
    std::fs::write(root.join("config"), config_content).unwrap();

    let d = daemon(&root);
    let token = agmux_core::get_or_create_token(&root.join("token")).unwrap();
    let sock = root.join("socket");

    let spawn = IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
        session_id: "session-webhook".into(),
        command: "sh".into(),
        args: vec![
            "-c".into(),
            "printf 'fatal build exception: cannot find dependency\\n'; sleep 30".into(),
        ],
        cwd: Some(root.to_string_lossy().into()),
        worktree_branch: None,
        ..Default::default()
    });
    assert!(matches!(
        agmux_cli::ipc_client::review_request(&sock, &token, spawn).await.unwrap(),
        IpcMessage::SpawnSessionResponse(r) if r.success
    ));
    tokio::time::sleep(Duration::from_millis(250)).await;

    let hook_resp = agmux_cli::ipc_client::send_internal_hook(
        &sock,
        &token,
        "session-webhook".into(),
        "error".into(),
    )
    .await
    .unwrap();
    assert!(hook_resp.success);

    let req = mock
        .wait_for_request(Duration::from_secs(5))
        .await
        .expect("webhook push request should be received");

    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/events/webhook");
    assert_eq!(
        req.headers.get("content-type").map(|s| s.as_str()),
        Some("application/json")
    );
    assert_eq!(
        req.headers.get("authorization").map(|s| s.as_str()),
        Some("Bearer webhook-secret-token")
    );

    let json: serde_json::Value = serde_json::from_str(&req.body).expect("valid webhook json body");
    assert_eq!(json["kind"], "ERRORED");
    assert_eq!(json["session_id"], "session-webhook");
    assert!(json["title"].as_str().unwrap().contains("ERRORED"));
    assert!(
        json["body"]
            .as_str()
            .unwrap()
            .contains("fatal build exception: cannot find dependency")
    );

    let actions = json["actions"].as_array().expect("actions array");
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0]["label"], "Reply Yes");
    assert_eq!(actions[0]["url"], "http://127.0.0.1:5050/api/reply");
    assert_eq!(actions[0]["method"], "POST");
    assert_eq!(
        actions[0]["body"],
        "{\"session_id\":\"session-webhook\",\"message\":\"y\\n\"}"
    );

    assert_eq!(actions[1]["label"], "Reply No");
    assert_eq!(actions[1]["url"], "http://127.0.0.1:5050/api/reply");
    assert_eq!(actions[1]["method"], "POST");
    assert_eq!(
        actions[1]["body"],
        "{\"session_id\":\"session-webhook\",\"message\":\"n\\n\"}"
    );

    assert_eq!(actions[2]["label"], "Open agmux");
    assert_eq!(actions[2]["url"], "agmux://session/session-webhook");

    drop(d);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_push_relay_action_reply_integration() {
    ensure_binaries();
    let root = std::env::temp_dir().join(format!(
        "agmux-push-action-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "action-test-branch"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );

    let mock = MockHttpServer::start().await;
    let mock_addr = mock.addr;

    let d = daemon(&root);
    let token = agmux_core::get_or_create_token(&root.join("token")).unwrap();
    let sock = root.join("socket");

    let mut gui = Process(
        Command::new("target/debug/agmux")
            .args([
                "--socket",
                sock.to_str().unwrap(),
                "--token-path",
                root.join("token").to_str().unwrap(),
                "gui",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );

    let mut line = String::new();
    BufReader::new(gui.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let gui_addr = line
        .trim()
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches('/');
    let origin = format!("http://{gui_addr}");

    let (_, page) = http_companion(gui_addr, "GET", "/", "", &origin, "");
    let gui_token = page
        .split("const token='")
        .nth(1)
        .unwrap()
        .split('\'')
        .next()
        .unwrap();

    let config_content = format!(
        r#"
[notifications.push]
provider = "webhook"
url = "http://{mock_addr}/push/actions"
reply_base_url = "http://{gui_addr}"
"#
    );
    std::fs::write(root.join("config"), config_content).unwrap();

    let spawn = IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
        session_id: "action-session".into(),
        command: "sh".into(),
        args: vec![
            "-c".into(),
            "printf 'prompt-ready\\n'; read reply_input; echo RECEIVED_INLINE:$reply_input; sleep 30".into(),
        ],
        cwd: Some(root.to_string_lossy().into()),
        worktree_branch: None,
        ..Default::default()
    });
    assert!(matches!(
        agmux_cli::ipc_client::review_request(&sock, &token, spawn).await.unwrap(),
        IpcMessage::SpawnSessionResponse(r) if r.success
    ));
    tokio::time::sleep(Duration::from_millis(250)).await;

    let hook_resp = agmux_cli::ipc_client::send_internal_hook(
        &sock,
        &token,
        "action-session".into(),
        "wait".into(),
    )
    .await
    .unwrap();
    assert!(hook_resp.success);

    let req = mock
        .wait_for_request(Duration::from_secs(5))
        .await
        .expect("push request containing action button should be received");

    let json: serde_json::Value = serde_json::from_str(&req.body).unwrap();
    let reply_yes_body = json["actions"][0]["body"]
        .as_str()
        .expect("reply yes body string");

    let (status, _) = http_companion(
        gui_addr,
        "POST",
        "/api/reply",
        gui_token,
        &origin,
        reply_yes_body,
    );
    assert!(
        status.contains("200"),
        "reply request should return 200 OK: {status}"
    );

    let (sel_status, _) = http_companion(
        gui_addr,
        "POST",
        "/api/select",
        gui_token,
        &origin,
        r#"{"session_id":"action-session"}"#,
    );
    assert!(
        sel_status.contains("200"),
        "select request should return 200 OK: {sel_status}"
    );

    let mut got_reply = false;
    let mut last_snapshot = String::new();
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let (_, snapshot) =
            http_companion(gui_addr, "GET", "/api/snapshot", gui_token, &origin, "");
        if snapshot.contains("RECEIVED_INLINE:y") {
            got_reply = true;
            break;
        }
        last_snapshot = snapshot;
    }
    assert!(
        got_reply,
        "session output snapshot must contain 'RECEIVED_INLINE:y', got: {last_snapshot}"
    );

    drop(gui);
    drop(d);
    let _ = std::fs::remove_dir_all(root);
}

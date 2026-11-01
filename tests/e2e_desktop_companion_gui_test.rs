//! TEST-VER-GUI-005: desktop companion bridge and event delivery.
#![cfg(unix)]
use agmux_proto::{CompanionRequest, IpcMessage, SpawnSessionRequest};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
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
fn http(
    addr: &str,
    method: &str,
    path: &str,
    key: &str,
    origin: &str,
    body: &str,
) -> (String, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    write!(stream,"{method} {path} HTTP/1.1\r\nHost: {addr}\r\nOrigin: {origin}\r\nX-Agmux-Token: {key}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    (head.to_owned(), body.to_owned())
}
#[tokio::test(flavor = "multi_thread")]
async fn test_companion_gui_split_panels_read_write_and_deep_link_resolution() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-daemon", "-p", "agmux-cli"])
            .status()
            .unwrap()
            .success()
    );
    let root = std::env::temp_dir().join(format!(
        "agmux-32-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "task32"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(root.join("file.txt"), "original").unwrap();
    let hook = root.join("hook.sh");
    let result = root.join("hook-result");
    std::fs::write(&hook,format!("printf '%s\\n' \"$AGMUX_NOTIFY_KIND\" \"$AGMUX_SESSION_ID\" \"$AGMUX_SESSION_NAME\" \"$AGMUX_TOOL\" \"$AGMUX_CWD\" \"$AGMUX_BRANCH\" \"$AGMUX_TITLE\" \"$AGMUX_BODY\" > '{}'",result.display())).unwrap();
    std::fs::write(
        root.join("config"),
        format!("notification_hook = \"sh {}\"\n", hook.display()),
    )
    .unwrap();
    let d = daemon(&root);
    let token = agmux_core::get_or_create_token(&root.join("token")).unwrap();
    let sock = root.join("socket");
    let spawn = IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
        session_id: "demo".into(),
        command: "sh".into(),
        args: vec![
            "-c".into(),
            "printf 'hello-unicode-界\\n'; read line; echo REPLY:$line; sleep 30".into(),
        ],
        cwd: Some(root.to_string_lossy().into()),
        worktree_branch: None,
        ..Default::default()
    });
    assert!(
        matches!(agmux_cli::ipc_client::review_request(&sock,&token,spawn).await.unwrap(),IpcMessage::SpawnSessionResponse(r) if r.success)
    );
    let metadata = agmux_proto::WorkspaceMetadata {
        project: "Demo project".into(),
        group: "team".into(),
        favorite: true,
        archived: false,
    };
    assert!(
        matches!(agmux_cli::ipc_client::review_request(&sock, &token, IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::Set { session_id: "demo".into(), metadata })).await.unwrap(), IpcMessage::WorkspaceResponse(r) if r.error.is_none())
    );
    tokio::time::sleep(Duration::from_millis(250)).await;
    let hook_resp =
        agmux_cli::ipc_client::send_internal_hook(&sock, &token, "demo".into(), "wait".into())
            .await
            .unwrap();
    assert!(hook_resp.success);
    for _ in 0..100 {
        if result.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let notification = std::fs::read_to_string(&result).unwrap();
    assert!(notification.contains("WAITING\ndemo\nDemo project\nsh\n"));
    assert!(notification.contains("\ntask32\n"));
    assert!(notification.contains("hello-unicode-界"));
    let panels = agmux_cli::companion_gui::request(
        &sock,
        &token,
        CompanionRequest::Open {
            session_id: "demo".into(),
            target: "file.txt".into(),
        },
    )
    .await
    .unwrap();
    let id = panels.panels[0].id;
    agmux_cli::companion_gui::request(
        &sock,
        &token,
        CompanionRequest::Open {
            session_id: "demo".into(),
            target: "http://localhost:3000".into(),
        },
    )
    .await
    .unwrap();
    assert!(
        agmux_cli::companion_gui::request(
            &sock,
            &token,
            CompanionRequest::Open {
                session_id: "demo".into(),
                target: "../outside".into()
            }
        )
        .await
        .is_err()
    );
    assert!(
        agmux_cli::companion_gui::open_link(
            &sock,
            &token,
            &format!("agmux://session/demo/panel/{id}")
        )
        .await
        .is_ok()
    );
    assert!(
        agmux_cli::companion_gui::open_link(&sock, &token, "agmux://session/demo/panel/9999")
            .await
            .is_err()
    );
    let cli = |args: &[&str]| {
        Command::new("target/debug/agmux")
            .args([
                "--socket",
                sock.to_str().unwrap(),
                "--token-path",
                root.join("token").to_str().unwrap(),
            ])
            .args(args)
            .output()
            .unwrap()
    };
    assert!(
        cli(&["panels", "open", "file.txt", "--session", "demo"])
            .status
            .success()
    );
    assert!(cli(&["open", "agmux://session/demo"]).status.success());
    let snapshot_cli = cli(&["gui", "--snapshot", "--json"]);
    assert!(snapshot_cli.status.success());
    let snapshot_json: serde_json::Value = serde_json::from_slice(&snapshot_cli.stdout).unwrap();
    assert_eq!(snapshot_json["selected"], "demo");
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
    let addr = line
        .trim()
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches('/');
    let origin = format!("http://{addr}");
    let (_, page) = http(addr, "GET", "/", "", &origin, "");
    let key = page
        .split("const token='")
        .nth(1)
        .unwrap()
        .split('\'')
        .next()
        .unwrap();
    assert!(
        http(
            addr,
            "POST",
            "/api/select",
            key,
            "http://evil.invalid",
            r#"{"session_id":"demo"}"#
        )
        .0
        .contains("403")
    );
    let (status, snapshot) = http(addr, "GET", "/api/snapshot", key, &origin, "");
    assert!(status.contains("200"));
    assert!(snapshot.contains("hello-unicode"));
    assert!(snapshot.contains("file.txt"));
    assert!(
        http(
            addr,
            "POST",
            "/api/file/write",
            key,
            &origin,
            &format!(r#"{{"panel_id":{id},"content":"edited"}}"#)
        )
        .0
        .contains("200")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("file.txt")).unwrap(),
        "edited"
    );
    assert!(
        http(
            addr,
            "POST",
            "/api/file/read",
            key,
            &origin,
            r#"{"panel_id":9999}"#
        )
        .0
        .contains("403")
    );
    let mut events = TcpStream::connect(addr).unwrap();
    events
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    write!(
        events,
        "GET /api/events?token={key} HTTP/1.1\r\nHost: {addr}\r\n\r\n"
    )
    .unwrap();
    let mut events = BufReader::new(events);
    let mut line = String::new();
    loop {
        line.clear();
        events.read_line(&mut line).unwrap();
        if line.starts_with("data: ") {
            break;
        }
    }
    assert!(line.contains("hello-unicode"));
    let started = std::time::Instant::now();
    assert!(
        http(
            addr,
            "POST",
            "/api/reply",
            key,
            &origin,
            r#"{"session_id":"demo","message":"hello"}"#
        )
        .0
        .contains("200")
    );
    loop {
        line.clear();
        assert_ne!(events.read_line(&mut line).unwrap(), 0);
        if line.starts_with("data: ") && line.contains("REPLY:hello") {
            break;
        }
    }
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "event took {:?}",
        started.elapsed()
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (_, snap) = http(addr, "GET", "/api/snapshot", key, &origin, "");
    assert!(snap.contains("REPLY:hello"));
    assert!(
        agmux_cli::ipc_client::send_internal_hook(&sock, &token, "demo".into(), "stop".into())
            .await
            .unwrap()
            .success
    );
    let error_spawn = IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
        session_id: "error-demo".into(),
        command: "sh".into(),
        args: vec!["-c".into(), "printf error-output; sleep 30".into()],
        cwd: Some(root.to_string_lossy().into()),
        worktree_branch: None,
        ..Default::default()
    });
    assert!(
        matches!(agmux_cli::ipc_client::review_request(&sock, &token, error_spawn).await.unwrap(), IpcMessage::SpawnSessionResponse(r) if r.success)
    );
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        agmux_cli::ipc_client::send_internal_hook(
            &sock,
            &token,
            "error-demo".into(),
            "error".into()
        )
        .await
        .unwrap()
        .success
    );
    let events = agmux_cli::companion_gui::request(&sock, &token, CompanionRequest::List)
        .await
        .unwrap()
        .notifications;
    assert!(
        events
            .iter()
            .any(|n| n.kind == "state.FINISHED" && n.session_id == "demo")
    );
    assert!(events.iter().any(|n| n.kind == "state.ERRORED"
        && n.session_id == "error-demo"
        && n.body.contains("error-output")));
    drop(gui);
    drop(d);
    std::fs::remove_file(root.join("socket")).ok();
    let _d = daemon(&root);
    let state = agmux_cli::companion_gui::request(&sock, &token, CompanionRequest::List)
        .await
        .unwrap();
    assert!(
        state
            .notifications
            .iter()
            .any(|n| n.kind == "state.WAITING" && n.body.contains("hello-unicode"))
    );
    assert_eq!(state.selected_panel, Some(id));
    assert_eq!(state.panels.len(), 2);
    assert_eq!(state.selected.as_deref(), Some("demo"));
    drop(_d);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_desktop_protocol_registration() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("agmux-scheme-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let xdg = root.join("xdg-mime");
    std::fs::write(&xdg, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&xdg, std::fs::Permissions::from_mode(0o755)).unwrap();
    let exe = std::env::current_dir().unwrap().join("target/debug/agmux");
    assert!(
        Command::new(&exe)
            .arg("register-scheme")
            .env("HOME", &root)
            .env("PATH", &root)
            .status()
            .unwrap()
            .success()
    );
    let entry = std::fs::read_to_string(
        root.join(".local/share/applications/agmux-scheme-handler.desktop"),
    )
    .unwrap();
    assert!(entry.contains("MimeType=x-scheme-handler/agmux;"));
    assert!(entry.contains(exe.to_str().unwrap()));
    std::fs::remove_dir_all(root).unwrap();
}

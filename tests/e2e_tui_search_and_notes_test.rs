//! TEST-VER-TUI-007/008: search and notes through authenticated daemon IPC.
#![cfg(unix)]
use agmux_proto::{IpcMessage, SearchNotesRequest, SearchNotesResponse};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Fixture {
    root: PathBuf,
    daemon: Child,
}
impl Fixture {
    fn start(root: PathBuf) -> Self {
        let daemon = Command::new("target/debug/agmuxd")
            .env("AGMUX_SOCK", root.join("socket"))
            .env("AGMUX_AUTH_TOKEN_PATH", root.join("token"))
            .env("AGMUX_DB_PATH", root.join("state.db"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        for _ in 0..150 {
            if root.join("socket").exists() {
                return Self { root, daemon };
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("daemon did not start");
    }
    fn cmd(&self) -> Command {
        // Use an absolute binary path because the session starts in the fixture repository.
        let mut cmd = {
            let mut c = Command::new(std::env::current_dir().unwrap().join("target/debug/agmux"));
            c.env("AGMUX_SOCK", self.root.join("socket"))
                .env("AGMUX_AUTH_TOKEN_PATH", self.root.join("token"))
                .env("AGMUX_DB_PATH", self.root.join("state.db"))
                .current_dir(&self.root);
            c
        };
        cmd.stdin(Stdio::null());
        cmd
    }
    async fn request(&self, req: SearchNotesRequest) -> SearchNotesResponse {
        let token = agmux_core::get_or_create_token(&self.root.join("token")).unwrap();
        match agmux_cli::ipc_client::review_request(
            &self.root.join("socket"),
            &token,
            IpcMessage::SearchNotesRequest(req),
        )
        .await
        .unwrap()
        {
            IpcMessage::SearchNotesResponse(r) => {
                assert!(r.error.is_none(), "{:?}", r.error);
                r
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_file(self.root.join("socket"));
    }
}

#[tokio::test]
async fn transcript_notes_filter_and_restart() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let root = std::env::temp_dir().join(format!(
        "agmux-30-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "task30-branch"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    let f = Fixture::start(root.clone());
    let mut live = f
        .cmd()
        .args([
            "run",
            "sh",
            "-c",
            "printf 'before\\nneedle-live\\nafter\\nunfinished-live'; sleep 60",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let live_id = format!("run-{}", live.id());
    let mut dead = f
        .cmd()
        .args([
            "run",
            "sh",
            "-c",
            "printf 'before\\nneedle-dead\\nafter\\n'",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let dead_id = format!("run-{}", dead.id());
    assert!(dead.wait().unwrap().success());
    let mut hits = Vec::new();
    for _ in 0..100 {
        hits = f
            .request(SearchNotesRequest::Search {
                query: "needle".into(),
            })
            .await
            .hits;
        if hits.iter().any(|h| h.session_id == live_id)
            && hits.iter().any(|h| h.session_id == dead_id)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert!(
        hits.iter()
            .any(|h| h.session_id == live_id && h.context[h.context_match].contains("needle-live"))
    );
    assert!(
        f.request(SearchNotesRequest::Search {
            query: "unfinished-live".into()
        })
        .await
        .hits
        .iter()
        .any(|hit| hit.session_id == live_id)
    );
    assert!(
        hits.iter()
            .any(|h| h.session_id == dead_id && h.context[h.context_match].contains("needle-dead"))
    );
    assert!(
        f.request(SearchNotesRequest::Search { query: "".into() })
            .await
            .hits
            .is_empty()
    );
    assert!(
        f.request(SearchNotesRequest::Search {
            query: "\" OR *".into()
        })
        .await
        .hits
        .is_empty()
    );
    for (page, value) in [(1, "alpha note"), (2, "special note marker")] {
        let out = f
            .cmd()
            .args(["note", "set", &live_id, &page.to_string(), value])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let list = f.cmd().args(["note", "list", &live_id]).output().unwrap();
    assert!(String::from_utf8_lossy(&list.stdout).contains("[2]\nspecial note marker"));
    assert!(
        f.request(SearchNotesRequest::Search {
            query: "special note".into()
        })
        .await
        .hits
        .iter()
        .any(|h| h.session_id == live_id && h.page == Some(2))
    );
    for query in ["special note", "task30-branch", &live_id] {
        assert!(
            f.request(SearchNotesRequest::Filter {
                query: query.into()
            })
            .await
            .session_ids
            .contains(&live_id),
            "filter {query}"
        );
    }
    // Drive the same framed requests produced by the TUI and inspect its rendered preview.
    let mut app = agmux_tui::App {
        sessions: vec![agmux_proto::SessionInfo {
            session_id: live_id.clone(),
            pid: live.id(),
            cwd: root.to_string_lossy().into_owned(),
            command: "sh".into(),
            created_at: 0,
            state: "RUNNING".into(),
        }],
        ..Default::default()
    };
    app.handle_key_with_modifiers(
        crossterm::event::KeyCode::Char('f'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    for c in "needle".chars() {
        app.handle_key(crossterm::event::KeyCode::Char(c));
    }
    let IpcMessage::SearchNotesRequest(request) = app.pending_message.take().unwrap() else {
        panic!("search request")
    };
    app.apply_search_notes_response(f.request(request).await);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|frame| agmux_tui::ui::render(frame, &app))
        .unwrap();
    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(rendered.contains("Read-only context"));
    assert!(rendered.contains("needle"));
    app.handle_key(crossterm::event::KeyCode::Esc);
    app.handle_key(crossterm::event::KeyCode::Char('N'));
    let IpcMessage::SearchNotesRequest(request) = app.pending_message.take().unwrap() else {
        panic!("note get")
    };
    app.apply_search_notes_response(f.request(request).await);
    app.handle_key(crossterm::event::KeyCode::Char('!'));
    let IpcMessage::SearchNotesRequest(request) = app.pending_message.take().unwrap() else {
        panic!("note set")
    };
    f.request(request).await;
    let out = f
        .cmd()
        .args(["note", "get", &live_id, "1"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "alpha note!");
    let _ = live.kill();
    let _ = live.wait();
    drop(f);
    let f = Fixture::start(root.clone());
    let out = f
        .cmd()
        .args(["note", "get", &live_id, "2"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "special note marker"
    );
    drop(f);
    let _ = std::fs::remove_dir_all(root);
}

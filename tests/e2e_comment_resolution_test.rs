#![cfg(unix)]
use agmux_proto::*;
use futures_util::{SinkExt, StreamExt};
use std::{
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::UnixStream;

#[tokio::test]
async fn resolves_comments_via_ipc_and_diff_inspection() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let root = std::env::temp_dir().join(format!(
        "agmux-review-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "test@example.com"],
        &["config", "user.name", "Test"],
    ] {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    for i in 1..=3 {
        std::fs::write(root.join(format!("file{i}")), "original\n").unwrap();
    }
    assert!(
        Command::new("git")
            .current_dir(&root)
            .args(["add", "."])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .current_dir(&root)
            .args(["commit", "-qm", "base"])
            .status()
            .unwrap()
            .success()
    );
    let socket = root.join("socket");
    let token = root.join("token");
    let mut daemon = Command::new("target/debug/agmuxd")
        .args([
            "--socket",
            socket.to_str().unwrap(),
            "--token-path",
            token.to_str().unwrap(),
        ])
        .env("AGMUX_DB_PATH", root.join("state.db"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let secret = std::fs::read_to_string(token).unwrap();
    let mut client = new_framed(UnixStream::connect(&socket).await.unwrap());
    client
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: "test".into(),
            token: secret.trim().into(),
        }))
        .await
        .unwrap();
    assert!(
        matches!(client.next().await.unwrap().unwrap(), IpcMessage::HandshakeResponse(r) if r.success)
    );
    client
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "review".into(),
            command: "cat".into(),
            args: vec![],
            cwd: Some(root.to_string_lossy().into_owned()),
            worktree_branch: None,
            ..Default::default()
        }))
        .await
        .unwrap();
    assert!(
        matches!(client.next().await.unwrap().unwrap(), IpcMessage::SpawnSessionResponse(r) if r.success)
    );
    let mut ids = Vec::new();
    for i in 1..=3 {
        client
            .send(IpcMessage::AddReviewCommentRequest(
                AddReviewCommentRequest {
                    session_id: "review".into(),
                    file_path: format!("file{i}"),
                    line_number: i,
                    content: format!("comment{i}"),
                },
            ))
            .await
            .unwrap();
        loop {
            if let IpcMessage::AddReviewCommentResponse(r) = client.next().await.unwrap().unwrap() {
                ids.push(r.comment_id.unwrap());
                break;
            }
        }
    }
    client
        .send(IpcMessage::DispatchReviewRoundRequest(
            DispatchReviewRoundRequest {
                session_id: "review".into(),
            },
        ))
        .await
        .unwrap();
    let mut output = Vec::new();
    let mut response = None;
    let mut working = false;
    tokio::time::timeout(Duration::from_secs(5), async {
        while response.is_none()
            || !working
            || !String::from_utf8_lossy(&output).contains("comment3")
        {
            match client.next().await.unwrap().unwrap() {
                IpcMessage::DispatchReviewRoundResponse(r) => response = Some(r),
                IpcMessage::SessionStateChangedEvent(e) if e.new_state == "WORKING" => {
                    working = true
                }
                IpcMessage::TerminalOutputFrame(f) => output.extend(f.data),
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    let response = response.unwrap();
    assert!(response.success);
    assert_eq!((response.round_number, response.count), (1, 3));
    for id in &ids {
        assert!(response.prompt.contains(&format!("[ID: {id}]")));
    }
    client
        .send(IpcMessage::ListReviewCommentsRequest(
            ListReviewCommentsRequest {
                session_id: "review".into(),
                status: Some("sent".into()),
            },
        ))
        .await
        .unwrap();
    loop {
        if let IpcMessage::ListReviewCommentsResponse(r) = client.next().await.unwrap().unwrap() {
            assert_eq!(r.comments.len(), 3);
            break;
        }
    }
    client
        .send(IpcMessage::ResolveReviewCommentRequest(
            ResolveReviewCommentRequest {
                session_id: Some("review".into()),
                comment_id: ids[0].clone(),
            },
        ))
        .await
        .unwrap();
    loop {
        if let IpcMessage::ResolveReviewCommentResponse(r) = client.next().await.unwrap().unwrap() {
            assert!(r.success);
            break;
        }
    }
    std::fs::write(root.join("file2"), "changed\n").unwrap();
    client
        .send(IpcMessage::GetDiffRequest(GetDiffRequest {
            session_id: "review".into(),
            scope: DiffScope::FullUncommitted,
            target: None,
        }))
        .await
        .unwrap();
    loop {
        if let IpcMessage::GetDiffResponse(r) = client.next().await.unwrap().unwrap() {
            assert!(r.error.is_none());
            break;
        }
    }
    client
        .send(IpcMessage::ListReviewCommentsRequest(
            ListReviewCommentsRequest {
                session_id: "review".into(),
                status: None,
            },
        ))
        .await
        .unwrap();
    loop {
        if let IpcMessage::ListReviewCommentsResponse(r) = client.next().await.unwrap().unwrap() {
            assert_eq!(
                r.comments
                    .iter()
                    .map(|c| c.status.as_str())
                    .collect::<Vec<_>>(),
                vec!["resolved", "resolved", "sent"]
            );
            break;
        }
    }
    client
        .send(IpcMessage::ResolveReviewCommentRequest(
            ResolveReviewCommentRequest {
                session_id: None,
                comment_id: "missing".into(),
            },
        ))
        .await
        .unwrap();
    loop {
        if let IpcMessage::ResolveReviewCommentResponse(r) = client.next().await.unwrap().unwrap() {
            assert!(!r.success);
            break;
        }
    }
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

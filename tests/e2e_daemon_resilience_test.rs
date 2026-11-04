//! Verification Tests:
//! - `TEST-VER-DAEMON-003` (`REQ-DAEMON-SHUTDOWN-SIG-003`, `LLD-DAEMON-SHUTDOWN-SIG-003`)
//! - `TEST-VER-DAEMON-004` (`REQ-DAEMON-LOCKFILE-004`, `LLD-DAEMON-LOCKFILE-004`)
//! - `TEST-VER-DAEMON-005` (`REQ-DAEMON-RECOVERY-005`, `LLD-DAEMON-RECOVERY-005`)
//! - `TEST-VER-STORAGE-005` (`REQ-STORAGE-CONFIG-HOTRELOAD-005`, `LLD-STORAGE-CONFIG-HOTRELOAD-005`)

#![cfg(unix)]
use std::{
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use agmux_core::storage_engine::StorageEngine;
use agmux_daemon::{
    config_watcher::{ConfigWatcher, RuntimeConfig},
    instance_lock::InstanceLock,
    journal_watcher::JournalWatcher,
    recovery_manager::RecoveryManager,
};
use agmux_proto::{
    HandshakeRequest, IpcMessage, ListSessionsRequest, ShutdownDaemonRequest, SpawnSessionRequest,
    new_framed,
};
use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixStream;

fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "agmux-{prefix}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `TEST-VER-DAEMON-004`: Enforce single-instance execution via `InstanceLock` (`agmuxd.pid`),
/// refusing duplicate daemon instances without removing the running daemon's socket.
#[tokio::test]
async fn test_ver_daemon_004_single_instance_lockfile() {
    let root = unique_test_dir("lockfile");
    let lock_path = root.join("agmuxd.pid");
    let socket = root.join("daemon.sock");
    let token = root.join("token");
    let db_path = root.join("state.db");

    // Unit-level lockfile check: holding `InstanceLock` blocks a second `acquire`
    {
        let _lock1 = InstanceLock::acquire(&lock_path).expect("first lock succeeds");
        assert!(lock_path.exists());
        let lock2 = InstanceLock::acquire(&lock_path);
        assert!(
            lock2.is_err(),
            "second InstanceLock::acquire must fail while first is held"
        );
    }

    // E2E daemon check: start first agmuxd, then attempt a second agmuxd on the same socket
    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let mut daemon1 = Command::new(binaries.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .env("AGMUX_DB_PATH", &db_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert!(socket.exists(), "First daemon must create socket");

    // Attempt to spawn a duplicate daemon on the same socket
    let dup = Command::new(binaries.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .env("AGMUX_DB_PATH", &db_path)
        .output()
        .unwrap();
    assert!(
        !dup.status.success(),
        "Duplicate daemon must exit with non-zero error"
    );
    assert!(
        socket.exists(),
        "Duplicate daemon attempt must NOT unlink the active daemon's socket"
    );

    let _ = daemon1.kill();
    let _ = daemon1.wait();
    let _ = std::fs::remove_dir_all(&root);
}

/// `TEST-VER-DAEMON-005`: On startup, `RecoveryManager` transitions orphaned sessions
/// (`RUNNING`, `WORKING`, `WAITING`, `WAITING_INPUT`) to `DEAD`, releases locks/reservations,
/// and preserves compressed ANSI scrollback snapshots.
#[test]
fn test_ver_daemon_005_crash_recovery_orphaned_sessions() {
    let root = unique_test_dir("recovery");
    let db_path = root.join("state.db");

    let db = StorageEngine::open(&db_path).expect("open db");
    db.insert_session(&agmux_proto::SessionInfo {
        session_id: "orphan-running".into(),
        pid: 999_991,
        cwd: "/tmp".into(),
        command: "claude".into(),
        created_at: 100,
        state: "RUNNING".into(),
    })
    .unwrap();
    db.insert_session(&agmux_proto::SessionInfo {
        session_id: "orphan-waiting".into(),
        pid: 999_992,
        cwd: "/tmp".into(),
        command: "codex".into(),
        created_at: 101,
        state: "WAITING_INPUT".into(),
    })
    .unwrap();

    // Orphaned session holds a lock, a file reservation, and a compressed snapshot
    assert!(db.acquire_lock("db_migrate", "orphan-running").unwrap());
    assert!(
        db.reserve_file("crates/core/src/lib.rs", "orphan-running")
            .unwrap()
            .is_ok()
    );
    let snap = agmux_core::snapshot_compressor::compress_snapshot(b"PRESERVED_SCROLLBACK_DATA")
        .expect("compress");
    db.save_snapshot("orphan-running", &snap, 25).unwrap();

    // Run crash recovery
    let recovered = RecoveryManager::recover_orphaned_sessions(&db).expect("recover");
    assert_eq!(recovered.len(), 2);

    // Verify both sessions transitioned to DEAD
    let s1 = db.get_session("orphan-running").unwrap().unwrap();
    let s2 = db.get_session("orphan-waiting").unwrap().unwrap();
    assert_eq!(s1.state, "DEAD");
    assert_eq!(s2.state, "DEAD");

    // Verify locks and reservations were evicted
    assert!(db.list_locks().unwrap().is_empty());
    assert!(db.list_reservations().unwrap().is_empty());

    // Verify snapshot is preserved
    let loaded = db.load_snapshot("orphan-running").unwrap().unwrap();
    let decompressed = agmux_core::snapshot_compressor::decompress_snapshot(&loaded).unwrap();
    assert_eq!(decompressed, b"PRESERVED_SCROLLBACK_DATA");

    // Verify journal recorded `session.recovered_dead`
    let events = db.read_events_since(0, None).unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.event_type == "session.recovered_dead"
                && e.session_id.as_deref() == Some("orphan-running"))
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// `TEST-VER-STORAGE-005`: `ConfigWatcher` watches `agmux.toml` and hot-reloads
/// `idle_after_ms`, `blocked_settle_ms`, and `fts_retention_lines`, emitting `config.reloaded`.
#[tokio::test]
async fn test_ver_storage_005_config_hot_reload() {
    let root = unique_test_dir("config-hotreload");
    let config_path = root.join("agmux.toml");
    let db_path = root.join("state.db");

    std::fs::write(
        &config_path,
        "[terminal]\nscrollback = 10000\nidle_after_ms = 120000\nblocked_settle_ms = 500\n",
    )
    .unwrap();

    let storage = std::sync::Arc::new(std::sync::Mutex::new(
        StorageEngine::open(&db_path).unwrap(),
    ));
    let journal = JournalWatcher::new(storage.clone());
    let watcher = ConfigWatcher::new(config_path.clone());
    assert_eq!(
        watcher.get(),
        RuntimeConfig {
            idle_after_ms: 120_000,
            blocked_settle_ms: 500,
            fts_retention_lines: 10_000,
            ..Default::default()
        }
    );

    let watch_handle = watcher.spawn_watch_loop(journal.clone());

    // Modify config on disk while watcher is running
    std::fs::write(
        &config_path,
        "[terminal]\nfts_retention_lines = 25000\nidle_after_ms = 45000\nblocked_settle_ms = 250\n",
    )
    .unwrap();

    let mut updated = false;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let current = watcher.get();
        if current.idle_after_ms == 45_000
            && current.blocked_settle_ms == 250
            && current.fts_retention_lines == 25_000
        {
            updated = true;
            break;
        }
    }
    assert!(updated, "ConfigWatcher must hot-reload updated values");

    let events = journal.read_since(0).await.unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.event_type == "config.reloaded" && e.payload.contains("idle_after_ms=45000")),
        "Journal must record config.reloaded event"
    );

    watch_handle.abort();
    let _ = std::fs::remove_dir_all(&root);
}

/// `TEST-VER-DAEMON-003`: Graceful daemon shutdown propagates `SIGHUP` then `SIGTERM`
/// to active child agent process groups before exiting.
#[tokio::test]
async fn test_ver_daemon_003_graceful_shutdown_signals_children() {
    let root = unique_test_dir("shutdown");
    let socket = root.join("daemon.sock");
    let token_path = root.join("token");
    let db_path = root.join("state.db");

    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let mut daemon = Command::new(binaries.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token_path)
        .env("AGMUX_DB_PATH", &db_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    let secret = std::fs::read_to_string(&token_path).unwrap();
    let mut client = new_framed(UnixStream::connect(&socket).await.unwrap());
    client
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: "test".into(),
            token: secret.trim().into(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        client.next().await.unwrap().unwrap(),
        IpcMessage::HandshakeResponse(r) if r.success
    ));

    // Spawn a long-running child process inside a PTY session
    client
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "child-sleep".into(),
            command: "sleep".into(),
            args: vec!["60".into()],
            cwd: None,
            worktree_branch: None,
            ..Default::default()
        }))
        .await
        .unwrap();
    assert!(matches!(
        client.next().await.unwrap().unwrap(),
        IpcMessage::SpawnSessionResponse(r) if r.success
    ));

    // Query the child PID
    client
        .send(IpcMessage::ListSessionsRequest(ListSessionsRequest {}))
        .await
        .unwrap();
    let child_pid = loop {
        match client.next().await.unwrap().unwrap() {
            IpcMessage::ListSessionsResponse(r) => {
                break r
                    .sessions
                    .iter()
                    .find(|s| s.session_id == "child-sleep")
                    .unwrap()
                    .pid;
            }
            _ => continue,
        }
    };

    // Request graceful daemon shutdown via ShutdownDaemonRequest
    client
        .send(IpcMessage::ShutdownDaemonRequest(ShutdownDaemonRequest {}))
        .await
        .unwrap();
    let terminated_count = loop {
        match client.next().await.unwrap().unwrap() {
            IpcMessage::ShutdownDaemonResponse(r) => {
                assert!(r.success);
                break r.terminated_sessions;
            }
            _ => continue,
        }
    };
    assert!(terminated_count >= 1);

    let status = daemon.wait().unwrap();
    assert!(status.success());

    // Verify the child process was terminated (kill(pid, 0) != 0)
    for _ in 0..30 {
        let alive = unsafe { libc::kill(child_pid as libc::pid_t, 0) == 0 };
        if !alive {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    let still_alive = unsafe { libc::kill(child_pid as libc::pid_t, 0) == 0 };
    assert!(
        !still_alive,
        "Child process {child_pid} must be terminated on daemon shutdown"
    );

    let _ = std::fs::remove_dir_all(&root);
}

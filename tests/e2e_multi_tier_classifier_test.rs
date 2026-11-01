//! End-to-end verification tests for the Multi-Tier State Classifier Engine.
//!
//! Verifies:
//! - REQ-STATE-TIER1-HOOKS-002 (TEST-VER-STATE-002)
//! - REQ-STATE-TIER2-CHILDPID-003 (TEST-VER-STATE-003)
//! - REQ-STATE-TIER3-TAILMATCH-004 (TEST-VER-STATE-004)
//! - REQ-STATE-DEBOUNCE-TIMERS-005 (TEST-VER-STATE-005)
//! - REQ-STATE-SHELL-TRACKING-006 (TEST-VER-STATE-006)

use std::path::PathBuf;
use std::time::{Duration, Instant};

use agmux_cli::ipc_client::{list_sessions, review_request, send_internal_hook};
use agmux_core::child_process_monitor::ChildProcessMonitor;
use agmux_core::debounce_timer::DebounceTimer;
use agmux_core::shell_classifier::ShellClassifier;
use agmux_core::state_machine::SessionState;
use agmux_core::tail_matcher::{TailMatchResult, TailMatcher};
use agmux_daemon::DaemonServer;
use agmux_proto::{IpcMessage, SpawnSessionRequest, connect_ipc};

struct TempDir(PathBuf);

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn start_test_daemon(dir: &TempDir) -> (PathBuf, String) {
    let sock_path = dir.path().join("agmux-state-e2e.sock");
    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();

    let server = DaemonServer::new(sock_path.clone(), token.clone());
    tokio::spawn(async move {
        let _ = server.run().await;
    });

    for _ in 0..50 {
        if connect_ipc(&sock_path).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    (sock_path, token)
}

#[tokio::test]
async fn test_ver_state_002_tier1_deterministic_cli_hooks() {
    let dir = TempDir::new("agmux-state-002");
    let (sock_path, token) = start_test_daemon(&dir).await;

    // Spawn a background session
    let spawn_resp = review_request(
        &sock_path,
        &token,
        IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "hook-agent-1".to_string(),
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 10".to_string()],
            cwd: Some(dir.path().display().to_string()),
            worktree_branch: None,
            ..Default::default()
        }),
    )
    .await
    .expect("spawn session");
    assert!(matches!(
        spawn_resp,
        IpcMessage::SpawnSessionResponse(ref r) if r.success
    ));

    // 1. Trigger `wait` hook -> WAITING_INPUT
    let t0 = Instant::now();
    let res1 = send_internal_hook(
        &sock_path,
        &token,
        "hook-agent-1".to_string(),
        "wait".to_string(),
    )
    .await
    .expect("wait hook");
    assert!(res1.success);
    assert_eq!(res1.new_state, "WAITING_INPUT");
    assert!(
        t0.elapsed() < Duration::from_millis(250),
        "Hook IPC took {:?}",
        t0.elapsed()
    );

    let sessions = list_sessions(&sock_path, &token).await.expect("list");
    let s = sessions
        .iter()
        .find(|s| s.session_id == "hook-agent-1")
        .expect("found session");
    assert_eq!(s.state, "WAITING_INPUT");

    // 2. Trigger `start` hook -> RUNNING
    let res2 = send_internal_hook(
        &sock_path,
        &token,
        "hook-agent-1".to_string(),
        "start".to_string(),
    )
    .await
    .expect("start hook");
    assert!(res2.success);
    assert_eq!(res2.new_state, "RUNNING");

    let sessions = list_sessions(&sock_path, &token).await.expect("list");
    let s = sessions
        .iter()
        .find(|s| s.session_id == "hook-agent-1")
        .expect("found session");
    assert_eq!(s.state, "RUNNING");

    // 3. Trigger `stop` hook -> COMPLETED
    let res3 = send_internal_hook(
        &sock_path,
        &token,
        "hook-agent-1".to_string(),
        "stop".to_string(),
    )
    .await
    .expect("stop hook");
    assert!(res3.success);
    assert_eq!(res3.new_state, "COMPLETED");

    // 4. Trigger `error` hook -> FAILED
    let res4 = send_internal_hook(
        &sock_path,
        &token,
        "hook-agent-1".to_string(),
        "error".to_string(),
    )
    .await
    .expect("error hook");
    assert!(res4.success);
    assert_eq!(res4.new_state, "FAILED");
}

#[test]
fn test_ver_state_003_tier2_child_process_inspection() {
    let mut parent_sh = std::process::Command::new("sh")
        .args(["-c", "sleep 0.5 & wait"])
        .spawn()
        .expect("spawn parent sh");
    let parent_pid = parent_sh.id();
    std::thread::sleep(Duration::from_millis(100));

    assert!(
        ChildProcessMonitor::has_active_children(parent_pid),
        "Expected active child PID under parent shell {parent_pid}"
    );

    let _ = parent_sh.wait();
    assert!(
        !ChildProcessMonitor::has_active_children(parent_pid),
        "Expected no active children after parent shell exits"
    );
}

#[test]
fn test_ver_state_004_tier3_bounded_tail_matching_and_false_positive_immunity() {
    // 1. Historical log output in upper rows must NOT trigger a false-positive prompt match
    let mut rows: Vec<String> = (0..19)
        .map(|i| format!("Row {i}: Allow tool execution? (y/n) [Y/n] Do you want to proceed?"))
        .collect();
    for i in 19..24 {
        rows.push(format!("Row {i}: normal compiler output line"));
    }
    assert_eq!(TailMatcher::classify_lines(&rows), TailMatchResult::None);

    // 2. Active Braille spinner in bottom 5 rows -> SpinnerWorking
    rows[23] = "⠋ Thinking and analyzing repository...".to_string();
    assert_eq!(
        TailMatcher::classify_lines(&rows),
        TailMatchResult::SpinnerWorking
    );
    assert_eq!(
        TailMatcher::classify_lines(&rows).to_session_state(),
        Some(SessionState::Running)
    );

    // 3. Confirmation prompt in bottom 5 rows -> PromptWaitingInput
    rows[23] = "Allow bash command execution? [y/N]".to_string();
    assert_eq!(
        TailMatcher::classify_lines(&rows),
        TailMatchResult::PromptWaitingInput
    );

    // 4. Rate limit banner in bottom 5 rows -> RateLimitOrQuotaError
    rows[23] = "Error: 429 Too Many Requests - Quota exceeded".to_string();
    assert_eq!(
        TailMatcher::classify_lines(&rows),
        TailMatchResult::RateLimitOrQuotaError
    );
    assert_eq!(
        TailMatcher::classify_lines(&rows).to_session_state(),
        Some(SessionState::Failed)
    );
}

#[test]
fn test_ver_state_005_debounce_timers_and_quiet_periods() {
    let mut timer = DebounceTimer::new(1800, 300);
    let t0 = Instant::now();

    // Prompt detected at t0: settle window is 300ms
    timer.record_output(t0, true);
    assert_eq!(timer.evaluate(t0 + Duration::from_millis(100), false), None);
    assert_eq!(
        timer.evaluate(t0 + Duration::from_millis(350), false),
        Some(SessionState::WaitingInput)
    );

    // Normal output at t1: idle_after_ms is 1800ms
    let t1 = t0 + Duration::from_millis(500);
    timer.record_output(t1, false);
    assert_eq!(
        timer.evaluate(t1 + Duration::from_millis(1200), false),
        None
    );
    // Active child processes lock state to Running even after 2000ms of silence
    assert_eq!(
        timer.evaluate(t1 + Duration::from_millis(2000), true),
        Some(SessionState::Running)
    );
    // Once child processes finish and quiet period >= 1800ms -> Idle
    assert_eq!(
        timer.evaluate(t1 + Duration::from_millis(2000), false),
        Some(SessionState::Idle)
    );
}

#[test]
fn test_ver_state_006_shell_tab_child_process_tracking() {
    let mut parent_sh = std::process::Command::new("sh")
        .args(["-c", "sleep 0.5 & wait"])
        .spawn()
        .expect("spawn parent sh");
    let shell_pid = parent_sh.id();
    std::thread::sleep(Duration::from_millis(100));

    let mut shell = ShellClassifier::new(shell_pid);
    assert_eq!(shell.state(), SessionState::Idle);

    // Active child command transitions shell sub-tab from Idle to Running
    assert_eq!(
        shell.poll(),
        Some((SessionState::Idle, SessionState::Running))
    );
    assert_eq!(shell.state(), SessionState::Running);

    // Wait for child command to exit -> shell sub-tab returns to Idle
    let _ = parent_sh.wait();
    assert_eq!(
        shell.poll(),
        Some((SessionState::Running, SessionState::Idle))
    );
    assert_eq!(shell.state(), SessionState::Idle);
}

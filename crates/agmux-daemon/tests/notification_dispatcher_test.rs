use agmux_daemon::notification_dispatcher::{HookNotification, latest_quote, run_hook};

fn notification(path: String) -> HookNotification {
    HookNotification {
        kind: "WAITING".into(),
        session_id: "session".into(),
        session_name: "name".into(),
        tool: "tool".into(),
        cwd: path,
        branch: "branch".into(),
        title: "title".into(),
        body: "body".into(),
    }
}

#[test]
fn unicode_quote_is_last_120_characters() {
    let output = format!("start{}", "界".repeat(120));
    assert_eq!(latest_quote(&output), "界".repeat(120));
}

#[cfg(unix)]
#[tokio::test]
async fn hook_receives_all_fields() {
    let path = std::env::temp_dir().join(format!("agmux-hook-{}", std::process::id()));
    let n = notification(path.to_string_lossy().into_owned());
    run_hook("printf '%s\\n' \"$AGMUX_NOTIFY_KIND\" \"$AGMUX_SESSION_ID\" \"$AGMUX_SESSION_NAME\" \"$AGMUX_TOOL\" \"$AGMUX_CWD\" \"$AGMUX_BRANCH\" \"$AGMUX_TITLE\" \"$AGMUX_BODY\" > \"$AGMUX_CWD\"", &n).await.expect("hook");
    let result = std::fs::read_to_string(&path).expect("hook output");
    assert_eq!(
        result,
        format!(
            "WAITING\nsession\nname\ntool\n{}\nbranch\ntitle\nbody\n",
            path.display()
        )
    );
    std::fs::remove_file(path).expect("cleanup");
}

#[cfg(unix)]
#[tokio::test]
async fn hook_times_out_after_fifteen_seconds() {
    let path = std::env::temp_dir().join(format!("agmux-hook-pid-{}", std::process::id()));
    let start = std::time::Instant::now();
    let error = run_hook(
        "sleep 30 & echo $! > \"$AGMUX_CWD\"; wait",
        &notification(path.to_string_lossy().into_owned()),
    )
    .await
    .expect_err("timeout");
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(start.elapsed() >= std::time::Duration::from_secs(15));
    assert!(start.elapsed() < std::time::Duration::from_secs(20));
    let pid: u32 = std::fs::read_to_string(&path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let status = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    assert!(
        status.is_empty() || status.split_whitespace().nth(2) == Some("Z"),
        "descendant still running: {status}"
    );
    std::fs::remove_file(path).unwrap();
}

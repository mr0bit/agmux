use std::process::Command;

#[test]
fn overview_and_detailed_references_work_without_a_daemon() {
    let bin = env!("CARGO_BIN_EXE_agmux");
    let overview = Command::new(bin).arg("agent-context").output().unwrap();
    assert!(overview.status.success());
    let text = String::from_utf8(overview.stdout).unwrap();
    assert!(text.split_whitespace().count() < 250);
    assert!(text.contains("task") && text.contains("lock"));
    for name in ["task", "lock"] {
        let result = Command::new(bin)
            .args(["agent-context", "--command", name])
            .output()
            .unwrap();
        assert!(result.status.success());
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(
            text.contains("Usage:") && text.contains("--session") && text.contains("JSON example")
        );
    }
    assert!(
        !Command::new(bin)
            .args(["agent-context", "--command", "no-such-command"])
            .status()
            .unwrap()
            .success()
    );
}

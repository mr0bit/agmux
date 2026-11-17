//! Startup diagnostics exercised through the actual CLI and backend executables.
use std::{
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn directory() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agmux-startup-error-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}
fn binary(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}
fn cli(root: &std::path::Path) -> Command {
    let mut command = Command::new(binary("agmux"));
    command
        .args(["gui", "--snapshot"])
        .env("AGMUXD_BIN", binary("agmuxd"))
        .env("AGMUX_SOCK", root.join("socket"))
        .env("AGMUX_AUTH_TOKEN_PATH", root.join("auth_token"))
        .env("AGMUX_DB_PATH", root.join("invalid-database"))
        .env_remove("AGMUX_LOCK_PATH");
    command
}

#[test]
fn startup_error_explains_backend_failure_and_keeps_diagnostic_log() {
    let root = directory();
    // SQLite cannot open an existing directory as its database file.
    std::fs::create_dir(root.join("invalid-database")).unwrap();
    let output = cli(&root).output().expect("run built CLI");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Agmux could not start its background service"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Failed to open session database"),
        "{stderr}"
    );
    assert!(stderr.contains("Startup log:"), "{stderr}");
    assert!(stderr.contains("agmux doctor"), "{stderr}");
    let log = std::fs::read_to_string(root.join("auth_token.daemon.log")).unwrap();
    assert!(log.contains("Failed to open session database"));
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn large_backend_stderr_is_saved_without_filling_a_pipe_and_error_is_bounded() {
    use std::os::unix::fs::PermissionsExt;
    let root = directory();
    let fake = root.join("failing-backend");
    std::fs::write(
        &fake,
        format!(
            "#!/bin/sh\nprintf '%s' '{}' >&2\nprintf '\\nFINAL STARTUP REASON\\n' >&2\nexit 7\n",
            "x".repeat(100_000)
        ),
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = cli(&root).env("AGMUXD_BIN", fake).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("FINAL STARTUP REASON"),
        "missing tail diagnostic"
    );
    assert!(stderr.len() < 10_000, "unbounded diagnostic output");
    assert!(
        std::fs::metadata(root.join("auth_token.daemon.log"))
            .unwrap()
            .len()
            > 100_000
    );
    std::fs::remove_dir_all(root).unwrap();
}

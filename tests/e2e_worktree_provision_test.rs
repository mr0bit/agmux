#![cfg(unix)]
use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn provisions_worktree_for_cli_session() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let root = std::env::temp_dir().join(format!(
        "agmux-worktree-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.email", "test@example.com"]);
    git(&root, &["config", "user.name", "Test"]);
    std::fs::write(root.join("tracked"), "initial").unwrap();
    git(&root, &["add", "tracked"]);
    git(&root, &["commit", "-m", "initial"]);
    std::fs::write(root.join(".worktreeinclude"), ".env\n*.local\n").unwrap();
    std::fs::write(
        root.join(".env"),
        "TEST_WORKTREE_SECRET=AgmuxWorktreeSecret123\n",
    )
    .unwrap();
    std::fs::write(root.join("config.local"), "local configuration").unwrap();
    let socket = root.join("daemon.sock");
    let token = root.join("token");
    let mut daemon = Command::new(binaries.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .env("AGMUX_DB_PATH", root.join("sessions.db"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let output = Command::new(binaries.join("agmux"))
        .current_dir(&root)
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "run",
            "--worktree",
            "test-branch",
            "sh",
            "-c",
            "test -f .env && cat .env && pwd && echo PORT=$PORT",
        ])
        .output()
        .unwrap();
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let path = root.join(".agmux-worktrees/test-branch");
    assert!(path.is_dir());
    assert!(path.join("config.local").is_file());
    assert!(
        stdout.contains("TEST_WORKTREE_SECRET=AgmuxWorktreeSecret123"),
        "{stdout}"
    );
    assert!(stdout.contains(path.to_str().unwrap()), "{stdout}");
    let port: u16 = stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("PORT="))
        .unwrap()
        .parse()
        .unwrap();
    assert!((8000..9000).contains(&port));
    let branch = Command::new("git")
        .current_dir(&path)
        .args(["branch", "--show-current"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&branch.stdout).trim(),
        "test-branch"
    );
    std::fs::remove_dir_all(root).unwrap();
}

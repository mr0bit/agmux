//! TEST-VER-ORCH-006/007/008: CLI, MCP and teardown over the daemon's public IPC.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    io::Write,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Fixture {
    root: PathBuf,
    daemon: Child,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        assert!(
            Command::new("cargo")
                .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
                .status()
                .unwrap()
                .success()
        );
        let root = std::env::temp_dir().join(format!(
            "agmux-29-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let daemon = Command::new("target/debug/agmuxd")
            .env("AGMUX_SOCK", root.join("socket"))
            .env("AGMUX_AUTH_TOKEN_PATH", root.join("token"))
            .env("AGMUX_DB_PATH", root.join("state.db"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        for _ in 0..100 {
            if root.join("socket").exists() {
                break;
            }
            thread::sleep(Duration::from_millis(30));
        }
        assert!(root.join("socket").exists());
        Self { root, daemon }
    }
    fn cmd(&self) -> Command {
        let mut c = Command::new("target/debug/agmux");
        c.env("AGMUX_SOCK", self.root.join("socket"))
            .env("AGMUX_AUTH_TOKEN_PATH", self.root.join("token"))
            .env("AGMUX_DB_PATH", self.root.join("state.db"));
        c
    }
    fn cli(&self, args: &[&str]) -> std::process::Output {
        self.cmd().args(args).output().unwrap()
    }
    fn rpc(&self, mode: &str, lines: &[Value]) -> Vec<Value> {
        let mut c = self
            .cmd()
            .arg(mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let input = c.stdin.as_mut().unwrap();
            for line in lines {
                writeln!(input, "{line}").unwrap();
            }
        }
        let output = c.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
    fn state(&self, id: &str) -> String {
        rusqlite::Connection::open(self.root.join("state.db"))
            .unwrap()
            .query_row("SELECT state FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap()
    }
}
fn call(id: u32, name: &str, args: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})
}
fn data(response: &Value) -> Value {
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}
#[test]
fn mcp_and_deferred_teardown() {
    let f = Fixture::new();
    let overview = f.cli(&["agent-context"]);
    assert!(overview.status.success());
    assert!(
        String::from_utf8_lossy(&overview.stdout)
            .split_whitespace()
            .count()
            < 250
    );
    let results=f.rpc("mcp",&[
    json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
    json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    call(3,"task_create",json!({"task_id":"T29","title":"Proof"})),
    call(4,"lock_acquire",json!({"lock_name":"x","session_id":"worker"})),
    call(5,"list_sessions",json!({})),
    call(6,"task_list",json!({})),
    call(7,"read_session",json!({"session_id":"missing"})),
    call(8,"task_create",json!({"title":"missing id"})),
    json!({"jsonrpc":"2.0","id":9,"method":"unknown"}),
 ]);
    assert_eq!(results.len(), 9);
    assert_eq!(results[0]["result"]["serverInfo"]["name"], "agmux");
    assert_eq!(results[1]["result"]["tools"].as_array().unwrap().len(), 16);
    assert_eq!(data(&results[2])["success"], true);
    assert_eq!(data(&results[3])["acquired"], true);
    assert_eq!(data(&results[5])["tasks"][0]["id"], "T29");
    assert_eq!(results[6]["result"]["isError"], true);
    assert_eq!(results[7]["error"]["code"], -32602);
    assert_eq!(results[8]["error"]["code"], -32601);
    assert_eq!(
        f.rpc(
            "--enable-mcp",
            &[json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})]
        )[0]["result"]["tools"]
            .as_array()
            .unwrap()
            .len(),
        16
    );
    let mut worker = f
        .cmd()
        .args(["run", "sh", "-c", "echo PROOF; sleep 30"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let sid = format!("run-{}", worker.id());
    for _ in 0..100 {
        let l = f.cli(&["list", "--json"]);
        if String::from_utf8_lossy(&l.stdout).contains(&sid) {
            break;
        }
        thread::sleep(Duration::from_millis(30));
    }
    let review = f.cli(&[
        "review",
        "comment",
        &sid,
        "--file",
        "Cargo.toml",
        "--line",
        "1",
        "--content",
        "review this",
    ]);
    assert!(
        review.status.success(),
        "{}",
        String::from_utf8_lossy(&review.stderr)
    );
    let comment_id = String::from_utf8(review.stdout).unwrap().trim().to_owned();
    let calls = f.rpc(
        "mcp",
        &[
            call(10, "read_session", json!({"session_id":sid})),
            call(
                11,
                "send_message",
                json!({"target_session_id":sid,"message":"hello"}),
            ),
            call(
                12,
                "task_claim",
                json!({"task_id":"T29","session_id":"worker"}),
            ),
            call(
                13,
                "report_submit",
                json!({"task_id":"T29","session_id":"worker","summary":"done"}),
            ),
            call(
                14,
                "task_finish",
                json!({"task_id":"T29","session_id":"worker"}),
            ),
            call(
                15,
                "lock_release",
                json!({"lock_name":"x","session_id":"worker"}),
            ),
            call(
                16,
                "reserve_files",
                json!({"session_id":"worker","paths":["Cargo.toml"]}),
            ),
            call(
                17,
                "release_files",
                json!({"session_id":"worker","paths":["Cargo.toml"]}),
            ),
            call(
                18,
                "resolve_review_comment",
                json!({"comment_id":comment_id}),
            ),
            call(
                19,
                "spawn_session",
                json!({"session_id":"mcp-worker","command":"sh","args":["-c","sleep 30"]}),
            ),
            call(20, "kill_self", json!({"session_id":"mcp-worker"})),
        ],
    );
    for r in &calls {
        assert_eq!(r["result"]["isError"], false, "{r}");
    }
    assert!(
        f.cli(&[
            "internal-hook",
            "--session",
            "mcp-worker",
            "--event",
            "stop"
        ])
        .status
        .success()
    );
    assert_eq!(f.state("mcp-worker"), "DEAD");
    assert!(f.cli(&["kill-self", &sid]).status.success());
    let listed = f.rpc("mcp", &[call(1, "list_sessions", json!({}))]);
    assert_eq!(data(&listed[0])["teardowns"][&sid], "kill");
    assert!(f.cli(&["cancel-teardown", &sid]).status.success());
    let listed = f.rpc("mcp", &[call(1, "list_sessions", json!({}))]);
    assert!(data(&listed[0])["teardowns"][&sid].is_null());
    assert!(f.cli(&["archive-self", &sid]).status.success());
    assert!(f.cli(&["cancel-teardown", &sid]).status.success());
    let scheduled = f.rpc("mcp", &[call(1, "archive_self", json!({"session_id":sid}))]);
    assert_eq!(scheduled[0]["result"]["isError"], false);
    assert!(
        f.cli(&["internal-hook", "--session", &sid, "--event", "stop"])
            .status
            .success()
    );
    for _ in 0..100 {
        if f.state(&sid) == "ARCHIVED" {
            break;
        }
        thread::sleep(Duration::from_millis(30));
    }
    assert_eq!(f.state(&sid), "ARCHIVED");
    assert!(!f.cli(&["kill-self", &sid]).status.success());
    assert!(f.cli(&["inspect", &sid]).status.success());
    let _ = worker.wait();
    let mut worker = f
        .cmd()
        .args(["run", "sh", "-c", "sleep 1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let sid = format!("run-{}", worker.id());
    for _ in 0..100 {
        if String::from_utf8_lossy(&f.cli(&["list", "--json"]).stdout).contains(&sid) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(f.cli(&["kill-self", &sid]).status.success());
    let _ = worker.wait();
    for _ in 0..100 {
        if f.state(&sid) == "DEAD" {
            break;
        }
        thread::sleep(Duration::from_millis(30));
    }
    assert_eq!(f.state(&sid), "DEAD");
}

//! End-to-end verification tests for the Two-Tier Agent Plugin System (`TASK-26`).
//!
//! Verifies:
//! - REQ-PLUGIN-TRAIT-001 (TEST-VER-PLUGIN-001)
//! - REQ-PLUGIN-STUCK-HEURISTIC-002 (TEST-VER-PLUGIN-002)
//! - REQ-PLUGIN-BUILTIN-ADAPTERS-003 (TEST-VER-PLUGIN-003)
//! - REQ-PLUGIN-DECLARATIVE-TOML-004 (TEST-VER-PLUGIN-004)
//! - REQ-PLUGIN-UUID-CAPTURE-005 (TEST-VER-PLUGIN-005)
//! - REQ-PLUGIN-MISSING-DETECTOR-006 (TEST-VER-PLUGIN-006)

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agmux_core::agent_discovery::{
    discover_all_builtin_agents_in_paths, discover_installed_agents,
};
use agmux_core::builtin_plugins::{all_builtin_plugins, is_windows_system_cmd_collision};
use agmux_core::cli_detector::CliDetector;
use agmux_core::declarative_plugin::DeclarativePlugin;
use agmux_core::plugin_api::{
    AgentPlugin, PluginOutputSignal, PluginRegistry, UuidCaptureStrategy,
};
use agmux_core::session_tracker::SessionTracker;
use agmux_core::storage_engine::StorageEngine;
use agmux_proto::SessionInfo;

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

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct MockAgentPlugin;

impl AgentPlugin for MockAgentPlugin {
    fn id(&self) -> &str {
        "mock-agent"
    }

    fn display_name(&self) -> &str {
        "Mock Agent"
    }

    fn binary_name_for_os(&self, _is_windows: bool) -> &str {
        "mock-agent-bin"
    }

    fn prompt_prefix(&self) -> &str {
        "mock> "
    }

    fn spawn_args(&self, pre_minted_uuid: Option<&str>, extra_args: &[String]) -> Vec<String> {
        let mut out = vec!["--headless".to_string()];
        if let Some(u) = pre_minted_uuid {
            out.push("--session-id".to_string());
            out.push(u.to_string());
        }
        out.extend_from_slice(extra_args);
        out
    }

    fn resume_args(&self, session_uuid: &str) -> Vec<String> {
        vec!["--resume".to_string(), session_uuid.to_string()]
    }

    fn check_stuck(
        &self,
        screen_lines: &[&str],
        last_input_elapsed_ms: u64,
        pty_silence_ms: u64,
        has_active_children: bool,
    ) -> bool {
        !has_active_children
            && last_input_elapsed_ms >= 10_000
            && pty_silence_ms >= 10_000
            && screen_lines
                .iter()
                .any(|l| l.starts_with("mock> ") && !l["mock> ".len()..].trim().is_empty())
    }

    fn parse_output(&self, line: &str) -> PluginOutputSignal {
        if line.contains("MOCK_WORKING") {
            PluginOutputSignal::Working
        } else {
            PluginOutputSignal::None
        }
    }
}

#[test]
fn test_ver_plugin_001_trait_registration_and_zero_overhead_callbacks() {
    let mut registry = PluginRegistry::new();
    registry.register(Arc::new(MockAgentPlugin));

    let plugin = registry.get("mock-agent").expect("mock-agent registered");
    assert_eq!(plugin.display_name(), "Mock Agent");
    assert_eq!(
        plugin.spawn_args(Some("uuid-123"), &["--fast".to_string()]),
        vec!["--headless", "--session-id", "uuid-123", "--fast"]
    );
    assert_eq!(plugin.resume_args("uuid-123"), vec!["--resume", "uuid-123"]);
    assert_eq!(
        plugin.parse_output("status: MOCK_WORKING"),
        PluginOutputSignal::Working
    );
}

#[test]
fn test_ver_plugin_002_stuck_prompt_heuristic_after_10s_silence() {
    let registry = PluginRegistry::with_builtins();
    let claude = registry.get("claude").expect("claude plugin");
    let aider = registry.get("aider").expect("aider plugin");

    let claude_screen = vec!["Welcome to Claude Code", "claude> refactor auth.rs"];
    let aider_screen = vec!["Aider v0.80", "aider> /add src/main.rs"];

    // Under 10s -> not stuck
    assert!(!claude.check_stuck(&claude_screen, 5_000, 5_000, false));
    // Active child process -> not stuck even after 12s
    assert!(!claude.check_stuck(&claude_screen, 12_000, 12_000, true));
    // Empty prompt -> not stuck
    assert!(!claude.check_stuck(&["claude>   "], 12_000, 12_000, false));

    // >= 10s with unsubmitted text after prompt marker and no child process -> STUCK!
    assert!(claude.check_stuck(&claude_screen, 10_500, 10_500, false));
    assert!(aider.check_stuck(&aider_screen, 10_500, 10_500, false));
}

#[test]
fn test_ver_plugin_003_all_14_builtin_adapters_and_windows_cmd_collision_guard() {
    let plugins = all_builtin_plugins();
    assert_eq!(plugins.len(), 14);

    let expected_ids = [
        "claude",
        "agy",
        "gemini",
        "codex",
        "opencode",
        "aider",
        "cursor-agent",
        "grok",
        "q",
        "pi",
        "omp",
        "cmd",
        "hermes",
        "muse",
    ];
    for expected in expected_ids {
        let p = plugins
            .iter()
            .find(|p| p.id() == expected)
            .unwrap_or_else(|| panic!("Missing builtin plugin {expected}"));
        assert!(!p.display_name().is_empty());
        assert!(!p.resume_args("sess-999").is_empty());
    }

    // Verify Command Code (`cmd`) Windows binary resolution and System32 collision guard
    let cmd_plugin = plugins.iter().find(|p| p.id() == "cmd").expect("cmd");
    assert_eq!(cmd_plugin.binary_name_for_os(false), "cmd");
    assert_eq!(cmd_plugin.binary_name_for_os(true), "command-code");
    assert!(is_windows_system_cmd_collision(
        "cmd",
        Path::new(r"C:\Windows\System32\cmd.exe")
    ));
    assert!(is_windows_system_cmd_collision(
        "cmd",
        Path::new(r"C:\Windows\SysWOW64\cmd.exe")
    ));
    assert!(!is_windows_system_cmd_collision(
        "cmd",
        Path::new(r"C:\Users\dev\AppData\Roaming\npm\cmd.cmd")
    ));

    // Verify `discover_installed_agents()` keeps length 6 for backward compatibility with onboarding wizard
    assert_eq!(discover_installed_agents().len(), 6);
    assert_eq!(discover_all_builtin_agents_in_paths(&[], false).len(), 14);
}

#[test]
fn test_ver_plugin_004_declarative_toml_agent_profile() {
    let toml_raw = r#"
[agents.my_custom_agent]
display_name = "In-House Agent"
binary = "inhouse-ai"
prompt_regex = "inhouse> "
stuck_timeout_ms = 8000
limit_regex = "INHOUSE_QUOTA_EXCEEDED|429_LIMIT"
resume_cmd = ["--continue-session", "{session_id}"]
session_id_flag = "--uuid"
"#;

    let loaded = DeclarativePlugin::load_from_toml_str(toml_raw).expect("parse agents.toml");
    assert_eq!(loaded.len(), 1);
    let custom = &loaded[0];
    assert_eq!(custom.id(), "my_custom_agent");
    assert_eq!(custom.display_name(), "In-House Agent");
    assert_eq!(custom.binary_name(), "inhouse-ai");
    assert_eq!(
        custom.spawn_args(Some("u-42"), &["--verbose".to_string()]),
        vec!["--uuid", "u-42", "--verbose"]
    );
    assert_eq!(
        custom.resume_args("u-42"),
        vec!["--continue-session", "u-42"]
    );
    assert_eq!(
        custom.parse_output("Error: INHOUSE_QUOTA_EXCEEDED"),
        PluginOutputSignal::RateLimited
    );
    assert!(custom.check_stuck(&["inhouse> draft text"], 8_500, 8_500, false));
}

#[test]
fn test_ver_plugin_005_uuid_capture_flag_injection_and_store_watcher() {
    let dir = TempDir::new("agmux-plugin-uuid");
    let db_path = dir.path().join("state.db");
    let db = StorageEngine::open(&db_path).expect("open db");

    for sid in ["claude-sess", "agy-sess"] {
        db.insert_session(&SessionInfo {
            session_id: sid.to_string(),
            pid: 100,
            cwd: "/tmp".to_string(),
            command: sid.to_string(),
            created_at: 1,
            state: "RUNNING".to_string(),
        })
        .expect("insert session");
    }

    // (a) Pre-minted UUID flag injection (e.g. Claude `--session-id <uuid>`)
    let (minted_uuid, args) = SessionTracker::inject_and_persist_uuid(
        &db,
        "claude-sess",
        "--session-id",
        &["--print".to_string()],
    )
    .expect("inject uuid");
    assert!(SessionTracker::is_uuid_format(&minted_uuid));
    assert_eq!(args, vec!["--session-id", &minted_uuid, "--print"]);
    assert_eq!(
        db.get_cli_session_uuid("claude-sess").expect("get uuid"),
        Some(minted_uuid)
    );

    // (b) Store directory watcher (e.g. Antigravity `.gemini/antigravity-cli/brain/<uuid>`)
    let brain_dir = dir.path().join("brain");
    std::fs::create_dir_all(&brain_dir).expect("create brain dir");
    let store_uuid = "e17e1333-cee2-48b7-be19-e21f542f10e0";
    std::fs::create_dir_all(brain_dir.join(store_uuid)).expect("create conv dir");

    let captured = SessionTracker::capture_from_store_dir(&db, "agy-sess", &brain_dir)
        .expect("capture store")
        .expect("found uuid");
    assert_eq!(captured, store_uuid);
    assert_eq!(
        db.get_cli_session_uuid("agy-sess").expect("get agy uuid"),
        Some(store_uuid.to_string())
    );
}

#[test]
fn test_ver_plugin_006_missing_cli_detector_and_installation_banner() {
    let dir = TempDir::new("agmux-empty-path");
    let result = CliDetector::check_in_dirs("claude", &[dir.path().to_path_buf()]);
    let err = result.expect_err("expected missing binary guidance");
    assert_eq!(err.binary_name, "claude");
    assert_eq!(err.display_name, "Claude Code");
    assert!(err.install_command.contains("@anthropic-ai/claude-code"));
    let banner = err.format_banner();
    assert!(banner.contains("not installed"));
    assert!(banner.contains("https://"));

    // Verify UuidCaptureStrategy is exposed properly on built-ins
    let claude = PluginRegistry::with_builtins().get("claude").unwrap();
    assert!(matches!(
        claude.uuid_capture_strategy(),
        UuidCaptureStrategy::PreMintedFlag(_)
    ));
}

//! Compiled-in `AgentPlugin` implementations for all 14 first-class CLI tools (`REQ-PLUGIN-BUILTIN-ADAPTERS-003`).

use std::path::Path;
use std::sync::Arc;

use crate::plugin_api::{AgentPlugin, PluginOutputSignal, UuidCaptureStrategy};
use crate::stuck_detector::{DEFAULT_STUCK_TIMEOUT_MS, StuckDetector};

/// Compiled-in adapter definition for a first-class coding agent CLI.
#[derive(Debug, Clone)]
pub struct BuiltinAgentPlugin {
    id: &'static str,
    display_name: &'static str,
    unix_binary: &'static str,
    windows_binary: &'static str,
    prompt_prefix: &'static str,
    resume_subcommand: Option<&'static str>,
    resume_flag: &'static str,
    uuid_strategy: UuidCaptureStrategy,
    stuck_detector: StuckDetector,
}

impl AgentPlugin for BuiltinAgentPlugin {
    fn id(&self) -> &str {
        self.id
    }

    fn display_name(&self) -> &str {
        self.display_name
    }

    fn binary_name_for_os(&self, is_windows: bool) -> &str {
        if is_windows {
            self.windows_binary
        } else {
            self.unix_binary
        }
    }

    fn prompt_prefix(&self) -> &str {
        self.prompt_prefix
    }

    fn spawn_args(&self, pre_minted_uuid: Option<&str>, extra_args: &[String]) -> Vec<String> {
        let mut args = Vec::new();
        if let (UuidCaptureStrategy::PreMintedFlag(flag), Some(uuid)) =
            (&self.uuid_strategy, pre_minted_uuid)
        {
            args.push(flag.clone());
            args.push(uuid.to_string());
        }
        args.extend_from_slice(extra_args);
        args
    }

    fn resume_args(&self, session_uuid: &str) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(sub) = self.resume_subcommand {
            args.push(sub.to_string());
        }
        if !self.resume_flag.is_empty() {
            args.push(self.resume_flag.to_string());
        }
        args.push(session_uuid.to_string());
        args
    }

    fn fork_args(&self, session_uuid: &str) -> Option<Vec<String>> {
        match self.id {
            "codex" => Some(vec!["fork".into(), session_uuid.into()]),
            "claude" => Some(vec![
                "--resume".into(),
                session_uuid.into(),
                "--fork-session".into(),
            ]),
            _ => None,
        }
    }

    fn uuid_capture_strategy(&self) -> UuidCaptureStrategy {
        self.uuid_strategy.clone()
    }

    fn check_stuck(
        &self,
        screen_lines: &[&str],
        last_input_elapsed_ms: u64,
        pty_silence_ms: u64,
        has_active_children: bool,
    ) -> bool {
        self.stuck_detector.is_stuck(
            self.prompt_prefix,
            screen_lines,
            last_input_elapsed_ms,
            pty_silence_ms,
            has_active_children,
        )
    }

    fn parse_output(&self, line: &str) -> PluginOutputSignal {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return PluginOutputSignal::None;
        }
        if trimmed.contains("Rate limit exceeded")
            || trimmed.contains("429 Too Many Requests")
            || trimmed.contains("Quota exceeded")
            || trimmed.contains("insufficient_quota")
        {
            return PluginOutputSignal::RateLimited;
        }
        if trimmed.contains("(y/n)")
            || trimmed.contains("[y/N]")
            || trimmed.contains("[Y/n]")
            || trimmed.contains("Do you want to proceed")
            || trimmed.contains("Allow?")
        {
            return PluginOutputSignal::WaitingInput;
        }
        if trimmed
            .chars()
            .any(|c| crate::tail_matcher::BRAILLE_SPINNERS.contains(&c))
        {
            return PluginOutputSignal::Working;
        }
        PluginOutputSignal::None
    }
}

/// Guard against Windows `C:\Windows\System32\cmd.exe` or `SysWOW64\cmd.exe` colliding with Command Code (`cmd`).
pub fn is_windows_system_cmd_collision(binary_id: &str, resolved_path: &Path) -> bool {
    if !binary_id.eq_ignore_ascii_case("cmd") {
        return false;
    }
    let normalized = resolved_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    normalized.ends_with("/system32/cmd.exe")
        || normalized.ends_with("/syswow64/cmd.exe")
        || normalized == "c:/windows/cmd.exe"
}

/// Return all 14 compiled-in first-class CLI agent plugins (`REQ-PLUGIN-BUILTIN-ADAPTERS-003`).
pub fn all_builtin_plugins() -> Vec<Arc<dyn AgentPlugin>> {
    let detector = StuckDetector::new(DEFAULT_STUCK_TIMEOUT_MS);
    vec![
        Arc::new(BuiltinAgentPlugin {
            id: "claude",
            display_name: "Claude Code",
            unix_binary: "claude",
            windows_binary: "claude",
            prompt_prefix: "claude> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::PreMintedFlag("--session-id".to_string()),
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "agy",
            display_name: "Antigravity",
            unix_binary: "agy",
            windows_binary: "agy",
            prompt_prefix: "agy> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::StoreWatcher {
                store_dir_hint: ".gemini/antigravity-cli/brain".to_string(),
            },
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "gemini",
            display_name: "Gemini CLI",
            unix_binary: "gemini",
            windows_binary: "gemini",
            prompt_prefix: "gemini> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::PreMintedFlag("--session-id".to_string()),
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "codex",
            display_name: "Codex",
            unix_binary: "codex",
            windows_binary: "codex",
            prompt_prefix: "codex> ",
            resume_subcommand: Some("resume"),
            resume_flag: "",
            uuid_strategy: UuidCaptureStrategy::StoreWatcher {
                store_dir_hint: ".codex/sessions".to_string(),
            },
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "opencode",
            display_name: "OpenCode",
            unix_binary: "opencode",
            windows_binary: "opencode",
            prompt_prefix: "opencode> ",
            resume_subcommand: None,
            resume_flag: "--session",
            uuid_strategy: UuidCaptureStrategy::StoreWatcher {
                store_dir_hint: ".opencode/sessions".to_string(),
            },
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "aider",
            display_name: "Aider",
            unix_binary: "aider",
            windows_binary: "aider",
            prompt_prefix: "aider> ",
            resume_subcommand: None,
            resume_flag: "--restore-chat-history",
            uuid_strategy: UuidCaptureStrategy::None,
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "cursor-agent",
            display_name: "Cursor CLI",
            unix_binary: "cursor-agent",
            windows_binary: "cursor-agent",
            prompt_prefix: "cursor> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::PreMintedFlag("--session-id".to_string()),
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "grok",
            display_name: "Grok",
            unix_binary: "grok",
            windows_binary: "grok",
            prompt_prefix: "grok> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::PreMintedFlag("--session-id".to_string()),
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "q",
            display_name: "Amazon Q",
            unix_binary: "q",
            windows_binary: "q",
            prompt_prefix: "q> ",
            resume_subcommand: Some("chat"),
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::None,
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "pi",
            display_name: "Pi",
            unix_binary: "pi",
            windows_binary: "pi",
            prompt_prefix: "pi> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::StoreWatcher {
                store_dir_hint: ".pi/sessions".to_string(),
            },
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "omp",
            display_name: "Oh My Pi",
            unix_binary: "omp",
            windows_binary: "omp",
            prompt_prefix: "omp> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::StoreWatcher {
                store_dir_hint: ".omp/sessions".to_string(),
            },
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "cmd",
            display_name: "Command Code",
            unix_binary: "cmd",
            windows_binary: "command-code",
            prompt_prefix: "cmd> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::PreMintedFlag("--session-id".to_string()),
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "hermes",
            display_name: "Hermes",
            unix_binary: "hermes",
            windows_binary: "hermes",
            prompt_prefix: "hermes> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::PreMintedFlag("--session-id".to_string()),
            stuck_detector: detector,
        }),
        Arc::new(BuiltinAgentPlugin {
            id: "muse",
            display_name: "Muse",
            unix_binary: "muse",
            windows_binary: "muse",
            prompt_prefix: "muse> ",
            resume_subcommand: None,
            resume_flag: "--resume",
            uuid_strategy: UuidCaptureStrategy::PreMintedFlag("--session-id".to_string()),
            stuck_detector: detector,
        }),
    ]
}

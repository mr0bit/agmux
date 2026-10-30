//! Zero-overhead `AgentPlugin: Send + Sync` trait and plugin registry (`REQ-PLUGIN-TRAIT-001`).

use std::collections::HashMap;
use std::sync::Arc;

use crate::state_machine::SessionState;

/// Strategy used to capture a CLI tool's native conversation UUID (`REQ-PLUGIN-UUID-CAPTURE-005`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UuidCaptureStrategy {
    /// Pre-mint a UUID v4 and inject it using the specified CLI flag (e.g. `"--session-id"`).
    PreMintedFlag(String),
    /// Watch a filesystem store directory or stdout stream for a newly created session file/token.
    StoreWatcher { store_dir_hint: String },
    /// Tool does not expose a persistent conversation UUID.
    None,
}

/// Output classification signal parsed from an agent's terminal line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginOutputSignal {
    /// No state transition signal detected on this line.
    None,
    /// Agent is actively working (`RUNNING`).
    Working,
    /// Agent is waiting on user confirmation or input (`WAITING_INPUT`).
    WaitingInput,
    /// Agent encountered a rate limit or fatal quota error (`FAILED`).
    RateLimited,
    /// Agent completed its turn (`IDLE`).
    TurnCompleted,
}

impl PluginOutputSignal {
    /// Convert to `Option<SessionState>`.
    pub fn to_session_state(self) -> Option<SessionState> {
        match self {
            Self::None => None,
            Self::Working => Some(SessionState::Running),
            Self::WaitingInput => Some(SessionState::WaitingInput),
            Self::RateLimited => Some(SessionState::Failed),
            Self::TurnCompleted => Some(SessionState::Idle),
        }
    }
}

/// Zero-overhead Rust trait defining an agent CLI adapter (`REQ-PLUGIN-TRAIT-001`).
pub trait AgentPlugin: Send + Sync {
    /// Canonical unique identifier (e.g. `"claude"`, `"agy"`, `"cmd"`).
    fn id(&self) -> &str;

    /// Human-readable display name (e.g. `"Claude Code"`, `"Antigravity"`).
    fn display_name(&self) -> &str;

    /// Default executable binary name on the current host OS.
    fn binary_name(&self) -> &str {
        self.binary_name_for_os(cfg!(windows))
    }

    /// Executable binary name resolved for Windows vs Unix (`cmd` -> `command-code` on Windows).
    fn binary_name_for_os(&self, _is_windows: bool) -> &str;

    /// Interactive prompt prefix marker used for stuck-input detection (e.g. `"> "`, `"claude> "`).
    fn prompt_prefix(&self) -> &str;

    /// Generate command-line arguments for spawning a fresh session.
    fn spawn_args(&self, pre_minted_uuid: Option<&str>, extra_args: &[String]) -> Vec<String>;

    /// Generate command-line arguments for resuming an existing session by UUID.
    fn resume_args(&self, session_uuid: &str) -> Vec<String>;

    /// Strategy for capturing the native CLI conversation UUID.
    fn uuid_capture_strategy(&self) -> UuidCaptureStrategy {
        UuidCaptureStrategy::None
    }

    /// Evaluate whether unsubmitted text is stuck after the prompt marker (`REQ-PLUGIN-STUCK-HEURISTIC-002`).
    fn check_stuck(
        &self,
        screen_lines: &[&str],
        last_input_elapsed_ms: u64,
        pty_silence_ms: u64,
        has_active_children: bool,
    ) -> bool;

    /// Zero-allocation hot-loop output parser inspecting a single terminal line (`REQ-PLUGIN-TRAIT-001`).
    fn parse_output(&self, line: &str) -> PluginOutputSignal;
}

/// Thread-safe registry of compiled-in and declarative `AgentPlugin` implementations.
#[derive(Clone, Default)]
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn AgentPlugin>>,
    ordered_ids: Vec<String>,
}

impl PluginRegistry {
    /// Create an empty `PluginRegistry`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a `PluginRegistry` pre-populated with all 14 built-in first-class CLI adapters.
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        for plugin in crate::builtin_plugins::all_builtin_plugins() {
            reg.register(plugin);
        }
        reg
    }

    /// Register or override an `AgentPlugin` by its `id()`.
    pub fn register(&mut self, plugin: Arc<dyn AgentPlugin>) {
        let id = plugin.id().to_string();
        if !self.plugins.contains_key(&id) {
            self.ordered_ids.push(id.clone());
        }
        self.plugins.insert(id, plugin);
    }

    /// Look up a registered plugin by `id`.
    pub fn get(&self, id: &str) -> Option<Arc<dyn AgentPlugin>> {
        self.plugins.get(id).cloned()
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Iterate over all registered plugins in registration order.
    pub fn all(&self) -> Vec<Arc<dyn AgentPlugin>> {
        self.ordered_ids
            .iter()
            .filter_map(|id| self.plugins.get(id).cloned())
            .collect()
    }
}

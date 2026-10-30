//! User-defined declarative agent profiles loaded from `agents.toml` (`REQ-PLUGIN-DECLARATIVE-TOML-004`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::plugin_api::{AgentPlugin, PluginOutputSignal, UuidCaptureStrategy};
use crate::stuck_detector::{DEFAULT_STUCK_TIMEOUT_MS, StuckDetector};

/// Top-level `agents.toml` file structure (`[agents.<id>]`).
#[derive(Debug, Clone, Default)]
pub struct DeclarativeAgentsFile {
    pub agents: HashMap<String, DeclarativeAgentConfig>,
}

/// Configuration for a single user-defined CLI profile in `agents.toml`.
#[derive(Debug, Clone)]
pub struct DeclarativeAgentConfig {
    pub display_name: Option<String>,
    pub binary: String,
    pub prompt_regex: String,
    pub stuck_timeout_ms: u64,
    pub limit_regex: Option<String>,
    pub resume_cmd: Vec<String>,
    pub session_id_flag: Option<String>,
}

impl Default for DeclarativeAgentConfig {
    fn default() -> Self {
        Self {
            display_name: None,
            binary: String::new(),
            prompt_regex: "> ".to_string(),
            stuck_timeout_ms: DEFAULT_STUCK_TIMEOUT_MS,
            limit_regex: None,
            resume_cmd: Vec::new(),
            session_id_flag: None,
        }
    }
}

/// Dynamic `AgentPlugin` backed by a `DeclarativeAgentConfig` from `agents.toml`.
#[derive(Debug, Clone)]
pub struct DeclarativePlugin {
    id: String,
    display_name: String,
    config: DeclarativeAgentConfig,
    stuck_detector: StuckDetector,
}

impl DeclarativePlugin {
    /// Construct a `DeclarativePlugin` from an ID and parsed `DeclarativeAgentConfig`.
    pub fn new(id: String, config: DeclarativeAgentConfig) -> Self {
        let display_name = config.display_name.clone().unwrap_or_else(|| id.clone());
        let stuck_detector = StuckDetector::new(config.stuck_timeout_ms);
        Self {
            id,
            display_name,
            config,
            stuck_detector,
        }
    }

    /// Parse `agents.toml` content into a vector of `Arc<dyn AgentPlugin>`.
    pub fn load_from_toml_str(raw: &str) -> Result<Vec<Arc<dyn AgentPlugin>>, String> {
        let parsed = parse_agents_toml(raw)?;
        let mut list: Vec<Arc<dyn AgentPlugin>> = Vec::with_capacity(parsed.agents.len());
        let mut keys: Vec<String> = parsed.agents.keys().cloned().collect();
        keys.sort();
        for key in keys {
            if let Some(cfg) = parsed.agents.get(&key) {
                list.push(Arc::new(Self::new(key, cfg.clone())));
            }
        }
        Ok(list)
    }

    /// Load declarative agent plugins from a TOML file on disk.
    pub fn load_from_path(path: &Path) -> Result<Vec<Arc<dyn AgentPlugin>>, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        Self::load_from_toml_str(&raw)
    }
}

fn unquote_toml_str(val: &str) -> String {
    let trimmed = val.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_toml_string_array(val: &str) -> Result<Vec<String>, String> {
    let trimmed = val.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Err(format!("Expected TOML array, got: {trimmed}"));
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    Ok(inner
        .split(',')
        .map(unquote_toml_str)
        .filter(|s| !s.is_empty())
        .collect())
}

fn parse_agents_toml(raw: &str) -> Result<DeclarativeAgentsFile, String> {
    let mut file = DeclarativeAgentsFile::default();
    let mut current_agent: Option<String> = None;

    for (line_idx, raw_line) in raw.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let section = line[1..line.len() - 1].trim();
            if let Some(agent_id) = section.strip_prefix("agents.") {
                let id = agent_id.trim().to_string();
                if id.is_empty() {
                    return Err(format!(
                        "Empty agent ID in section header on line {}",
                        line_idx + 1
                    ));
                }
                file.agents.entry(id.clone()).or_default();
                current_agent = Some(id);
            } else {
                current_agent = None;
            }
            continue;
        }

        let Some((k, v)) = line.split_once('=') else {
            return Err(format!(
                "Invalid TOML key-value on line {}: {}",
                line_idx + 1,
                line
            ));
        };
        let Some(ref agent_id) = current_agent else {
            continue;
        };
        let cfg = file.agents.entry(agent_id.clone()).or_default();
        let key = k.trim();
        let val = v.trim();
        match key {
            "display_name" => cfg.display_name = Some(unquote_toml_str(val)),
            "binary" => cfg.binary = unquote_toml_str(val),
            "prompt_regex" => cfg.prompt_regex = unquote_toml_str(val),
            "stuck_timeout_ms" => {
                cfg.stuck_timeout_ms = val.replace('_', "").parse::<u64>().map_err(|e| {
                    format!("Invalid stuck_timeout_ms on line {}: {e}", line_idx + 1)
                })?;
            }
            "limit_regex" => cfg.limit_regex = Some(unquote_toml_str(val)),
            "resume_cmd" => cfg.resume_cmd = parse_toml_string_array(val)?,
            "session_id_flag" => cfg.session_id_flag = Some(unquote_toml_str(val)),
            _ => {}
        }
    }

    for (id, cfg) in &file.agents {
        if cfg.binary.is_empty() {
            return Err(format!("Agent '{id}' is missing required 'binary' field"));
        }
    }

    Ok(file)
}

impl AgentPlugin for DeclarativePlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn binary_name_for_os(&self, _is_windows: bool) -> &str {
        &self.config.binary
    }

    fn prompt_prefix(&self) -> &str {
        &self.config.prompt_regex
    }

    fn spawn_args(&self, pre_minted_uuid: Option<&str>, extra_args: &[String]) -> Vec<String> {
        let mut args = Vec::new();
        if let (Some(flag), Some(uuid)) = (&self.config.session_id_flag, pre_minted_uuid) {
            args.push(flag.clone());
            args.push(uuid.to_string());
        }
        args.extend_from_slice(extra_args);
        args
    }

    fn resume_args(&self, session_uuid: &str) -> Vec<String> {
        if self.config.resume_cmd.is_empty() {
            return vec!["--resume".to_string(), session_uuid.to_string()];
        }
        let mut replaced = false;
        let mut out: Vec<String> = self
            .config
            .resume_cmd
            .iter()
            .map(|part| {
                if part.contains("{session_id}") {
                    replaced = true;
                    part.replace("{session_id}", session_uuid)
                } else {
                    part.clone()
                }
            })
            .collect();
        if !replaced {
            out.push(session_uuid.to_string());
        }
        out
    }

    fn uuid_capture_strategy(&self) -> UuidCaptureStrategy {
        match &self.config.session_id_flag {
            Some(flag) => UuidCaptureStrategy::PreMintedFlag(flag.clone()),
            None => UuidCaptureStrategy::None,
        }
    }

    fn check_stuck(
        &self,
        screen_lines: &[&str],
        last_input_elapsed_ms: u64,
        pty_silence_ms: u64,
        has_active_children: bool,
    ) -> bool {
        let prefix = self
            .config
            .prompt_regex
            .trim_start_matches('^')
            .trim_end_matches('$');
        self.stuck_detector.is_stuck(
            prefix,
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
        if let Some(ref limit_pat) = self.config.limit_regex {
            for pat in limit_pat.split('|') {
                let p = pat.trim();
                if !p.is_empty() && trimmed.contains(p) {
                    return PluginOutputSignal::RateLimited;
                }
            }
        }
        let prefix = self
            .config
            .prompt_regex
            .trim_start_matches('^')
            .trim_end_matches('$')
            .trim();
        if !prefix.is_empty() && trimmed.starts_with(prefix) {
            return PluginOutputSignal::WaitingInput;
        }
        PluginOutputSignal::None
    }
}

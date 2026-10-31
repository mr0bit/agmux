//! Dynamic configuration hot-reload watcher (`agmux.toml`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::journal_watcher::JournalWatcher;

/// Dynamically reloadable daemon runtime configuration parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub idle_after_ms: u64,
    pub blocked_settle_ms: u64,
    pub fts_retention_lines: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        let idle_after_ms = std::env::var("AGMUX_STUCK_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(120_000);
        Self {
            idle_after_ms,
            blocked_settle_ms: 500,
            fts_retention_lines: 10_000,
        }
    }
}

impl RuntimeConfig {
    /// Parse `RuntimeConfig` overrides from TOML text.
    pub fn parse_toml(content: &str) -> Self {
        let mut cfg = Self::default();
        for raw_line in content.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                let val = v.trim().trim_matches('"');
                match key {
                    "idle_after_ms" | "stuck_timeout_ms" => {
                        if let Ok(n) = val.parse::<u64>() {
                            cfg.idle_after_ms = n;
                        }
                    }
                    "blocked_settle_ms" => {
                        if let Ok(n) = val.parse::<u64>() {
                            cfg.blocked_settle_ms = n;
                        }
                    }
                    "fts_retention_lines" | "scrollback" => {
                        if let Ok(n) = val.parse::<usize>() {
                            cfg.fts_retention_lines = n;
                        }
                    }
                    _ => {}
                }
            }
        }
        cfg
    }
}

/// Read the configured notification hook as a TOML basic or literal string.
pub fn notification_hook(content: &str) -> Option<String> {
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "notification_hook" {
            continue;
        }
        let value = value.trim_start();
        if let Some(rest) = value.strip_prefix('\'') {
            let (literal, tail) = rest.split_once('\'')?;
            if tail.trim().is_empty() || tail.trim_start().starts_with('#') {
                return Some(literal.to_owned());
            }
            return None;
        }
        if value.starts_with('"') {
            let mut escaped = false;
            for (i, c) in value.char_indices().skip(1) {
                if c == '"' && !escaped {
                    let tail = value[i + 1..].trim();
                    if !tail.is_empty() && !tail.starts_with('#') {
                        return None;
                    }
                    return serde_json::from_str(&value[..=i]).ok();
                }
                if c == '\\' {
                    escaped = !escaped;
                } else {
                    escaped = false;
                }
            }
        }
        return None;
    }
    None
}

/// Watches `agmux.toml` for changes and hot-reloads `RuntimeConfig` without restarting `agmuxd`.
#[derive(Clone)]
pub struct ConfigWatcher {
    config_path: PathBuf,
    current: Arc<RwLock<RuntimeConfig>>,
}

impl ConfigWatcher {
    /// Create a new `ConfigWatcher` reading from `config_path`.
    pub fn new(config_path: PathBuf) -> Self {
        let initial = std::fs::read_to_string(&config_path)
            .map(|s| RuntimeConfig::parse_toml(&s))
            .unwrap_or_default();
        Self {
            config_path,
            current: Arc::new(RwLock::new(initial)),
        }
    }

    /// Resolve the default configuration path (`AGMUX_CONFIG_PATH` or `agmux.toml`).
    pub fn default_config_path() -> PathBuf {
        if let Some(p) = std::env::var_os("AGMUX_CONFIG_PATH") {
            PathBuf::from(p)
        } else {
            PathBuf::from("agmux.toml")
        }
    }

    /// Read a snapshot of the current `RuntimeConfig`.
    pub fn get(&self) -> RuntimeConfig {
        self.current.read().map(|g| g.clone()).unwrap_or_default()
    }

    /// Force an immediate reload from disk, returning `Some(RuntimeConfig)` if values changed.
    pub fn reload_if_changed(&self, last_raw: &mut Option<String>) -> Option<RuntimeConfig> {
        let content = std::fs::read_to_string(&self.config_path).ok()?;
        if last_raw.as_deref() == Some(content.as_str()) {
            return None;
        }
        *last_raw = Some(content.clone());
        let parsed = RuntimeConfig::parse_toml(&content);
        if let Ok(mut guard) = self.current.write() {
            *guard = parsed.clone();
        }
        Some(parsed)
    }

    /// Spawn a background polling watcher that hot-reloads `config_path` and emits `config.reloaded`.
    pub fn spawn_watch_loop(&self, journal: JournalWatcher) -> tokio::task::JoinHandle<()> {
        let watcher = self.clone();
        let mut last_raw = std::fs::read_to_string(&watcher.config_path).ok();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(40));
            loop {
                interval.tick().await;
                if let Some(cfg) = watcher.reload_if_changed(&mut last_raw) {
                    let payload = format!(
                        "idle_after_ms={},blocked_settle_ms={},fts_retention_lines={}",
                        cfg.idle_after_ms, cfg.blocked_settle_ms, cfg.fts_retention_lines
                    );
                    let _ = journal
                        .publish_event("config.reloaded", None, &payload)
                        .await;
                }
            }
        })
    }

    /// Path being watched.
    pub fn path(&self) -> &Path {
        &self.config_path
    }
}

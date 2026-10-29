//! Creation of a first-run configuration without overwriting user settings.
use crate::{
    AgmuxError,
    agent_discovery::{DiscoveredAgent, discover_installed_agents},
};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Generate a TOML configuration describing the detected agents.
pub fn generate_default_config(agents: &[DiscoveredAgent]) -> String {
    let mut config = String::from(
        "[terminal]\nscrollback = 10000\n\n[keybindings]\npalette = \"F1\"\nquick_prompt = \"Space\"\n\n",
    );
    for agent in agents.iter().filter(|agent| agent.available) {
        // JSON string escaping is also valid for these TOML basic strings.
        config.push_str(&format!(
            "[[agents]]\nname = {}\nbinary = {}\n",
            quote(&agent.name),
            quote(&agent.binary_name)
        ));
        if let Some(path) = &agent.path {
            config.push_str(&format!("path = {}\n", quote(&path.to_string_lossy())));
        }
        config.push('\n');
    }
    config
}

fn quote(value: &str) -> String {
    let mut result = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c.is_control() => result.push_str(&format!("\\u{:04X}", c as u32)),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

/// Create the configuration at `path` if absent, returning its path.
pub fn ensure_config_exists(path: &Path) -> Result<PathBuf, AgmuxError> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            if let Err(error) =
                file.write_all(generate_default_config(&discover_installed_agents()).as_bytes())
            {
                drop(file);
                let _ = std::fs::remove_file(path);
                return Err(error.into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && path.is_file() => {}
        Err(error) => return Err(error.into()),
    }
    Ok(path.to_path_buf())
}

# Low-Level Design: Declarative TOML Agent Adapter (`declarative_plugin`)

* **LLD ID:** `LLD-PLUGIN-DECLARATIVE-TOML-004`
* **Upstream Requirement:** `REQ-PLUGIN-DECLARATIVE-TOML-004`
* **Module:** `crates/agmux-core::declarative_plugin`
* **Verification Test:** `TEST-VER-PLUGIN-004`

## 1. Design Overview
`declarative_plugin` parses user-defined CLI profiles from `agents.toml` (`prompt_regex`, `stuck_timeout_ms`, `limit_regex`, `resume_cmd`) and implements `AgentPlugin` dynamically without requiring recompilation.

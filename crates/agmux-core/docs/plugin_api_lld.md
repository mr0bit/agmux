# Low-Level Design: AgentPlugin Trait & Registry (`plugin_api`)

* **LLD ID:** `LLD-PLUGIN-TRAIT-001`
* **Upstream Requirement:** `REQ-PLUGIN-TRAIT-001`
* **Module:** `crates/agmux-core::plugin_api`
* **Verification Test:** `TEST-VER-PLUGIN-001`

## 1. Design Overview
`plugin_api` defines the zero-allocation `AgentPlugin: Send + Sync` trait and `PluginRegistry` in `crates/agmux-core/src/plugin_api.rs`, exposing hooks for command generation (`spawn_args`), resume command generation (`resume_args`), session UUID capture strategy (`uuid_capture_strategy`), stuck prompt evaluation (`check_stuck`), and output parsing (`parse_output`).

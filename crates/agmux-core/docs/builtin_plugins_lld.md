# Low-Level Design: 14 First-Class Compiled Agent Adapters (`builtin_plugins`)

* **LLD ID:** `LLD-PLUGIN-BUILTIN-ADAPTERS-003`
* **Upstream Requirement:** `REQ-PLUGIN-BUILTIN-ADAPTERS-003`
* **Module:** `crates/agmux-core::builtin_plugins`
* **Verification Test:** `TEST-VER-PLUGIN-003`

## 1. Design Overview
`builtin_plugins` provides compiled-in `AgentPlugin` implementations for all 14 first-class CLI tools (`claude`, `agy`, `gemini`, `codex`, `opencode`, `aider`, `cursor-agent`, `grok`, `q`, `pi`, `omp`, `cmd`, `hermes`, `muse`), including OS-aware binary name resolution (`binary_name_for_os`) and Windows `System32\cmd.exe` collision protection (`is_windows_system_cmd_collision`).

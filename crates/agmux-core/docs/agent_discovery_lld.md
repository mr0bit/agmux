# Agent discovery and first-run configuration

`LLD-AGENT-DISCOVERY-001`: Discovery searches PATH in order for six supported executable names, checks executable permissions on Unix, and returns both found and missing entries. Configuration generation emits terminal defaults, keybindings and detected agent paths as TOML. Creation uses exclusive file creation so existing user settings are preserved. The CLI init command operates without starting the daemon.

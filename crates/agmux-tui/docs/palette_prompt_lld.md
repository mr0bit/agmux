# Palette and prompt tracer design

The command palette filters action labels case-insensitively and dispatches actions through the existing daemon connection.

## LLD-TUI-QUICK-PROMPT-003

Space opens a composer. An unchanged reply sends its text and newline to the selected session's PTY. Tab cycles reply, Codex and Claude. Ctrl+L cycles the chosen tool's model; Ctrl+X cycles reasoning effort; Ctrl+T toggles an isolated worktree. Choosing a spawn modifier while reply is selected chooses Codex, because a running process cannot have its launch configuration changed. Enter constructs a real spawn request with the chosen executable, tool-specific model and effort arguments, prompt and optional worktree branch. Escape cancels. Spawn errors are displayed in the status bar. `tests/e2e_tui_workspace_hierarchy_test.rs` covers the modifier request and `tests/e2e_tui_palette_prompt_test.rs` covers an ordinary reply.

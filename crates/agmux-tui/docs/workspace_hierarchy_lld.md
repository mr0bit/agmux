# Workspace hierarchy design

## LLD-TUI-WORKSPACE-HIERARCHY-009

The daemon persists a workspace metadata row per session: project, slash-separated nested group path, favorite and archived flags. Missing rows default to the session working directory, an empty group and clear flags. Metadata is independent of lifecycle state, so archive and restore do not stop or revive a process. Removing a session removes its workspace row. Authenticated workspace requests list all persisted sessions or replace one session's metadata; responses carry a snapshot or an explicit error. Changes broadcast a session-list notification so other clients reload the snapshot.

The sidebar sorts visible sessions by favorite, project, group and identity. Favorite sessions appear once under Favorites; the heading is absent when no visible favorite exists. Group components render as nested headings. `g` edits a slash-separated group path, `*` toggles the favorite, `a` archives, `t` switches archive views, `u` restores, and `D` toggles three-line rows. `T` opens a shell sub-tab. The same ordered indices drive rendering and keyboard navigation, and snapshot replacement resolves selection and pinned selection by session ID. The list scrolls to the selected row. Search and attention filters apply before ordering.

The `f` menu requests either a standalone fork or a child whose ID includes its parent. The daemon validates the parent, captured native conversation ID, adapter and child identity, then passes a real fork command through the normal PTY spawn pipeline. Claude uses `--resume ID --fork-session`; Codex uses `fork ID`. Unsupported adapters and missing IDs return errors without spawning. Fresh Claude launches receive a pre-minted native ID, persisted with their session; other adapters require an already captured native ID. Forks inherit the parent's directory and workspace organization but are neither archived nor favorites.

Verification: `crates/agmux-tui/tests/workspace_hierarchy_test.rs` checks the view and navigation; `tests/e2e_tui_workspace_hierarchy_test.rs` exercises authenticated IPC, persistence, archive/restore, fork dispatch and prompt requests.

# Static Architecture — Agmux

## 1. System Overview & Scope
**Agmux** is a lightweight, cross-platform agentic terminal multiplexer and session orchestrator written in Rust. It utilizes a decoupled Client-Daemon topology where `agmuxd` directly manages OS pseudo-terminals and maintains in-memory terminal emulation state, while frontends (`agmux-tui`, `agmux-cli`, and `agmux-gui`) connect over a framed IPC transport.

---

## 2. Layered Architecture

```mermaid
flowchart TD
    subgraph Layer1["Layer 1: Stateless Frontend Clients"]
        TUI["agmux-tui (Ratatui 0.30 + Crossterm 0.29)"]
        CLI["agmux CLI (Subcommands & Agent Context)"]
        GUI["agmux-gui (Phase 3: Tauri v2 + Svelte 5)"]
    end

    subgraph Layer2["Layer 2: Framed Transport & IPC (agmux-proto)"]
        IPC["Framed IPC Transport (Bincode 2.0)"]
        AUTH["Token Handshake (daemon.token)"]
        SOCKET["Unix Domain Sockets / Windows Named Pipes / SSH Stdio"]
    end

    subgraph Layer3["Layer 3: Core Daemon Subsystems (agmuxd & agmux-core)"]
        SUPER["Process Supervisor & Group Isolation (setpgid)"]
        PTY["PTY Manager (portable-pty POSIX / ConPTY)"]
        VT["VT100 State Machine (vt100 10k-line grid)"]
        STATE["4-Signal State Classifier & AgentPlugin Trait"]
        GIT["Worktree Manager & Diff Review Engine (gix/git2)"]
        ORCH["Orchestration Bus (Locks, Reservations, Queues)"]
    end

    subgraph Layer4["Layer 4: Storage & Persistence (agmux-core)"]
        SQLITE["SQLite WAL Database (state.db)"]
        FTS["FTS5 Trigram Search (terminal_logs_fts)"]
        ZSTD["zstd Level 3 ANSI Snapshot Compressor"]
        JOURNAL["Event Ring Buffer (event_journal)"]
    end

    Layer1 --> Layer2
    Layer2 --> Layer3
    Layer3 --> Layer4
```

---

## 3. High-Level Component Topology

```mermaid
flowchart LR
    Client["Client Binary (agmux)"] -- "Framed Bincode (IPC)" --> Daemon["Daemon Binary (agmuxd)"]
    
    subgraph agmuxd["agmuxd Daemon"]
        direction TB
        Listener["IPC Listener + Auth Gate"]
        Router["IPC Request Router"]
        
        subgraph Subsystems["Core Subsystems"]
            PTYPool["PTY Pool (portable-pty)"]
            VTGrid["VT100 Screen Grids"]
            Classifier["4-Signal State Classifier"]
            PluginHost["AgentPlugin Registry"]
            LockTable["Resource & File Locks"]
            WorktreeEng["Git Worktree Engine"]
        end
        
        subgraph Store["Storage Layer"]
            DB[(SQLite WAL)]
            FTSIdx[("FTS5 Trigram Index")]
        end
    end
    
    Listener --> Router
    Router --> Subsystems
    Subsystems --> Store
    PTYPool --> OSProcesses["Child OS Processes (Agent CLIs)"]
```

---

## 4. Third-Party Integrations & Dependencies

| Crate / Library | Purpose in Agmux | Justification & Architectural Role |
| :--- | :--- | :--- |
| `portable-pty` | Pseudo-terminal allocation | Cross-platform abstraction over POSIX `openpty`/`forkpty` and Windows ConPTY without requiring external tools like tmux. |
| `vt100` / `libghostty-vt` | In-memory terminal parsing | Parses raw ANSI stream into a deterministic 2D character matrix and manages 10,000-line scrollback. |
| `ratatui` (0.30) + `crossterm` (0.29) | TUI client rendering | Modern, high-performance terminal UI renderer supporting mouse events, alternative screens, and sub-millisecond redraws. |
| `tui-term` | Terminal widget | Renders `vt100::Screen` instances directly inside Ratatui layout chunks. |
| `interprocess` | Local IPC transport | Fast, cross-platform Unix domain socket and Windows Named Pipe abstractions. |
| `bincode` (2.0) | IPC frame serialization | Zero-overhead binary serialization for client-daemon IPC frames. |
| `rusqlite` (bundled) | Single-writer storage | High-performance embedded SQLite database operating in WAL mode with trigram FTS5 extensions. |
| `zstd` | Dead-session compression | Compresses 10,000-line ANSI scrollback to <60 KB in <2ms, enabling 0 MB RAM dead sessions. |
| `gix` (gitoxide) / `git2` | Git operations | Pure-Rust Git worktree creation, branch checkout, `.worktreeinclude` handling, and hunk diffing. |
| `nucleo` | Fuzzy search engine | High-performance multithreaded fuzzy matching driving the Command Palette (`Ctrl+Shift+P`) and search modal. |
| `sysinfo` | Process tree inspection | Inspects child PID trees (cargo, git, npm, pytest) to maintain `WORKING` status during silent build phases. |
| `aho-corasick` | SIMD string matching | Multi-pattern terminal tail scanning for Braille spinners, limit banners, and blocked prompts. |
| `notify` | File watching | Hot-reloads `config.toml` and detects session store changes without daemon restarts. |

---

## 5. Core Domain Class & Struct Diagram

```mermaid
classDiagram
    class Session {
        +SessionId id
        +Option~SessionId~ parent_session_id
        +String title
        +String agent_tool
        +Option~Uuid~ cli_session_uuid
        +SessionState state
        +PathBuf cwd
        +Option~PathBuf~ worktree_path
        +Option~String~ worktree_branch
        +Option~String~ start_git_sha
        +Option~Vec~u8~~ ansi_snapshot
    }

    class SessionState {
        <<enumeration>>
        Working
        Waiting
        Stuck
        Finished
        Idle
        Errored
        Dead
    }

    class AgentPlugin {
        <<interface>>
        +name() String
        +build_spawn_command(config) Command
        +build_resume_command(uuid) Command
        +extract_session_id(store, output) Option~Uuid~
        +check_stuck(screen, last_input_time, silence) bool
        +parse_custom_status(screen) Option~SessionState~
    }

    class PtyHandle {
        +Box~MasterPty~ master
        +Box~Child~ child_process
        +u32 pid
        +u32 process_group_id
        +Sender~Vec~u8~~ stdin_tx
    }

    class VtScreenBuffer {
        +vt100::Parser parser
        +usize max_scrollback
        +get_screen_sync_frame() ScreenSyncFrame
        +get_ansi_snapshot() Vec~u8~
    }

    class LockManager {
        +DashMap~String, MutexQueue~ resource_locks
        +DashMap~PathBuf, SessionId~ file_reservations
        +acquire_lock(name, session_id, wait) Result
        +release_lock(name, session_id)
        +evict_session(session_id)
    }

    Session --> SessionState
    Session ..> PtyHandle : owns when active
    Session ..> VtScreenBuffer : owns when active
    AgentPlugin <|.. BuiltinClaudePlugin : implements
    AgentPlugin <|.. BuiltinAgyPlugin : implements
    AgentPlugin <|.. DeclarativeTomlPlugin : implements
```

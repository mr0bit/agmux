//! Progressive command reference for agent consumers.

/// Compact discovery text; detailed command help is available on demand.
pub const OVERVIEW: &str = "agmux commands: panels, open, gui, list, run, send, input, task, report, lock, reserve, release, reservations, watch, review, diff, search, freeze, inspect, remove, internal-hook, init, tui, connect, remote, doctor, daemon, agent-context, mcp, archive-self, kill-self, cancel-teardown. Use `agmux agent-context --command NAME` for usage and `agmux NAME --help` for options. JSON output is available on list, task list, report view, lock list, reservations, watch, diff, search, remote list and doctor. Session-aware commands accept --session or AGMUX_SESSION_ID.";

/// Examples supplement clap's authoritative command usage.
pub fn examples(command: &str) -> &'static str {
    match command {
        "panels" => "Example: agmux panels open src/main.rs --session worker-1",
        "open" => "Example: agmux open agmux://session/worker-1",
        "gui" => "Examples: agmux gui; agmux gui --snapshot --json",
        "task" => {
            "Examples:\n  agmux task create T-101 'Implement feature' --description 'Details'\n  agmux task claim T-101 --session worker-1\n  agmux task finish T-101 --session worker-1\n  agmux task list --json\nJSON example: [{\"id\":\"T-101\",\"title\":\"Implement feature\",\"description\":\"Details\",\"status\":\"done\",\"claimed_by\":\"worker-1\",\"created_at\":0,\"updated_at\":0}]"
        }
        "lock" => {
            "Examples:\n  agmux lock acquire db_migrate --session worker-1 --wait\n  agmux lock release db_migrate --session worker-1\n  agmux lock list --json\nJSON example: [{\"name\":\"db_migrate\",\"holder_session_id\":\"worker-1\",\"acquired_at\":0}]"
        }
        "list" => {
            "Example: agmux list --json\nJSON example: [{\"session_id\":\"worker-1\",\"pid\":123,\"cwd\":\"/repo\",\"command\":\"sh\",\"created_at\":0,\"state\":\"RUNNING\"}]"
        }
        "report" => {
            "Examples: agmux report post T-101 --summary 'Done' --session worker-1; agmux report view T-101 --json\nJSON example: [{\"task_id\":\"T-101\",\"session_id\":\"worker-1\",\"summary\":\"Done\",\"pr_url\":null,\"commit_sha\":null,\"created_at\":0}]"
        }
        "reserve" => "Example: agmux reserve crates/core/src --session worker-1",
        "release" => "Example: agmux release crates/core/src --session worker-1",
        "send" => "Example: agmux send worker-1 'Please continue'",
        "watch" => "Example: agmux watch --as orchestrator --follow --json",
        "run" => "Examples: agmux run sh -c 'echo hello'; agmux run --enable-mcp codex",
        "mcp" => {
            "Example request: {\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"task_list\",\"arguments\":{}}}"
        }
        "archive-self" => "Example: agmux archive-self worker-1",
        "kill-self" => "Example: agmux kill-self worker-1",
        "cancel-teardown" => "Example: agmux cancel-teardown worker-1",
        "input" => "Example: agmux input worker-1 'hello'",
        "reservations" => {
            "Example: agmux reservations --json\nJSON example: [{\"path\":\"src\",\"holder_session_id\":\"worker-1\",\"acquired_at\":0}]"
        }
        "review" => {
            "Examples: agmux review comment worker-1 --file src/main.rs --line 1 --content 'Check'; agmux review resolve COMMENT_ID --session worker-1"
        }
        "search" => "Example: agmux search 'error' --session-id worker-1 --limit 20 --json",
        "diff" => "Example: agmux diff worker-1 --scope uncommitted --json",
        "freeze" => "Example: agmux freeze worker-1",
        "inspect" => "Example: agmux inspect worker-1",
        "remove" => "Example: agmux remove worker-1",
        "internal-hook" => "Example: agmux internal-hook --session worker-1 --event stop",
        "remote" => "Examples: agmux remote connect ssh://host; agmux remote list --json",
        "connect" => "Example: agmux connect ssh://host",
        "doctor" => "Example: agmux doctor --json",
        "daemon" => "Example: agmux daemon repair",
        "init" => "Example: agmux init",
        "tui" => "Example: agmux tui",
        "agent-context" => "Example: agmux agent-context --command task",
        _ => "Use --help on a nested subcommand for its options.",
    }
}

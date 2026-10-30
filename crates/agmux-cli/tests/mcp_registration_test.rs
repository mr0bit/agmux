use agmux_cli::mcp_server::agent_arguments;
use std::path::Path;

#[test]
fn opt_in_registration_preserves_agent_arguments_and_transport_paths() {
    for agent in ["claude", "codex"] {
        let args = agent_arguments(
            agent,
            vec!["hello".into()],
            Path::new("/tmp/agmux.sock"),
            Path::new("/tmp/agmux.token"),
        )
        .unwrap();
        let joined = args.join(" ");
        assert!(
            joined.contains("agmux.sock")
                && joined.contains("agmux.token")
                && joined.contains("mcp")
        );
        assert_eq!(args.last().unwrap(), "hello");
    }
    let relative = agent_arguments("claude", Vec::new(), Path::new("sock"), Path::new("token"))
        .unwrap()
        .join(" ");
    assert!(
        relative.contains(
            &std::env::current_dir()
                .unwrap()
                .join("sock")
                .display()
                .to_string()
        )
    );
    assert!(agent_arguments("other", Vec::new(), Path::new("sock"), Path::new("token")).is_err());
}

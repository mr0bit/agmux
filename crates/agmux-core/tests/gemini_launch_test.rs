use agmux_core::agent_discovery::{discover_all_builtin_agents_in_paths, windows_gemini_launch};
use std::path::PathBuf;

struct Installation(PathBuf);
impl Installation {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agmux-gemini-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("node_modules/@google/gemini-cli/bundle")).unwrap();
        std::fs::write(root.join("gemini.cmd"), "npm shim").unwrap();
        std::fs::write(root.join("gemini"), "POSIX shim, not a Windows executable").unwrap();
        std::fs::write(root.join("node.exe"), "node executable").unwrap();
        std::fs::write(
            root.join("node_modules/@google/gemini-cli/package.json"),
            r#"{"bin":{"gemini":"bundle/gemini.js"}}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("node_modules/@google/gemini-cli/bundle/gemini.js"),
            "// entry",
        )
        .unwrap();
        Self(root)
    }
}
impl Drop for Installation {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn discovers_npm_shim_and_launches_official_entry_without_a_command_shell() {
    let installation = Installation::new();
    let dirs = vec![installation.0.clone()];
    let found = discover_all_builtin_agents_in_paths(&dirs, true)
        .into_iter()
        .find(|p| p.binary_name == "gemini")
        .unwrap();
    assert!(found.available);
    assert_eq!(found.path, Some(installation.0.join("gemini.cmd")));
    let launch = windows_gemini_launch("gemini", &dirs).unwrap().unwrap();
    assert_eq!(launch.command, installation.0.join("node.exe"));
    #[cfg(windows)]
    assert!(!launch.args[0].starts_with(r"\\?\"), "Node must receive an ordinary Windows entry path");
    let entry = installation
        .0
        .join("node_modules/@google/gemini-cli/bundle/gemini.js")
        .canonicalize()
        .unwrap();
    assert_eq!(
        launch.args,
        vec![dunce::simplified(&entry).to_string_lossy().into_owned()]
    );
}

#[test]
fn prefers_native_executables_and_leaves_other_providers_unchanged() {
    let installation = Installation::new();
    std::fs::write(installation.0.join("gemini.exe"), "native executable").unwrap();
    let launch = windows_gemini_launch("gemini", std::slice::from_ref(&installation.0))
        .unwrap()
        .unwrap();
    assert_eq!(launch.command, installation.0.join("gemini.exe"));
    assert!(launch.args.is_empty());
    assert!(windows_gemini_launch("codex", &[]).unwrap().is_none());
}

#[test]
fn rejects_missing_runtime_and_out_of_package_entry_points() {
    let installation = Installation::new();
    std::fs::remove_file(installation.0.join("node.exe")).unwrap();
    assert!(
        windows_gemini_launch("gemini", std::slice::from_ref(&installation.0))
            .err()
            .unwrap()
            .to_string()
            .contains("Node.js")
    );
    std::fs::write(
        installation
            .0
            .join("node_modules/@google/gemini-cli/package.json"),
        r#"{"bin":{"gemini":"../outside.js"}}"#,
    )
    .unwrap();
    std::fs::write(
        installation.0.join("node_modules/@google/outside.js"),
        "// outside",
    )
    .unwrap();
    assert!(
        windows_gemini_launch("gemini", std::slice::from_ref(&installation.0))
            .err()
            .unwrap()
            .to_string()
            .contains("escapes")
    );
}

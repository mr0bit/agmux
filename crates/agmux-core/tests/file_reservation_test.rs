use agmux_core::storage_engine::{StorageEngine, path_conflicts};
use std::path::Path;

#[test]
fn test_hierarchical_path_conflicts() {
    // Exact match
    assert!(path_conflicts("src/lib.rs", "src/lib.rs"));

    // Ancestor directory conflicts with nested child
    assert!(path_conflicts(
        "crates/core/src",
        "crates/core/src/state.rs"
    ));
    assert!(path_conflicts(
        "crates/core/src/",
        "crates/core/src/state.rs"
    ));
    assert!(path_conflicts(
        "crates/core/src/state.rs",
        "crates/core/src"
    ));

    // Disjoint siblings do not conflict
    assert!(!path_conflicts("src/lib.rs", "src/main.rs"));
    assert!(!path_conflicts("crates/core/src", "crates/cli/src"));

    // Prefix lookalike without path boundary does NOT conflict
    assert!(!path_conflicts("src", "src_other/file.rs"));
    assert!(!path_conflicts("app", "application/main.rs"));
}

#[test]
fn test_file_reservation_prefix_exclusion_and_release() {
    let storage = StorageEngine::open(Path::new(":memory:")).expect("open storage");

    // Agent A reserves "crates/core/src/"
    let res_a = storage
        .reserve_file("crates/core/src/", "agent-a")
        .expect("reserve");
    assert!(
        res_a.is_ok(),
        "Agent A should successfully reserve directory"
    );

    // Agent B attempts to reserve nested "crates/core/src/state.rs" -> CONFLICT!
    let res_b = storage
        .reserve_file("crates/core/src/state.rs", "agent-b")
        .expect("reserve");
    assert!(
        res_b.is_err(),
        "Agent B's nested reservation must be rejected"
    );
    let (conflict_path, holder) = res_b.unwrap_err();
    assert_eq!(conflict_path, "crates/core/src");
    assert_eq!(holder, "agent-a");

    // Agent A can re-reserve or reserve nested path (idempotent / same holder)
    let res_a_nested = storage
        .reserve_file("crates/core/src/state.rs", "agent-a")
        .expect("nested");
    assert!(res_a_nested.is_ok());

    // Agent B reserves non-overlapping path "crates/cli/src/main.rs" -> SUCCESS!
    let res_b_disjoint = storage
        .reserve_file("crates/cli/src/main.rs", "agent-b")
        .expect("disjoint");
    assert!(res_b_disjoint.is_ok());

    // Agent A releases "crates/core/src"
    let released = storage
        .release_file("crates/core/src", "agent-a")
        .expect("release");
    assert!(released);

    // After release, Agent B can now reserve "crates/core/src/state.rs"
    let res_b_now = storage
        .reserve_file("crates/core/src/state.rs", "agent-b")
        .expect("reserve after release");
    assert!(res_b_now.is_ok());

    // Test eviction of Agent B's reservations
    let evicted = storage
        .release_session_reservations("agent-b")
        .expect("evict");
    assert_eq!(evicted.len(), 2);
    assert!(storage.list_reservations().unwrap().is_empty());
}

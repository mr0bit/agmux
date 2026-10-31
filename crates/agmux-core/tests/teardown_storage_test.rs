use agmux_core::storage_engine::StorageEngine;
use agmux_proto::SessionInfo;
use std::path::Path;

#[test]
fn teardown_is_consumed_and_late_transitions_cannot_revive() {
    let db = StorageEngine::open(Path::new(":memory:")).unwrap();
    db.insert_session(&SessionInfo {
        session_id: "s".into(),
        pid: 1,
        cwd: "/".into(),
        command: "agent".into(),
        created_at: 0,
        state: "RUNNING".into(),
    })
    .unwrap();
    db.set_teardown("s", Some("archive")).unwrap();
    assert_eq!(db.list_teardowns().unwrap()["s"], "archive");
    db.finish_teardown("s", "ARCHIVED").unwrap();
    assert!(db.list_teardowns().unwrap().is_empty());
    assert!(!db.update_live_session_state("s", "COMPLETED").unwrap());
    assert_eq!(db.get_session("s").unwrap().unwrap().state, "ARCHIVED");
    assert!(db.list_active_sessions().unwrap().is_empty());
}

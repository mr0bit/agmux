use agmux_core::storage_engine::StorageEngine;
use std::path::Path;

#[test]
fn test_event_journal_monotonic_sequences_and_replay() {
    let storage = StorageEngine::open(Path::new(":memory:")).expect("open storage");

    // Record 5 events
    let e1 = storage
        .record_event("session_state_changed", Some("s-1"), "RUNNING -> IDLE")
        .unwrap();
    let e2 = storage
        .record_event("lock_acquired", Some("s-2"), "db_migrate")
        .unwrap();
    let e3 = storage
        .record_event("session_state_changed", Some("s-1"), "IDLE -> RUNNING")
        .unwrap();
    let e4 = storage
        .record_event("files_reserved", Some("s-3"), "src/")
        .unwrap();
    let e5 = storage
        .record_event("session_exit", Some("s-1"), "exit_code: 0")
        .unwrap();

    assert_eq!(e1.seq, 1);
    assert_eq!(e2.seq, 2);
    assert_eq!(e3.seq, 3);
    assert_eq!(e4.seq, 4);
    assert_eq!(e5.seq, 5);

    // Initial cursor query returns 0
    assert_eq!(storage.get_cursor("test_cursor").unwrap(), 0);

    // Read since seq 0
    let all = storage.read_events_since(0, None).unwrap();
    assert_eq!(all.len(), 5);
    assert_eq!(all[0].seq, 1);
    assert_eq!(all[4].seq, 5);

    // Advance cursor to seq 2
    storage.set_cursor("test_cursor", 2).unwrap();
    assert_eq!(storage.get_cursor("test_cursor").unwrap(), 2);

    // Read since cursor position (seq 2)
    let missed = storage.read_events_since(2, None).unwrap();
    assert_eq!(missed.len(), 3);
    assert_eq!(missed[0].seq, 3);
    assert_eq!(missed[1].seq, 4);
    assert_eq!(missed[2].seq, 5);

    // Advance cursor to seq 5
    storage.set_cursor("test_cursor", 5).unwrap();
    let empty = storage.read_events_since(5, None).unwrap();
    assert!(empty.is_empty());
}

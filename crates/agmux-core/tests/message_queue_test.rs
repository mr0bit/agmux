use agmux_core::storage_engine::StorageEngine;
use std::path::Path;

#[test]
fn queued_messages_are_isolated_ordered_and_claimed_once() {
    let db = StorageEngine::open(Path::new(":memory:")).expect("open database");
    let first = db.enqueue_message("a", "first").expect("enqueue");
    db.enqueue_message("b", "other").expect("enqueue");
    let second = db.enqueue_message("a", "second").expect("enqueue");
    assert_eq!(db.pending_message_count("a").unwrap(), 2);
    assert_eq!(
        db.pop_pending_messages("a").unwrap(),
        vec![(first, "first".into()), (second, "second".into())]
    );
    assert!(db.pop_pending_messages("a").unwrap().is_empty());
    assert_eq!(db.pending_message_count("b").unwrap(), 1);
}

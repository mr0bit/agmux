use agmux_core::storage_engine::StorageEngine;
use std::path::Path;

#[test]
fn test_lock_acquire_and_release() {
    let storage = StorageEngine::open(Path::new(":memory:")).expect("open storage");

    // Acquire lock for session 1
    let acquired = storage
        .acquire_lock("db_migrate", "agent-1")
        .expect("acquire");
    assert!(acquired, "Agent 1 should acquire the lock");

    // Reacquiring same lock by same session should succeed
    let reacquire = storage
        .acquire_lock("db_migrate", "agent-1")
        .expect("reacquire");
    assert!(reacquire, "Reacquisition by same holder should return true");

    // Lock contention: Agent 2 attempts to acquire held lock
    let contender = storage
        .acquire_lock("db_migrate", "agent-2")
        .expect("contender");
    assert!(!contender, "Agent 2 should not acquire held lock");

    // Verify holder query
    let holder = storage.get_lock_holder("db_migrate").expect("get holder");
    assert_eq!(holder.as_deref(), Some("agent-1"));

    // Verify lock listing
    let locks = storage.list_locks().expect("list locks");
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].0, "db_migrate");
    assert_eq!(locks[0].1, "agent-1");

    // Agent 2 attempts to release Agent 1's lock -> should fail (0 rows deleted)
    let bad_release = storage
        .release_lock("db_migrate", "agent-2")
        .expect("bad release");
    assert!(!bad_release, "Agent 2 cannot release Agent 1's lock");

    // Agent 1 releases lock
    let released = storage
        .release_lock("db_migrate", "agent-1")
        .expect("release");
    assert!(released, "Agent 1 should release lock successfully");

    // Lock is now free; Agent 2 acquires it
    let acquired2 = storage
        .acquire_lock("db_migrate", "agent-2")
        .expect("acquire2");
    assert!(acquired2, "Agent 2 should now acquire free lock");
}

#[test]
fn test_release_session_locks_eviction() {
    let storage = StorageEngine::open(Path::new(":memory:")).expect("open storage");

    // Session 1 holds multiple locks
    assert!(storage.acquire_lock("lock_a", "sess-1").unwrap());
    assert!(storage.acquire_lock("lock_b", "sess-1").unwrap());
    assert!(storage.acquire_lock("lock_c", "sess-2").unwrap());

    // Evict session 1
    let evicted = storage.release_session_locks("sess-1").unwrap();
    assert_eq!(evicted.len(), 2);
    assert!(evicted.contains(&"lock_a".to_string()));
    assert!(evicted.contains(&"lock_b".to_string()));

    // Verify sess-1 locks are gone
    assert!(storage.get_lock_holder("lock_a").unwrap().is_none());
    assert!(storage.get_lock_holder("lock_b").unwrap().is_none());

    // sess-2 lock is untouched
    assert_eq!(
        storage.get_lock_holder("lock_c").unwrap().as_deref(),
        Some("sess-2")
    );
}

use agmux_core::storage_engine::StorageEngine;

#[test]
fn pages_survive_reopening_and_remain_independent() {
    let path = std::env::temp_dir().join(format!("agmux-notes-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let db = StorageEngine::open(&path).expect("open database");
        db.set_note("session", 2, "second").expect("write page");
        db.set_note("session", 1, "first").expect("write page");
        db.set_note("other", 1, "other")
            .expect("write other session");
        db.set_note("session", 1, "updated").expect("replace page");
    }
    let db = StorageEngine::open(&path).expect("reopen database");
    assert_eq!(
        db.get_note("session", 1).expect("get page"),
        Some("updated".into())
    );
    assert_eq!(db.get_note("session", 3).expect("missing page"), None);
    assert_eq!(
        db.list_notes("session").expect("list pages"),
        vec![(1, "updated".into()), (2, "second".into())]
    );
    assert_eq!(db.search_notes("updated", 10).expect("search").len(), 1);
    assert!(db.search_notes("first", 10).expect("old index").is_empty());
    assert!(
        db.search_notes("\" OR *", 10)
            .expect("literal query")
            .is_empty()
    );
    drop(db);
    let _ = std::fs::remove_file(path);
}

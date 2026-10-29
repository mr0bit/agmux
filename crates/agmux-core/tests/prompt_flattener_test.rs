use agmux_core::prompt_flattener::flatten_review_comments;
use agmux_proto::ReviewComment;

#[test]
fn formats_stable_comment_ids_and_round() {
    let comments = vec![ReviewComment {
        id: "rev_01".into(),
        session_id: "s".into(),
        file_path: "src/file.rs".into(),
        line_number: 42,
        line_hash: "hash".into(),
        content: "Fix memory leak".into(),
        status: "draft".into(),
        created_at: 1,
    }];
    assert_eq!(
        flatten_review_comments(1, &comments),
        "## Code Review Feedback (Round #1)\nPlease address the following comments:\n\n- [ID: rev_01] `src/file.rs:42`: Fix memory leak\n"
    );
}

#[test]
fn persists_comments_and_tracks_rounds_per_session() {
    use agmux_core::storage_engine::StorageEngine;
    use agmux_proto::SessionInfo;
    let db = StorageEngine::open(std::path::Path::new(":memory:")).unwrap();
    db.insert_session(&SessionInfo {
        session_id: "s".into(),
        pid: 1,
        cwd: ".".into(),
        command: "cat".into(),
        created_at: 1,
        state: "IDLE".into(),
    })
    .unwrap();
    let comment = ReviewComment {
        id: "rev_01".into(),
        session_id: "s".into(),
        file_path: "file".into(),
        line_number: 1,
        line_hash: "h".into(),
        content: "fix".into(),
        status: "draft".into(),
        created_at: 1,
    };
    db.add_review_comment(&comment).unwrap();
    assert_eq!(
        db.list_review_comments("s", Some("draft")).unwrap(),
        vec![comment]
    );
    assert_eq!(db.next_review_round("s").unwrap(), 1);
    assert_eq!(
        db.complete_review_round("s", 1, &["rev_01".into()])
            .unwrap(),
        "IDLE"
    );
    assert_eq!(db.next_review_round("s").unwrap(), 2);
    assert_eq!(db.next_review_round("other").unwrap(), 1);
    assert!(
        db.list_review_comments("s", Some("draft"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(db.list_review_comments("s", Some("sent")).unwrap().len(), 1);
    assert_eq!(db.get_session("s").unwrap().unwrap().state, "WORKING");
}

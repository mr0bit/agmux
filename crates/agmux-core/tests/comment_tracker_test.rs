use agmux_core::comment_tracker::check_auto_resolutions;
use agmux_proto::{FileDiff, ReviewComment};

#[test]
fn resolves_only_sent_comments_for_changed_old_or_new_paths() {
    let comment = |id: &str, path: &str, status: &str| ReviewComment {
        id: id.into(),
        session_id: "s".into(),
        file_path: path.into(),
        line_number: 42,
        line_hash: String::new(),
        content: String::new(),
        status: status.into(),
        created_at: 0,
    };
    let comments = vec![
        comment("a", "old", "sent"),
        comment("b", "new", "sent"),
        comment("c", "new", "draft"),
        comment("d", "other", "sent"),
    ];
    let diffs = vec![FileDiff {
        old_path: Some("old".into()),
        new_path: Some("new".into()),
        hunks: vec![],
    }];
    assert_eq!(check_auto_resolutions(&comments, &diffs), vec!["a", "b"]);
    assert!(check_auto_resolutions(&comments, &[]).is_empty());
}

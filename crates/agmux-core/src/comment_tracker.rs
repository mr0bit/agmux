//! Identify sent comments whose files and hunks appear in an inspected diff.
use agmux_proto::{FileDiff, ReviewComment};

pub fn check_auto_resolutions(comments: &[ReviewComment], diffs: &[FileDiff]) -> Vec<String> {
    comments
        .iter()
        .filter(|comment| {
            comment.status == "sent"
                && diffs.iter().any(|file| {
                    let path_matches = file.old_path.as_deref() == Some(comment.file_path.as_str())
                        || file.new_path.as_deref() == Some(comment.file_path.as_str());
                    if !path_matches {
                        return false;
                    }
                    if file.hunks.is_empty() {
                        return true;
                    }
                    // Check if hunk modifies lines around the commented line (+/- 3 lines)
                    file.hunks.iter().any(|hunk| {
                        let old_end = hunk.old_start + hunk.old_count.max(1);
                        let new_end = hunk.new_start + hunk.new_count.max(1);
                        (comment.line_number >= hunk.old_start.saturating_sub(3)
                            && comment.line_number <= old_end + 3)
                            || (comment.line_number >= hunk.new_start.saturating_sub(3)
                                && comment.line_number <= new_end + 3)
                    })
                })
        })
        .map(|comment| comment.id.clone())
        .collect()
}

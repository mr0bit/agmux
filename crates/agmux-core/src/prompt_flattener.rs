//! Compile review drafts into a single prompt.
use agmux_proto::ReviewComment;

pub fn flatten_review_comments(round_num: u32, comments: &[ReviewComment]) -> String {
    let mut prompt = format!(
        "## Code Review Feedback (Round #{round_num})\nPlease address the following comments:\n\n"
    );
    for comment in comments {
        prompt.push_str(&format!(
            "- [ID: {}] `{}:{}`: {}\n",
            comment.id, comment.file_path, comment.line_number, comment.content
        ));
    }
    prompt
}

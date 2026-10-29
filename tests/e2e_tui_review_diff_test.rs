use agmux_proto::{DiffHunk, DiffLine, DiffLineType, DiffScope, FileDiff, IpcMessage, SessionInfo};
use agmux_tui::{App, ModalState, ui};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|frame| ui::render(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn test_review_diff_mode_and_hunk_navigation() {
    let mut app = App {
        sessions: vec![SessionInfo {
            session_id: "agent-review-session".into(),
            pid: 100,
            cwd: "/tmp/repo".into(),
            command: "codex".into(),
            created_at: 0,
            state: "RUNNING".into(),
        }],
        ..App::default()
    };

    // Press Ctrl+R to open Review Diff
    app.handle_key_with_modifiers(KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(matches!(
        app.pending_message.take(),
        Some(IpcMessage::GetDiffRequest(req)) if req.scope == DiffScope::SessionDiff
    ));

    // Populate mock diff files
    if let ModalState::ReviewDiff { files, .. } = &mut app.modal {
        *files = vec![FileDiff {
            old_path: Some("src/main.rs".into()),
            new_path: Some("src/main.rs".into()),
            hunks: vec![
                DiffHunk {
                    old_start: 1,
                    old_count: 3,
                    new_start: 1,
                    new_count: 4,
                    header: "@@ -1,3 +1,4 @@".into(),
                    lines: vec![
                        DiffLine {
                            kind: DiffLineType::Context,
                            content: "fn main() {".into(),
                            old_line: Some(1),
                            new_line: Some(1),
                        },
                        DiffLine {
                            kind: DiffLineType::Added,
                            content: "    println!(\"Review mode active!\");".into(),
                            old_line: None,
                            new_line: Some(2),
                        },
                    ],
                },
                DiffHunk {
                    old_start: 10,
                    old_count: 2,
                    new_start: 11,
                    new_count: 2,
                    header: "@@ -10,2 +11,2 @@".into(),
                    lines: vec![DiffLine {
                        kind: DiffLineType::Removed,
                        content: "    old_logic();".into(),
                        old_line: Some(10),
                        new_line: None,
                    }],
                },
            ],
        }];
    }

    let rendered = screen(&app);
    assert!(rendered.contains("Review Diff"));
    assert!(rendered.contains("Session Diff"));
    assert!(rendered.contains("src/main.rs"));
    assert!(rendered.contains("Review mode active!"));

    // Navigate to next hunk
    app.handle_key(KeyCode::Down);
    if let ModalState::ReviewDiff { selected_hunk, .. } = &app.modal {
        assert_eq!(*selected_hunk, 1);
    } else {
        panic!("Expected ModalState::ReviewDiff");
    }

    // Switch scope via Tab to FullUncommitted
    app.handle_key(KeyCode::Tab);
    assert!(matches!(
        app.pending_message.take(),
        Some(IpcMessage::GetDiffRequest(req)) if req.scope == DiffScope::FullUncommitted
    ));

    // Switch scope directly via '4' to LastCommit
    app.handle_key(KeyCode::Char('4'));
    assert!(matches!(
        app.pending_message.take(),
        Some(IpcMessage::GetDiffRequest(req)) if req.scope == DiffScope::LastCommit
    ));

    // Draft a comment with 'c'
    app.handle_key(KeyCode::Char('c'));
    assert!(matches!(
        app.pending_message.take(),
        Some(IpcMessage::AddReviewCommentRequest(req)) if req.file_path == "src/main.rs"
    ));

    // Dispatch review round with 'C'
    app.handle_key(KeyCode::Char('C'));
    assert!(matches!(
        app.pending_message.take(),
        Some(IpcMessage::DispatchReviewRoundRequest(req)) if req.session_id == "agent-review-session"
    ));

    // Close review diff modal via Esc
    app.handle_key(KeyCode::Esc);
    assert_eq!(app.modal, ModalState::None);
}

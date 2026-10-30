use agmux_proto::{IpcMessage, SearchNotesRequest, SearchNotesResponse, SessionInfo};
use agmux_tui::{App, ModalState};
use crossterm::event::{KeyCode, KeyModifiers};

fn session(id: &str) -> SessionInfo {
    SessionInfo {
        session_id: id.into(),
        pid: 1,
        cwd: "/".into(),
        command: "sh".into(),
        created_at: 0,
        state: "RUNNING".into(),
    }
}
fn response(request: SearchNotesRequest, ids: Vec<String>) -> SearchNotesResponse {
    SearchNotesResponse {
        request,
        pages: vec![],
        hits: vec![],
        session_ids: ids,
        error: None,
    }
}
#[test]
fn modifiers_filter_selection_and_stale_responses() {
    let mut app = App {
        sessions: vec![session("a"), session("b")],
        ..App::default()
    };
    app.handle_key_with_modifiers(KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert!(matches!(app.modal, ModalState::GlobalSearch { .. }));
    app.handle_key(KeyCode::Char('x'));
    app.handle_key(KeyCode::Char('y'));
    app.apply_search_notes_response(response(
        SearchNotesRequest::Search { query: "x".into() },
        vec![],
    ));
    assert!(matches!(&app.modal, ModalState::GlobalSearch { query, .. } if query == "xy"));
    app.handle_key(KeyCode::Esc);
    app.handle_key(KeyCode::Char('/'));
    app.handle_key(KeyCode::Char('b'));
    app.apply_search_notes_response(response(
        SearchNotesRequest::Filter { query: "b".into() },
        vec!["b".into()],
    ));
    assert_eq!(app.selected_index, 1);
    app.handle_key(KeyCode::Enter);
    app.handle_key(KeyCode::Down);
    assert_eq!(app.selected_index, 1);
    app.handle_key_with_modifiers(KeyCode::PageDown, KeyModifiers::CONTROL);
    assert!(
        matches!(app.pending_message, Some(IpcMessage::SearchNotesRequest(SearchNotesRequest::Get { ref session_id, page: 1 })) if session_id == "b")
    );
    app.apply_search_notes_response(SearchNotesResponse {
        request: SearchNotesRequest::Get {
            session_id: "b".into(),
            page: 1,
        },
        pages: vec![(1, "hello".into())],
        hits: vec![],
        session_ids: vec![],
        error: None,
    });
    app.handle_key(KeyCode::Char('!'));
    assert!(
        matches!(&app.pending_message, Some(IpcMessage::SearchNotesRequest(SearchNotesRequest::Set { content, .. })) if content == "hello!")
    );
    app.handle_key_with_modifiers(KeyCode::PageDown, KeyModifiers::CONTROL);
    assert!(matches!(app.modal, ModalState::Notes { page: 2, .. }));
}

#[test]
fn failed_load_cannot_overwrite_and_preview_scrolls_to_selected_hit() {
    let mut app = App {
        sessions: vec![session("a")],
        ..App::default()
    };
    app.handle_key(KeyCode::Char('N'));
    app.pending_message.take();
    app.apply_search_notes_response(SearchNotesResponse {
        request: SearchNotesRequest::Get {
            session_id: "a".into(),
            page: 1,
        },
        pages: vec![],
        hits: vec![],
        session_ids: vec![],
        error: Some("database unavailable".into()),
    });
    app.handle_key(KeyCode::Char('x'));
    assert!(!matches!(
        app.pending_message,
        Some(IpcMessage::SearchNotesRequest(
            SearchNotesRequest::Set { .. }
        ))
    ));
    app.handle_key(KeyCode::Esc);
    app.handle_key_with_modifiers(KeyCode::Char('f'), KeyModifiers::CONTROL);
    app.handle_key(KeyCode::Char('x'));
    let mut context: Vec<_> = (0..100).map(|n| format!("line {n}")).collect();
    context[90] = "target at line ninety".into();
    app.apply_search_notes_response(SearchNotesResponse {
        request: SearchNotesRequest::Search { query: "x".into() },
        pages: vec![],
        hits: vec![agmux_proto::GlobalSearchHit {
            session_id: "a".into(),
            page: None,
            line_no: 90,
            snippet: "target".into(),
            context,
            context_match: 90,
        }],
        session_ids: vec![],
        error: None,
    });
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24)).unwrap();
    terminal
        .draw(|frame| agmux_tui::ui::render(frame, &app))
        .unwrap();
    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(rendered.contains("target at line ninety"));
}

#[test]
fn density_toggle_keeps_selected_session() {
    let mut app = App {
        sessions: vec![session("a"), session("b")],
        selected_index: 1,
        ..App::default()
    };
    app.handle_key(KeyCode::Char('D'));
    assert!(app.comfortable_rows);
    assert_eq!(app.sessions[app.selected_index].session_id, "b");
    app.handle_key(KeyCode::Char('D'));
    assert!(!app.comfortable_rows);
}

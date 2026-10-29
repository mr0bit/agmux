//! Ratatui layout for the session sidebar and terminal viewport.
use crate::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Render an app into any Ratatui backend, including the headless test backend.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(rows[0]);
    let items: Vec<_> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let (badge, color) = match session.state.as_str() {
                "RUNNING" => ("RUNNING", Color::Green),
                "WAITING" | "WAITING_INPUT" => ("WAITING", Color::Yellow),
                "STUCK" => ("STUCK", Color::Magenta),
                "DEAD" => ("DEAD", Color::Red),
                _ => (session.state.as_str(), Color::Cyan),
            };
            let style = if index == app.selected_index {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::raw(format!("{} {} ", session.session_id, session.command)),
                Span::styled(format!("[{badge}]"), Style::default().fg(color)),
            ]))
            .style(style)
        })
        .collect();
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title("Sessions")),
        columns[0],
    );
    let title = app
        .sessions
        .get(app.selected_index)
        .map_or("Viewport", |s| s.session_id.as_str());
    frame.render_widget(
        Paragraph::new(app.active_viewport_lines.join("\n"))
            .block(Block::default().borders(Borders::ALL).title(title)),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(
            "q: quit | j/k or ↑/↓: switch session | Enter: attach | Space: prompt | F1: palette",
        ),
        rows[1],
    );
}

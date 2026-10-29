//! Ratatui layout for the session sidebar and terminal viewport.
use crate::{App, ModalState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
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
        .filter(|(_, session)| {
            !app.attention_filter || !matches!(session.state.as_str(), "IDLE" | "DEAD")
        })
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
    match &app.modal {
        ModalState::None => {}
        ModalState::CommandPalette { query, selected } => {
            let area = centered(frame.area(), 60, 12);
            frame.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Command Palette");
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(inner);
            frame.render_widget(Paragraph::new(format!("> {query}")), rows[0]);
            let actions = app.palette_actions(query);
            let offset = selected.saturating_sub(usize::from(rows[1].height).saturating_sub(1));
            let items: Vec<_> = actions
                .into_iter()
                .enumerate()
                .skip(offset)
                .map(|(index, action)| {
                    ListItem::new(action).style(if index == *selected {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    })
                })
                .collect();
            frame.render_widget(List::new(items), rows[1]);
        }
        ModalState::QuickPrompt { input } => {
            let area = centered(frame.area(), 60, 5);
            frame.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).title("Quick Prompt");
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);
            frame.render_widget(Paragraph::new(input.as_str()), rows[0]);
            frame.render_widget(Paragraph::new("[Enter] Submit | [Esc] Cancel"), rows[1]);
        }
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

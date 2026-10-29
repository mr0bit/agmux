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
                "WAITING" | "WAITING_INPUT" => ("WAITING ✉", Color::Yellow),
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
    let sidebar_title = if app.attention_filter {
        "Sessions [ATTENTION FILTER]"
    } else {
        "Sessions"
    };
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(sidebar_title)),
        columns[0],
    );

    if app.split_view {
        let split_panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(columns[1]);

        let pinned_title = app
            .pinned_session_index
            .and_then(|i| app.sessions.get(i))
            .map_or("Pinned Session", |s| s.session_id.as_str());

        frame.render_widget(
            Paragraph::new(app.pinned_viewport_lines.join("\n")).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{pinned_title} [PINNED]")),
            ),
            split_panes[0],
        );

        let live_title = app
            .sessions
            .get(app.selected_index)
            .map_or("Live Session", |s| s.session_id.as_str());

        frame.render_widget(
            Paragraph::new(app.active_viewport_lines.join("\n")).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{live_title} [LIVE]")),
            ),
            split_panes[1],
        );
    } else {
        let title = app
            .sessions
            .get(app.selected_index)
            .map_or("Viewport", |s| s.session_id.as_str());
        frame.render_widget(
            Paragraph::new(app.active_viewport_lines.join("\n"))
                .block(Block::default().borders(Borders::ALL).title(title)),
            columns[1],
        );
    }
    frame.render_widget(
        Paragraph::new(
            "q: quit | j/k: nav | Space: prompt | w: filter | .: ack | v: split | m: pin | F1: palette",
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
        ModalState::OnboardingWizard { selected } => {
            let area = centered(frame.area(), 70, 22);
            frame.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Welcome to Agmux — First-Run Setup");
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let mut lines = vec![Line::from("Detected AI Agents in PATH:")];
            for agent in &app.discovered_agents {
                let (status, style) = if agent.available {
                    ("[INSTALLED]", Style::default().fg(Color::Green))
                } else {
                    ("[NOT FOUND]", Style::default().add_modifier(Modifier::DIM))
                };
                lines.push(Line::from(vec![
                    Span::styled(status, style),
                    Span::raw(format!(" {} ({})", agent.name, agent.binary_name)),
                ]));
            }
            if let Some(root) = &app.repository_root {
                lines.push(Line::from(format!("Repository: {}", root.display())));
            }
            lines.push(Line::from("Select Default Environment to Launch:"));
            for (index, command) in app.onboarding_commands().iter().enumerate() {
                let label = if index == 0 {
                    format!("Shell ({command})")
                } else {
                    command.clone()
                };
                lines.push(Line::styled(
                    format!("{} {label}", if index == *selected { ">" } else { " " }),
                    if index == *selected {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    },
                ));
            }
            lines.push(Line::from("[↑/↓] Select  [Enter] Launch  [Esc] Quit"));
            if let Some(error) = &app.onboarding_error {
                lines.push(Line::styled(
                    error.as_str(),
                    Style::default().fg(Color::Red),
                ));
            }
            frame.render_widget(Paragraph::new(lines), inner);
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
        ModalState::ReviewDiff {
            scope,
            selected_hunk,
            files,
            ..
        } => {
            let area = centered(frame.area(), 90, 22);
            frame.render_widget(Clear, area);
            let scope_title = match scope {
                agmux_proto::DiffScope::SessionDiff => "Session Diff (since start)",
                agmux_proto::DiffScope::FullUncommitted => "Full Uncommitted",
                agmux_proto::DiffScope::VersusTarget => "Versus Target Branch",
                agmux_proto::DiffScope::LastCommit => "Last Commit (HEAD~1)",
                agmux_proto::DiffScope::Staged => "Staged (Index)",
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" Review Diff: {scope_title} (Ctrl+R) "));
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(inner);

            let scopes = [
                (agmux_proto::DiffScope::SessionDiff, "1:Session"),
                (agmux_proto::DiffScope::FullUncommitted, "2:Uncommitted"),
                (agmux_proto::DiffScope::VersusTarget, "3:Target"),
                (agmux_proto::DiffScope::LastCommit, "4:LastCommit"),
                (agmux_proto::DiffScope::Staged, "5:Staged"),
            ];
            let mut scope_spans = Vec::new();
            for (s, label) in scopes {
                let active = s == *scope;
                scope_spans.push(Span::styled(
                    format!(" [{label}] "),
                    if active {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().add_modifier(Modifier::DIM)
                    },
                ));
            }
            frame.render_widget(Paragraph::new(Line::from(scope_spans)), rows[0]);

            let mut diff_lines = Vec::new();
            if files.is_empty() {
                diff_lines.push(Line::styled(
                    "No changes in this review scope.",
                    Style::default().add_modifier(Modifier::DIM),
                ));
            } else {
                let mut current_hunk_index = 0;
                for file in files {
                    let old = file.old_path.as_deref().unwrap_or("/dev/null");
                    let new = file.new_path.as_deref().unwrap_or("/dev/null");
                    diff_lines.push(Line::styled(
                        format!("diff --git a/{old} b/{new}"),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                    for hunk in &file.hunks {
                        let is_selected = current_hunk_index == *selected_hunk;
                        diff_lines.push(Line::styled(
                            format!("{} {}", if is_selected { "▶" } else { " " }, hunk.header),
                            if is_selected {
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Blue)
                            },
                        ));
                        for line in &hunk.lines {
                            let (style, prefix) = match line.kind {
                                agmux_proto::DiffLineType::Added => {
                                    (Style::default().fg(Color::Green), "+")
                                }
                                agmux_proto::DiffLineType::Removed => {
                                    (Style::default().fg(Color::Red), "-")
                                }
                                agmux_proto::DiffLineType::Context => (Style::default(), " "),
                                agmux_proto::DiffLineType::NoNewline => {
                                    (Style::default().add_modifier(Modifier::DIM), "\\")
                                }
                            };
                            diff_lines
                                .push(Line::styled(format!("{prefix}{}", line.content), style));
                        }
                        current_hunk_index += 1;
                    }
                }
            }
            frame.render_widget(Paragraph::new(diff_lines), rows[1]);

            let footer = Line::from(vec![Span::raw(
                "[Tab/1-5] Scope | [c] Draft Comment | [C] Dispatch Round | [↑/↓] Hunk | [Esc] Close",
            )]);
            frame.render_widget(Paragraph::new(footer), rows[2]);
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

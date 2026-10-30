//! Ratatui layout for the session sidebar and terminal viewport.
use crate::{App, ModalState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
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
    let mut items = Vec::new();
    let mut selected_row = None;
    let mut previous_favorite = false;
    let mut previous_project = String::new();
    let mut previous_group = String::new();
    for index in app.workspace_order() {
        let session = &app.sessions[index];
        let meta = app.metadata_for(index);
        if meta.favorite && !previous_favorite {
            items.push(ListItem::new("⭐ Favorites"));
        }
        if !meta.favorite {
            if previous_favorite || previous_project != meta.project {
                items.push(ListItem::new(format!("▾ {}", meta.project)));
                previous_project = meta.project.clone();
                previous_group.clear();
            }
            if previous_group != meta.group {
                let mut indent = String::new();
                for component in meta.group.split('/').filter(|s| !s.is_empty()) {
                    indent.push_str("  ");
                    items.push(ListItem::new(format!("{indent}▾ {component}")));
                }
                previous_group = meta.group.clone();
            }
        }
        previous_favorite = meta.favorite;
        let style = if index == app.selected_index {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let indent = if meta.favorite {
            2
        } else {
            2 + meta.group.split('/').filter(|s| !s.is_empty()).count() * 2
        };
        let (badge, color) = match session.state.as_str() {
            "RUNNING" => ("RUNNING", Color::Green),
            "WAITING" | "WAITING_INPUT" => ("WAITING ✉", Color::Yellow),
            "STUCK" => ("STUCK", Color::Magenta),
            "DEAD" => ("DEAD", Color::Red),
            _ => (session.state.as_str(), Color::Cyan),
        };
        let first = Line::from(vec![
            Span::raw(format!("{}{} ", " ".repeat(indent), session.session_id)),
            Span::styled(format!("[{badge}]"), Style::default().fg(color)),
            Span::raw(
                match app.teardowns.get(&session.session_id).map(String::as_str) {
                    Some("archive") => " ↓",
                    Some("kill") => " ■",
                    _ => "",
                },
            ),
        ]);
        let lines = if app.comfortable_rows {
            vec![
                first,
                Line::raw(format!("  {}", session.command)),
                Line::raw(format!("  {}", session.cwd)),
            ]
        } else {
            vec![first]
        };
        if index == app.selected_index {
            selected_row = Some(items.len());
        }
        items.push(ListItem::new(lines).style(style));
    }
    let sidebar_title = if app.show_archived {
        "Sessions [ARCHIVED]".into()
    } else if !app.filter_query.is_empty() || matches!(app.modal, ModalState::Filter) {
        format!("Sessions /{}", app.filter_query)
    } else if app.attention_filter {
        "Sessions [ATTENTION FILTER]".into()
    } else {
        "Sessions".into()
    };
    let mut list_state = ListState::default();
    list_state.select(selected_row);
    frame.render_stateful_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(sidebar_title)),
        columns[0],
        &mut list_state,
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
        let base_title = app
            .sessions
            .get(app.selected_index)
            .map_or("Viewport", |s| s.session_id.as_str());
        let subtab_suffix = if app.subtabs.is_empty() {
            String::new()
        } else {
            let active_sub = app
                .subtabs
                .get(app.active_subtab_idx)
                .map_or("", String::as_str);
            format!(" [❯ {active_sub}]")
        };
        let title = if app.input_mode {
            format!("{base_title}{subtab_suffix} [INTERACTIVE - Esc to exit]")
        } else if subtab_suffix.is_empty() {
            base_title.to_string()
        } else {
            format!("{base_title}{subtab_suffix}")
        };

        let body_text = if app.sessions.is_empty()
            && app.active_viewport_lines.is_empty()
            && matches!(app.modal, ModalState::None)
        {
            vec![
                "Welcome to Agmux — Agentic Terminal Multiplexer & Orchestrator".to_string(),
                String::new(),
                "Quick-Start Hotkeys:".to_string(),
                "  [n]         Spawn native shell session immediately".to_string(),
                "  [i / Enter] Enter Interactive Terminal Mode (Esc to return to Normal Mode)"
                    .to_string(),
                "  [t / T]     Toggle archive / open shell sub-tab".to_string(),
                "  [Space]     Quick Prompt composer".to_string(),
                "  [v / m]     Split Compare & Pin viewport".to_string(),
                "  [K]         Shared Multi-Agent Kanban Board".to_string(),
                "  [Ctrl+R]    5-Scope Git Code Review Diff Engine".to_string(),
                "  [F1 / :]    Command Palette".to_string(),
                String::new(),
                "14 Supported Agent Profiles:".to_string(),
                "  claude, agy, gemini, codex, opencode, aider, cursor-agent,".to_string(),
                "  grok, q, pi, omp, cmd (command-code), hermes, muse".to_string(),
            ]
            .join("\n")
        } else {
            app.active_viewport_lines.join("\n")
        };

        frame.render_widget(
            Paragraph::new(body_text).block(Block::default().borders(Borders::ALL).title(title)),
            columns[1],
        );
    }
    let status_bar = if let Some(error) = &app.feature_error {
        format!("Error: {error}")
    } else if app.input_mode {
        "[INTERACTIVE] Esc: normal | Ctrl+F: search | Ctrl+PgUp/PgDn: notes".into()
    } else {
        "[NORMAL] g: group | *: favorite | D: density | a/t/u: archive | f: fork | Space: prompt | /: filter | F1: palette"
            .into()
    };
    frame.render_widget(Paragraph::new(status_bar), rows[1]);
    match &app.modal {
        ModalState::None => {}
        ModalState::Filter => {}
        ModalState::GlobalSearch {
            query,
            selected,
            hits,
            error,
        } => {
            let area = centered(frame.area(), 90, 80);
            frame.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!("Global search: {query} (Esc to close)"));
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(inner);
            let offset = selected.saturating_sub(usize::from(panes[0].height).saturating_sub(1));
            let items: Vec<_> =
                hits.iter()
                    .enumerate()
                    .skip(offset)
                    .map(|(i, hit)| {
                        let source = hit.page.map_or_else(
                            || format!("line {}", hit.line_no),
                            |page| format!("note {page}:{}", hit.line_no),
                        );
                        ListItem::new(format!("{} {source}: {}", hit.session_id, hit.snippet))
                            .style(if i == *selected {
                                Style::default().add_modifier(Modifier::REVERSED)
                            } else {
                                Style::default()
                            })
                    })
                    .collect();
            frame.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(error.as_deref().unwrap_or("Matches")),
                ),
                panes[0],
            );
            if let Some(hit) = hits.get(*selected) {
                let lines: Vec<_> = hit
                    .context
                    .iter()
                    .enumerate()
                    .map(|(i, text)| {
                        Line::styled(
                            text.clone(),
                            if i == hit.context_match {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            },
                        )
                    })
                    .collect();
                let scroll = hit
                    .context_match
                    .saturating_sub(usize::from(panes[1].height.saturating_sub(2)) / 2)
                    .min(u16::MAX as usize) as u16;
                frame.render_widget(
                    Paragraph::new(lines).scroll((scroll, 0)).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Read-only context"),
                    ),
                    panes[1],
                );
            }
        }
        ModalState::Notes {
            session_id,
            page,
            content,
            loaded,
        } => {
            let area = centered(frame.area(), 80, 70);
            frame.render_widget(Clear, area);
            let title =
                format!("Notes {session_id} page {page} | Ctrl+PgUp/PgDn or Tab/Shift+Tab | Esc");
            let text = if *loaded {
                content.as_str()
            } else {
                "Loading…"
            };
            let scroll = text
                .lines()
                .count()
                .saturating_sub(usize::from(area.height.saturating_sub(2)))
                .min(u16::MAX as usize) as u16;
            frame.render_widget(
                Paragraph::new(text)
                    .scroll((scroll, 0))
                    .block(Block::default().borders(Borders::ALL).title(title)),
                area,
            );
        }
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
        ModalState::GroupEditor { input } => {
            let area = centered(frame.area(), 70, 5);
            frame.render_widget(Clear, area);
            frame.render_widget(
                Paragraph::new(input.as_str()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Group path (Enter to save)"),
                ),
                area,
            );
        }
        ModalState::ForkMenu => {
            let area = centered(frame.area(), 60, 5);
            frame.render_widget(Clear, area);
            frame.render_widget(
                Paragraph::new("[s] Standalone   [t] Sub-tab   [Esc] Cancel").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Fork conversation"),
                ),
                area,
            );
        }
        ModalState::QuickPrompt { input } => {
            let area = centered(frame.area(), 80, 5);
            frame.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).title("Quick Prompt");
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);
            frame.render_widget(Paragraph::new(input.as_str()), rows[0]);
            frame.render_widget(
                Paragraph::new(format!(
                    "Tab: {} | Ctrl+L: {} | Ctrl+X: {} | Ctrl+T: {}",
                    ["reply", "codex", "claude"][app.prompt_tool],
                    if app.prompt_tool == 2 {
                        ["default", "sonnet", "opus"][app.prompt_model]
                    } else {
                        ["default", "gpt-5.4", "gpt-5.3-codex"][app.prompt_model]
                    },
                    ["default", "low", "medium", "high"][app.prompt_effort],
                    if app.prompt_worktree {
                        "worktree"
                    } else {
                        "current dir"
                    }
                )),
                rows[1],
            );
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
        ModalState::KanbanBoard { selected } => {
            let area = centered(frame.area(), 76, 16);
            frame.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Kanban Task Board (K)");
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let mut lines = Vec::new();
            if app.kanban_tasks.is_empty() {
                lines.push(Line::from(
                    "No Kanban tasks found. Create tasks with `agmux task create <ID> <TITLE>`.",
                ));
            } else {
                for (idx, task) in app.kanban_tasks.iter().enumerate() {
                    let marker = if idx == *selected { "▶" } else { " " };
                    let holder = task.claimed_by.as_deref().unwrap_or("-");
                    lines.push(Line::styled(
                        format!(
                            "{marker} [{}] {} ({}) — {}",
                            task.status, task.id, holder, task.title
                        ),
                        if idx == *selected {
                            Style::default().add_modifier(Modifier::REVERSED)
                        } else {
                            Style::default()
                        },
                    ));
                }
            }
            lines.push(Line::from("[↑/↓] Navigate | [Esc / K] Close"));
            frame.render_widget(Paragraph::new(lines), inner);
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

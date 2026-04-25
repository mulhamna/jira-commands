use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

use super::app::App;
use super::column::{format_column_summary, AVAILABLE_COLUMNS};
use super::mode::Mode;
use super::panel::{DetailTab, Focus};

pub(super) fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(size);

    let title = match app.mode {
        Mode::Browse => {
            if app.focus == Focus::Detail {
                format!(
                    " Jira CLI — {} — {} ",
                    app.selected_issue_key().unwrap_or_else(|| "Issue Detail".into()),
                    app.active_tab.label()
                )
            } else {
                format!(" Jira CLI  {}  ({} issues) ", app.jql, app.issues.len())
            }
        }
        Mode::Search => " Jira CLI — Search ".to_string(),
        Mode::Transition => " Jira CLI — Select Transition ".to_string(),
        Mode::Help => " Jira CLI — Help ".to_string(),
        Mode::ColumnPicker => " Jira CLI — Columns ".to_string(),
        Mode::AssigneePicker => " Jira CLI — Assignee Picker ".to_string(),
        Mode::ComponentPicker => " Jira CLI — Component Picker ".to_string(),
        Mode::SavedJqlPicker => " Jira CLI — Saved Queries ".to_string(),
        Mode::ServerInfo => " Jira CLI — Server ".to_string(),
        Mode::ConfigView => " Jira CLI — Config ".to_string(),
        Mode::ThemePicker => " Jira CLI — Themes ".to_string(),
        Mode::CommentCompose => " Jira CLI — Comment ".to_string(),
        Mode::LinkUrlInput => " Jira CLI — Link ".to_string(),
    };

    let header = Paragraph::new(title).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(header, chunks[0]);
    render_footer(f, app, chunks[2]);

    match app.mode {
        Mode::Browse => render_browse(f, app, chunks[1]),
        Mode::Search => {
            render_browse(f, app, chunks[1]);
            render_search_bar(f, app, size);
        }
        Mode::Transition => {
            render_browse(f, app, chunks[1]);
            render_transition_popup(f, app, size);
        }
        Mode::Help => {
            render_browse(f, app, chunks[1]);
            render_help_popup(f, size);
        }
        Mode::ColumnPicker => {
            render_browse(f, app, chunks[1]);
            render_column_picker_popup(f, app, size);
        }
        Mode::AssigneePicker => {
            render_browse(f, app, chunks[1]);
            render_assignee_picker_popup(f, app, size);
        }
        Mode::ComponentPicker => {
            render_browse(f, app, chunks[1]);
            render_component_picker_popup(f, app, size);
        }
        Mode::SavedJqlPicker
        | Mode::ServerInfo
        | Mode::ConfigView
        | Mode::ThemePicker
        | Mode::CommentCompose
        | Mode::LinkUrlInput => render_browse(f, app, chunks[1]),
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::Browse if app.focus == Focus::Detail => {
            " ←/→:tab  Esc:back  t:transition  e:edit  a:assign  ;:comment  w:worklog  u:upload  o:browser  ?:help  q:quit"
                .to_string()
        }
        Mode::Browse => {
            " j/k:move  Enter:detail  t:transition  C:columns  c:create  e:edit  a:assign  ;:comment  w:worklog  l:labels  m:comps  u:upload  o:browser  r:refresh  /:search  ?:help  q:quit"
                .to_string()
        }
        Mode::Search => " Type JQL  Enter:search  Esc:cancel".to_string(),
        Mode::Transition => " j/k:move  Enter:execute  Esc:cancel".to_string(),
        Mode::Help => " Any key: close".to_string(),
        Mode::ColumnPicker => " j/k:move  Space:toggle  a:all  Enter:save  Esc:cancel".to_string(),
        Mode::AssigneePicker => " type:search  j/k:move  Enter:assign  Esc:cancel".to_string(),
        Mode::ComponentPicker => " type:search  j/k:move  Space:toggle  Enter:save  Esc:cancel".to_string(),
        _ => " Esc:back".to_string(),
    };

    let (fg, bg) = if let Some((_, true)) = &app.status {
        (Color::White, Color::Red)
    } else if let Some((msg, false)) = &app.status {
        let status_line = Paragraph::new(format!(" {msg}"))
            .style(Style::default().fg(Color::Black).bg(Color::Green));
        f.render_widget(status_line, area);
        return;
    } else {
        (Color::DarkGray, Color::Reset)
    };

    if let Some((msg, true)) = &app.status {
        let err_line = Paragraph::new(format!(" ✗ {msg}")).style(Style::default().fg(fg).bg(bg));
        f.render_widget(err_line, area);
        return;
    }

    let footer = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: false });
    f.render_widget(footer, area);
}

fn render_browse(f: &mut Frame, app: &mut App, area: Rect) {
    if app.focus == Focus::Detail {
        render_master_detail(f, app, area);
    } else {
        render_list(f, app, area);
    }
}

fn render_master_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(area);
    render_list(f, app, cols[0]);
    render_detail(f, app, cols[1]);
}

fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = if app.visible_columns.is_empty() {
        AVAILABLE_COLUMNS.to_vec()
    } else {
        app.visible_columns.clone()
    };

    let header_cells = columns.iter().map(|column| {
        Cell::from(column.label()).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.issues.iter().map(|issue| {
        let cells = columns
            .iter()
            .map(|column| column.cell(issue))
            .collect::<Vec<_>>();
        Row::new(cells)
    });

    let widths: Vec<Constraint> = columns.iter().map(|column| column.width()).collect();
    let title = if app.focus == Focus::Detail {
        " Issues (master) "
    } else {
        " Issues "
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(issue) = app.selected_issue() else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let tab_titles = DetailTab::ALL
        .iter()
        .map(|tab| {
            let active = *tab == app.active_tab;
            let style = if active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Span::styled(format!(" {} ", tab.label()), style)
        })
        .collect::<Vec<_>>();

    let tabs = Paragraph::new(Line::from(tab_titles)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", issue.key)),
    );
    f.render_widget(tabs, chunks[0]);

    let body = match app.active_tab {
        DetailTab::Summary => build_summary_lines(issue),
        DetailTab::Comments => build_placeholder_lines("Comments panel", "Read/write path exists in client, panel wiring next."),
        DetailTab::Worklog => build_placeholder_lines("Worklog panel", "Worklog fetch exists, panel wiring next."),
        DetailTab::Attachments => build_attachment_lines(issue),
        DetailTab::Subtasks => build_subtask_lines(issue),
        DetailTab::Links => build_placeholder_lines("Links panel", "Remote links client exists, panel wiring next."),
    };

    let paragraph = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title(" Detail "))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, chunks[1]);
}

fn build_summary_lines(issue: &jira_core::model::Issue) -> Vec<Line<'static>> {
    let created = &issue.created[..10.min(issue.created.len())];
    let updated = &issue.updated[..10.min(issue.updated.len())];

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled(
                issue.key.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" — "),
            Span::styled(
                issue.summary.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        owned_field_line("Type", issue.issue_type.clone()),
        owned_field_line("Status", issue.status.clone()),
        owned_field_line("Project", issue.project_key.clone()),
    ];

    if let Some(p) = &issue.priority {
        lines.push(owned_field_line("Priority", p.clone()));
    }
    if let Some(a) = &issue.assignee {
        lines.push(owned_field_line("Assignee", a.clone()));
    }
    if let Some(r) = &issue.reporter {
        lines.push(owned_field_line("Reporter", r.clone()));
    }
    lines.push(owned_field_line("Created", created.to_string()));
    lines.push(owned_field_line("Updated", updated.to_string()));

    if let Some(desc) = &issue.description {
        let text = jira_core::adf::adf_to_text(desc);
        if !text.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Description:",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));
            for line in text.lines() {
                lines.push(Line::from(format!("  {line}")));
            }
        }
    }

    lines
}

fn build_attachment_lines(issue: &jira_core::model::Issue) -> Vec<Line<'static>> {
    if issue.attachments.is_empty() {
        return build_placeholder_lines("Attachments", "No attachments on this issue.");
    }

    let mut lines = vec![Line::from(format!(
        "{} attachment(s)",
        issue.attachments.len()
    ))];
    lines.push(Line::from(""));
    for attachment in &issue.attachments {
        lines.push(Line::from(format!(
            "• {} ({} bytes)",
            attachment.filename, attachment.size
        )));
    }
    lines
}

fn build_subtask_lines(issue: &jira_core::model::Issue) -> Vec<Line<'static>> {
    let subtasks = issue
        .fields
        .get("subtasks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if subtasks.is_empty() {
        return build_placeholder_lines("Subtasks", "No subtasks found.");
    }

    let mut lines = vec![Line::from(format!("{} subtask(s)", subtasks.len())), Line::from("")];
    for subtask in subtasks {
        let key = subtask.get("key").and_then(|v| v.as_str()).unwrap_or("?");
        let summary = subtask
            .get("fields")
            .and_then(|f| f.get("summary"))
            .and_then(|v| v.as_str())
            .unwrap_or("(no summary)");
        let status = subtask
            .get("fields")
            .and_then(|f| f.get("status"))
            .and_then(|s| s.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        lines.push(Line::from(format!("• {}  [{}]  {}", key, status, summary)));
    }
    lines
}

fn build_placeholder_lines(title: &str, message: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            title.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(message.to_string()),
    ]
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let popup = bottom_bar_rect(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" JQL Search ")
        .style(Style::default().bg(Color::Black));

    let input = Paragraph::new(app.search_input.as_str())
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(Clear, popup);
    f.render_widget(input, popup);

    let before_cursor: String = app.search_input.chars().take(app.search_cursor).collect();
    let cursor_x = popup.x + 1 + before_cursor.len() as u16;
    let cursor_y = popup.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_transition_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(50, 60, area);
    let items: Vec<ListItem> = app
        .transitions
        .iter()
        .map(|(_, name)| ListItem::new(name.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Transition: {} ", app.transition_issue_key))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut app.transition_list_state);
}

fn render_column_picker_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(54, 72, area);
    let [summary_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .areas(popup_area);

    let items: Vec<ListItem> = AVAILABLE_COLUMNS
        .iter()
        .map(|column| {
            let checked = if app.visible_columns.contains(column) {
                "[x]"
            } else {
                "[ ]"
            };
            ListItem::new(format!("{checked} {}", column.label()))
        })
        .collect();

    let selected_summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Selected: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_column_summary(&app.visible_columns),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(Span::styled(
            "Tip: press Space to toggle columns, then S or Enter to save.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title(" Column Settings ")
            .style(Style::default().bg(Color::Black)),
    );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let hints = Paragraph::new(vec![
        Line::from("↑/↓ move   Space toggle   a select all   r reset defaults"),
        Line::from("s or Enter save   Esc cancel"),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(Color::DarkGray));

    f.render_widget(Clear, popup_area);
    f.render_widget(selected_summary, summary_area);
    f.render_stateful_widget(list, list_area, &mut app.column_picker_state);
    f.render_widget(hints, hint_area);
}

fn render_assignee_picker_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(70, 70, area);
    let [input_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .areas(popup_area);

    let input = Paragraph::new(app.assignee_query.as_str())
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .title(format!(" Assignee: {} ", app.assignee_issue_key))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(Color::White));

    let items: Vec<ListItem> = app
        .assignee_options
        .iter()
        .map(|option| ListItem::new(option.label.clone()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let hints = Paragraph::new(vec![
        Line::from("Type to search assignees"),
        Line::from("↑/↓ move   Enter assign   Esc cancel"),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(Color::DarkGray));

    f.render_widget(Clear, popup_area);
    f.render_widget(input, input_area);
    f.render_stateful_widget(list, list_area, &mut app.assignee_state);
    f.render_widget(hints, hint_area);

    let before_cursor: String = app.assignee_query.chars().take(app.assignee_cursor).collect();
    f.set_cursor_position((
        input_area.x + 1 + before_cursor.len() as u16,
        input_area.y + 1,
    ));
}

fn render_component_picker_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(70, 75, area);
    let [input_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .areas(popup_area);

    let input = Paragraph::new(app.component_query.as_str())
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .title(format!(
                    " Components: {} ({}) ",
                    app.component_issue_key, app.component_project_key
                ))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(Color::White));

    let items: Vec<ListItem> = app
        .component_options
        .iter()
        .map(|option| {
            let checked = if app.component_selected.contains(&option.value) {
                "[x]"
            } else {
                "[ ]"
            };
            ListItem::new(format!("{checked} {}", option.label))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let hints = Paragraph::new(vec![
        Line::from("Type to filter project components"),
        Line::from("↑/↓ move   Space toggle   Enter save"),
        Line::from("Esc cancel"),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(Color::DarkGray));

    f.render_widget(Clear, popup_area);
    f.render_widget(input, input_area);
    f.render_stateful_widget(list, list_area, &mut app.component_state);
    f.render_widget(hints, hint_area);

    let before_cursor: String = app.component_query.chars().take(app.component_cursor).collect();
    f.set_cursor_position((
        input_area.x + 1 + before_cursor.len() as u16,
        input_area.y + 1,
    ));
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(70, 95, area);

    let lines = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Issue List:", Style::default().fg(Color::Yellow))),
        Line::from("  ↑/k       Move up"),
        Line::from("  ↓/j       Move down"),
        Line::from("  Enter     Open split detail view"),
        Line::from("  t         Transition issue"),
        Line::from("  C         Column settings"),
        Line::from("  c         Create new issue"),
        Line::from("  e         Edit selected issue"),
        Line::from("  a         Assign selected issue"),
        Line::from("  ;         Add comment"),
        Line::from("  w         Add worklog"),
        Line::from("  l         Set labels"),
        Line::from("  m         Edit components"),
        Line::from("  u         Upload attachment"),
        Line::from("  o         Open issue in browser"),
        Line::from("  r         Refresh list"),
        Line::from("  /         Search with JQL"),
        Line::from("  ?         Show help"),
        Line::from("  q         Quit the TUI"),
        Line::from(""),
        Line::from(Span::styled("Detail View:", Style::default().fg(Color::Yellow))),
        Line::from("  Esc / q   Back to list"),
        Line::from("  ←/→ / Tab Switch detail tabs"),
        Line::from("  Summary / Comments / Worklog / Attachments / Subtasks / Links"),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    f.render_widget(Clear, popup_area);
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

fn owned_field_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {label:<12}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(value),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn bottom_bar_rect(r: Rect) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(r);
    layout[1]
}

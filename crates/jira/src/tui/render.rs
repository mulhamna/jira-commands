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
        Mode::List => format!(" Jira CLI  {}  ({} issues) ", app.jql, app.issues.len()),
        Mode::View => " Jira CLI — Issue Detail ".to_string(),
        Mode::Search => " Jira CLI — Search ".to_string(),
        Mode::Transition => " Jira CLI — Select Transition ".to_string(),
        Mode::Help => " Jira CLI — Help ".to_string(),
        Mode::ColumnPicker => " Jira CLI — Columns ".to_string(),
        Mode::AssigneePicker => " Jira CLI — Assignee Picker ".to_string(),
        Mode::ComponentPicker => " Jira CLI — Component Picker ".to_string(),
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
        Mode::List => render_list(f, app, chunks[1]),
        Mode::View => render_detail(f, app, chunks[1]),
        Mode::Search => {
            render_list(f, app, chunks[1]);
            render_search_bar(f, app, size);
        }
        Mode::Transition => {
            render_list(f, app, chunks[1]);
            render_transition_popup(f, app, size);
        }
        Mode::Help => {
            render_list(f, app, chunks[1]);
            render_help_popup(f, size);
        }
        Mode::ColumnPicker => {
            render_list(f, app, chunks[1]);
            render_column_picker_popup(f, app, size);
        }
        Mode::AssigneePicker => {
            render_list(f, app, chunks[1]);
            render_assignee_picker_popup(f, app, size);
        }
        Mode::ComponentPicker => {
            render_list(f, app, chunks[1]);
            render_component_picker_popup(f, app, size);
        }
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::List => {
            " j/k:move  Enter:view  t:transition  C:columns  c:create  e:edit  a:assign  ;:comment  w:worklog  l:labels  m:comps  u:upload  o:browser  r:refresh  /:search  ?:help  q:quit"
                .to_string()
        }
        Mode::View => {
            " Esc:back  t:transition  e:edit  a:assign  ;:comment  w:worklog  l:labels  m:comps  u:upload  o:browser  ?:help  q:quit"
                .to_string()
        }
        Mode::Search => " Type JQL  Enter:search  Esc:cancel".to_string(),
        Mode::Transition => " j/k:move  Enter:execute  Esc:cancel".to_string(),
        Mode::Help => " Any key: close".to_string(),
        Mode::ColumnPicker => " j/k:move  Space:toggle  a:all  Enter:save  Esc:cancel".to_string(),
        Mode::AssigneePicker => " type:search  j/k:move  Enter:assign  Esc:cancel".to_string(),
        Mode::ComponentPicker => " type:search  j/k:move  Space:toggle  Enter:save  Esc:cancel".to_string(),
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
        Row::new(
            columns
                .iter()
                .map(|column| column.cell(issue))
                .collect::<Vec<_>>(),
        )
        .height(1)
    });

    let widths = columns
        .iter()
        .map(|column| column.width())
        .collect::<Vec<_>>();

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Issues "))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(issue) = app.selected_issue() else {
        return;
    };

    let created = &issue.created[..10.min(issue.created.len())];
    let updated = &issue.updated[..10.min(issue.updated.len())];

    let mut lines: Vec<Line> = vec![
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
        field_line("Type", &issue.issue_type),
        field_line("Status", &issue.status),
        field_line("Project", &issue.project_key),
    ];

    if let Some(p) = &issue.priority {
        lines.push(field_line("Priority", p));
    }
    if let Some(a) = &issue.assignee {
        lines.push(field_line("Assignee", a));
    }
    if let Some(r) = &issue.reporter {
        lines.push(field_line("Reporter", r));
    }
    lines.push(field_line("Created", created));
    lines.push(field_line("Updated", updated));

    if !issue.attachments.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Attachments ({}):", issue.attachments.len()),
            Style::default().fg(Color::DarkGray),
        )));
        for a in &issue.attachments {
            lines.push(Line::from(format!(
                "    • {} ({} bytes)",
                a.filename, a.size
            )));
        }
    }

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

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Issue Detail "),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
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

    let before_cursor: String = app
        .assignee_query
        .chars()
        .take(app.assignee_cursor)
        .collect();
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

    let before_cursor: String = app
        .component_query
        .chars()
        .take(app.component_cursor)
        .collect();
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
        Line::from(Span::styled(
            "Issue List:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  ↑/k       Move up"),
        Line::from("  ↓/j       Move down"),
        Line::from("  Enter     View issue detail"),
        Line::from("  t         Transition issue (in-TUI picker)"),
        Line::from("  C         Open native column settings popup"),
        Line::from("  c         Create new issue"),
        Line::from("  e         Edit selected issue (summary/assignee/priority)"),
        Line::from("  a         Open native assignee popup and assign selected issue"),
        Line::from("  ;         Add comment to selected issue"),
        Line::from("  w         Add worklog to selected issue"),
        Line::from("  l         Set labels on selected issue"),
        Line::from("  m         Open native component popup and set issue components"),
        Line::from("  u         Upload attachment to selected issue"),
        Line::from("  o         Open issue in browser"),
        Line::from("  r         Refresh list"),
        Line::from("  /         Search with JQL"),
        Line::from("  ?         Show this help"),
        Line::from("  q         Quit the TUI"),
        Line::from("  Esc       Back / cancel (quit from list)"),
        Line::from(""),
        Line::from(Span::styled(
            "Issue Detail:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  Esc / q   Back to list"),
        Line::from("  t         Transition this issue"),
        Line::from("  e / a / w / l / m / u   (same as list)"),
        Line::from("  o         Open in browser"),
        Line::from(""),
        Line::from(Span::styled(
            "Column Picker:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  ↑/k       Move up"),
        Line::from("  ↓/j       Move down"),
        Line::from("  Space     Toggle selected column"),
        Line::from("  a         Select all available columns"),
        Line::from("  r         Reset to default columns"),
        Line::from("  s / Enter Save preferences"),
        Line::from("  Esc       Cancel without saving"),
        Line::from(""),
        Line::from(Span::styled("Search:", Style::default().fg(Color::Yellow))),
        Line::from("  Type JQL, press Enter to search"),
        Line::from("  Left/Right  Move cursor   Home/End  Jump   Del  Delete forward"),
        Line::from("  Esc         Cancel"),
        Line::from(""),
        Line::from(Span::styled(
            "JQL Quick Reference:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  project = PROJ                       issues in a project"),
        Line::from("  assignee = currentUser()             assigned to me"),
        Line::from("  assignee = \"email@example.com\"       assigned to someone"),
        Line::from("  status = \"In Progress\"               by status"),
        Line::from("  status in (\"To Do\", \"In Progress\")   multiple statuses"),
        Line::from("  priority = High                      by priority"),
        Line::from("  sprint = openSprints()               current sprint"),
        Line::from("  updated >= -7d                       updated in last 7 days"),
        Line::from("  created >= -30d                      created in last 30 days"),
        Line::from("  text ~ \"login bug\"                   full-text search"),
        Line::from("  labels = backend                     by label"),
        Line::from("  ORDER BY updated DESC                sort order"),
        Line::from(""),
        Line::from(Span::styled(
            "Tip: combine with AND / OR — run `jirac issue jql --help` for more",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
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

fn field_line<'a>(label: &'a str, value: &'a str) -> Line<'a> {
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

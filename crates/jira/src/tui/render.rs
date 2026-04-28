use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

use super::app::App;
use super::column::format_column_summary;
use super::modal::render_modal;
use super::mode::Mode;
use super::panel::{DetailTab, Focus};
use super::theme::{Palette, ThemeName};

pub(super) fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let palette = app.prefs.theme.palette();
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
                    app.selected_issue_key()
                        .unwrap_or_else(|| "Issue Detail".into()),
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
        Mode::FixVersionPicker => " Jira CLI — Fix Version Picker ".to_string(),
        Mode::SavedJqlPicker => " Jira CLI — Saved Queries ".to_string(),
        Mode::ServerInfo => " Jira CLI — Server ".to_string(),
        Mode::ConfigView => " Jira CLI — Config ".to_string(),
        Mode::ThemePicker => " Jira CLI — Themes ".to_string(),
        Mode::Modal => " Jira CLI ".to_string(),
    };

    let header = Paragraph::new(title).style(
        Style::default()
            .fg(palette.header_fg)
            .bg(palette.header_bg)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(header, chunks[0]);
    render_footer(f, app, chunks[2], palette);

    match app.mode {
        Mode::Browse => render_browse(f, app, chunks[1], palette),
        Mode::Search => {
            render_browse(f, app, chunks[1], palette);
            render_search_bar(f, app, size, palette);
        }
        Mode::Transition => {
            render_browse(f, app, chunks[1], palette);
            render_transition_popup(f, app, size, palette);
        }
        Mode::Help => {
            render_browse(f, app, chunks[1], palette);
            render_help_popup(f, size, palette);
        }
        Mode::ColumnPicker => {
            render_browse(f, app, chunks[1], palette);
            render_column_picker_popup(f, app, size, palette);
        }
        Mode::AssigneePicker => {
            render_browse(f, app, chunks[1], palette);
            render_assignee_picker_popup(f, app, size, palette);
        }
        Mode::ComponentPicker => {
            render_browse(f, app, chunks[1], palette);
            render_component_picker_popup(f, app, size, palette);
        }
        Mode::FixVersionPicker => {
            render_browse(f, app, chunks[1], palette);
            render_fix_version_picker_popup(f, app, size, palette);
        }
        Mode::SavedJqlPicker => {
            render_browse(f, app, chunks[1], palette);
            render_saved_jql_popup(f, app, size, palette);
        }
        Mode::ThemePicker => {
            render_browse(f, app, chunks[1], palette);
            render_theme_picker_popup(f, app, size, palette);
        }
        Mode::ServerInfo => {
            render_browse(f, app, chunks[1], palette);
            render_text_popup(f, " Server Info ", &app.server_info_lines, size, palette);
        }
        Mode::ConfigView => {
            render_browse(f, app, chunks[1], palette);
            render_text_popup(f, " Config View ", &app.config_lines, size, palette);
        }
        Mode::Modal => {
            render_browse(f, app, chunks[1], palette);
            if let Some(modal) = app.modal.as_ref() {
                render_modal(f, modal, palette, size);
            }
        }
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect, palette: Palette) {
    let text = match &app.mode {
        Mode::Browse if app.focus == Focus::Detail => {
            " ↑/↓:scroll  PgUp/PgDn:fast scroll  Home:top  ←/→:tab  Esc:back  t:transition  e:edit  a:assign  ;:comment  w:worklog  m:comps  v:versions  u:upload  o:browser  ?:help  q:quit"
                .to_string()
        }
        Mode::Browse => {
            " j/k:move  Enter:detail  p:queries  T:theme  S:server  g:config  t:transition  C:columns  c:create  e:edit  a:assign  ;:comment  w:worklog  l:labels  m:comps  v:versions  u:upload  o:browser  r:refresh  /:search  ?:help  q:quit"
                .to_string()
        }
        Mode::Search => " Type JQL  Enter:search  Esc:cancel".to_string(),
        Mode::Transition => " j/k:move  Enter:execute  Esc:cancel".to_string(),
        Mode::Help => " Any key: close".to_string(),
        Mode::ColumnPicker => " ↑/↓:move  Space:toggle  type:filter  Tab:clear  Enter:save  Esc:cancel".to_string(),
        Mode::AssigneePicker => " type:search  j/k:move  Enter:assign  Esc:cancel".to_string(),
        Mode::ComponentPicker => " type:search  j/k:move  Space:toggle  Enter:save  Esc:cancel".to_string(),
        Mode::FixVersionPicker => " type:search  j/k:move  Space:toggle  Enter:save  Esc:cancel".to_string(),
        Mode::SavedJqlPicker => " ↑/↓:move  Enter:run  type:filter  Tab:clear  c:new  e:edit  d:delete  Esc:cancel".to_string(),
        Mode::ThemePicker => " j/k:move  Enter:apply theme  Esc:cancel".to_string(),
        Mode::Modal => {
            " Tab:next field  Ctrl+S:submit  Enter:newline (multiline)  Esc:cancel".to_string()
        }
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
        (palette.muted, Color::Reset)
    };

    if let Some((msg, true)) = &app.status {
        let err_line = Paragraph::new(format!(" ✗ {msg}")).style(Style::default().fg(fg).bg(bg));
        f.render_widget(err_line, area);
        return;
    }

    let footer = Paragraph::new(text)
        .style(Style::default().fg(palette.muted))
        .wrap(Wrap { trim: false });
    f.render_widget(footer, area);
}

fn render_browse(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    if app.focus == Focus::Detail {
        render_master_detail(f, app, area, palette);
    } else {
        render_list(f, app, area, palette);
    }
}

fn render_master_detail(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(area);
    render_list(f, app, cols[0], palette);
    render_detail(f, app, cols[1], palette);
}

fn render_list(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let columns = app.visible_column_specs();

    let header_cells = columns.iter().map(|col| {
        Cell::from(col.label.clone()).style(
            Style::default()
                .fg(palette.tab_active)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.issues.iter().map(|issue| {
        let cells = columns
            .iter()
            .map(|col| col.cell(issue))
            .collect::<Vec<_>>();
        Row::new(cells)
    });

    let widths: Vec<Constraint> = columns.iter().map(|col| col.width).collect();
    let title = if app.focus == Focus::Detail {
        " Issues (master) "
    } else {
        " Issues "
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if app.focus == Focus::Detail {
                    palette.blur_border
                } else {
                    palette.focus_border
                }))
                .title(title),
        )
        .row_highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
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
                    .fg(palette.tab_active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.tab_inactive)
            };
            Span::styled(format!(" {} ", tab.label()), style)
        })
        .collect::<Vec<_>>();

    let tabs = Paragraph::new(Line::from(tab_titles)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.focus_border))
            .title(format!(" {} ", issue.key)),
    );
    f.render_widget(tabs, chunks[0]);

    let body = match app.active_tab {
        DetailTab::Summary => build_summary_lines(issue, palette),
        DetailTab::Comments => build_comment_lines(app, palette),
        DetailTab::Worklog => build_worklog_lines(app, palette),
        DetailTab::Attachments => build_attachment_lines(issue),
        DetailTab::Subtasks => build_subtask_lines(issue),
        DetailTab::Links => build_link_lines(app),
    };

    let paragraph = Paragraph::new(body)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.focus_border))
                .title(format!(" Detail  (scroll:{}) ", app.detail_scroll)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));
    f.render_widget(paragraph, chunks[1]);
}

fn build_summary_lines(issue: &jira_core::model::Issue, palette: Palette) -> Vec<Line<'static>> {
    let created = &issue.created[..10.min(issue.created.len())];
    let updated = &issue.updated[..10.min(issue.updated.len())];

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled(
                issue.key.clone(),
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" — "),
            Span::styled(
                issue.summary.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        owned_field_line("Type", issue.issue_type.clone(), palette),
        owned_field_line("Status", issue.status.clone(), palette),
        owned_field_line("Project", issue.project_key.clone(), palette),
    ];

    if let Some(p) = &issue.priority {
        lines.push(owned_field_line("Priority", p.clone(), palette));
    }
    if let Some(a) = &issue.assignee {
        lines.push(owned_field_line("Assignee", a.clone(), palette));
    }
    if let Some(r) = &issue.reporter {
        lines.push(owned_field_line("Reporter", r.clone(), palette));
    }
    lines.push(owned_field_line("Created", created.to_string(), palette));
    lines.push(owned_field_line("Updated", updated.to_string(), palette));

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

fn build_comment_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    match &app.detail.comments {
        Some(comments) if comments.is_empty() => {
            build_placeholder_lines("Comments", "No comments on this issue.")
        }
        Some(comments) => {
            let mut lines = vec![
                Line::from(format!("{} comment(s)", comments.len())),
                Line::from(""),
            ];
            for comment in comments {
                let author = comment.author.clone().unwrap_or_else(|| "Unknown".into());
                let created = comment.created.get(..10).unwrap_or(&comment.created);
                lines.push(Line::from(vec![
                    Span::styled(
                        author,
                        Style::default()
                            .fg(palette.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("  {}", created)),
                ]));
                let body = comment.body.clone().unwrap_or_else(|| "(empty)".into());
                for line in body.lines() {
                    lines.push(Line::from(format!("  {line}")));
                }
                lines.push(Line::from(""));
            }
            lines
        }
        None => build_placeholder_lines("Comments", "Loading comments..."),
    }
}

fn build_worklog_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    match &app.detail.worklogs {
        Some(worklogs) if worklogs.is_empty() => {
            build_placeholder_lines("Worklog", "No worklogs on this issue.")
        }
        Some(worklogs) => {
            let mut lines = vec![
                Line::from(format!("{} worklog entr(y/ies)", worklogs.len())),
                Line::from(""),
            ];
            for worklog in worklogs {
                let author = worklog.author.clone().unwrap_or_else(|| "Unknown".into());
                let started = worklog.started.get(..10).unwrap_or(&worklog.started);
                lines.push(Line::from(vec![
                    Span::styled(
                        author,
                        Style::default()
                            .fg(palette.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("  {}  {}", worklog.time_spent, started)),
                ]));
                if let Some(comment) = &worklog.comment {
                    for line in comment.lines() {
                        lines.push(Line::from(format!("  {line}")));
                    }
                }
                lines.push(Line::from(""));
            }
            lines
        }
        None => build_placeholder_lines("Worklog", "Loading worklogs..."),
    }
}

fn build_link_lines(app: &App) -> Vec<Line<'static>> {
    match &app.detail.remote_links {
        Some(links) if links.is_empty() => {
            build_placeholder_lines("Links", "No remote links on this issue.")
        }
        Some(links) => {
            let mut lines = vec![
                Line::from(format!("{} link(s)", links.len())),
                Line::from(""),
            ];
            for link in links {
                let object = link.get("object");
                let title = object
                    .and_then(|o| o.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled link");
                let url = object
                    .and_then(|o| o.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                lines.push(Line::from(Span::styled(
                    title.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(format!("  {url}")));
                lines.push(Line::from(""));
            }
            lines
        }
        None => build_placeholder_lines("Links", "Loading remote links..."),
    }
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

    let mut lines = vec![
        Line::from(format!("{} subtask(s)", subtasks.len())),
        Line::from(""),
    ];
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

fn render_search_bar(f: &mut Frame, app: &App, area: Rect, palette: Palette) {
    let popup = bottom_bar_rect(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.focus_border))
        .title(" JQL Search ")
        .style(Style::default().bg(Color::Black));

    let input = Paragraph::new(app.search_input.as_str())
        .block(block)
        .style(Style::default().fg(palette.header_fg));

    f.render_widget(Clear, popup);
    f.render_widget(input, popup);

    let before_cursor: String = app.search_input.chars().take(app.search_cursor).collect();
    let cursor_x = popup.x + 1 + before_cursor.len() as u16;
    let cursor_y = popup.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_transition_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
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
                .border_style(Style::default().fg(palette.focus_border))
                .title(format!(" Transition: {} ", app.transition_issue_key))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut app.transition_list_state);
}

fn render_column_picker_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let popup_area = centered_rect(58, 80, area);
    let [header_area, search_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .areas(popup_area);

    let filtered = app.filtered_picker_fields();
    let specs = app.visible_column_specs();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|col| {
            let checked = if app.visible_columns.contains(&col.id) {
                "[x]"
            } else {
                "[ ]"
            };
            ListItem::new(format!("{checked} {} ({})", col.label, col.id))
        })
        .collect();

    let selected_summary = Paragraph::new(vec![Line::from(vec![
        Span::styled("Active: ", Style::default().fg(palette.muted)),
        Span::styled(
            format_column_summary(&specs),
            Style::default().fg(palette.accent),
        ),
    ])])
    .block(
        Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(palette.focus_border))
            .title(" Column Settings ")
            .style(Style::default().bg(Color::Black)),
    );

    let search_display = format!(
        "{}{}",
        app.column_picker_filter,
        if app.column_picker_filter.is_empty() {
            "type to filter..."
        } else {
            ""
        }
    );
    let search_bar = Paragraph::new(search_display)
        .style(Style::default().fg(if app.column_picker_filter.is_empty() {
            palette.muted
        } else {
            palette.accent
        }))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
                .border_style(Style::default().fg(palette.focus_border))
                .title(" Search ")
                .style(Style::default().bg(Color::Black)),
        );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(palette.focus_border))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let hints = Paragraph::new(Line::from(
        "↑/↓ move   Space toggle   Enter save   Tab clear filter   Esc cancel",
    ))
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(palette.focus_border))
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(palette.muted));

    f.render_widget(Clear, popup_area);
    f.render_widget(selected_summary, header_area);
    f.render_widget(search_bar, search_area);
    f.render_stateful_widget(list, list_area, &mut app.column_picker_state);
    f.render_widget(hints, hint_area);
}

fn render_assignee_picker_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
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
                .border_style(Style::default().fg(palette.focus_border))
                .title(format!(" Assignee: {} ", app.assignee_issue_key))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(palette.header_fg));

    let items: Vec<ListItem> = app
        .assignee_options
        .iter()
        .map(|option| ListItem::new(option.label.clone()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(palette.focus_border))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
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
            .border_style(Style::default().fg(palette.focus_border))
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(palette.muted));

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

fn render_component_picker_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let popup_area = side_panel_rect(area);
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
                .border_style(Style::default().fg(palette.focus_border))
                .title(format!(
                    " Components: {} ({}) ",
                    app.component_issue_key, app.component_project_key
                ))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(palette.header_fg));

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
                .border_style(Style::default().fg(palette.focus_border))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
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
            .border_style(Style::default().fg(palette.focus_border))
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(palette.muted));

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

fn render_fix_version_picker_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let popup_area = side_panel_rect(area);
    let [input_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .areas(popup_area);

    let input = Paragraph::new(app.fix_version_query.as_str())
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(palette.focus_border))
                .title(format!(
                    " Fix Versions: {} ({}) ",
                    app.fix_version_issue_key, app.fix_version_project_key
                ))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(palette.header_fg));

    let items: Vec<ListItem> = app
        .fix_version_options
        .iter()
        .map(|option| {
            let checked = if app.fix_version_selected.contains(&option.value) {
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
                .border_style(Style::default().fg(palette.focus_border))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let hints = Paragraph::new(vec![
        Line::from("Type to filter project fix versions"),
        Line::from("↑/↓ move   Space toggle   Enter save"),
        Line::from("Esc cancel"),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(palette.focus_border))
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(palette.muted));

    f.render_widget(Clear, popup_area);
    f.render_widget(input, input_area);
    f.render_stateful_widget(list, list_area, &mut app.fix_version_state);
    f.render_widget(hints, hint_area);

    let before_cursor: String = app
        .fix_version_query
        .chars()
        .take(app.fix_version_cursor)
        .collect();
    f.set_cursor_position((
        input_area.x + 1 + before_cursor.len() as u16,
        input_area.y + 1,
    ));
}

fn render_saved_jql_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let popup_area = centered_rect(72, 75, area);
    let [summary_area, search_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .areas(popup_area);

    let filtered = app.filtered_saved_jqls();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, saved)| ListItem::new(format!("{}  •  {}", saved.name, saved.jql)))
        .collect();

    let selected_summary = if let Some(saved) = app.selected_saved_jql() {
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Selected: ", Style::default().fg(palette.muted)),
                Span::styled(saved.name.clone(), Style::default().fg(palette.accent)),
            ]),
            Line::from(Span::styled(
                saved.jql.clone(),
                Style::default().fg(palette.header_fg),
            )),
        ])
    } else if app.prefs.saved_jqls.is_empty() {
        Paragraph::new(vec![
            Line::from(Span::styled(
                "No saved queries yet.",
                Style::default().fg(palette.muted),
            )),
            Line::from(Span::styled(
                "Press c to create one.",
                Style::default().fg(palette.muted),
            )),
        ])
    } else {
        Paragraph::new(Line::from(Span::styled(
            "No results.",
            Style::default().fg(palette.muted),
        )))
    }
    .block(
        Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(palette.focus_border))
            .title(" Saved Queries ")
            .style(Style::default().bg(Color::Black)),
    );

    let search_display = if app.jql_picker_filter.is_empty() {
        Span::styled("type to filter...", Style::default().fg(palette.muted))
    } else {
        Span::styled(
            app.jql_picker_filter.clone(),
            Style::default().fg(palette.accent),
        )
    };
    let search_bar = Paragraph::new(Line::from(search_display)).block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
            .border_style(Style::default().fg(palette.focus_border))
            .title(" Search ")
            .style(Style::default().bg(Color::Black)),
    );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(palette.focus_border))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let hints = Paragraph::new(Line::from(
        "↑/↓ move   Enter run   c create   e edit   d delete   Tab clear   Esc cancel",
    ))
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(palette.focus_border))
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(palette.muted));

    f.render_widget(Clear, popup_area);
    f.render_widget(selected_summary, summary_area);
    f.render_widget(search_bar, search_area);
    f.render_stateful_widget(list, list_area, &mut app.saved_jql_state);
    f.render_widget(hints, hint_area);
}

fn render_theme_picker_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let popup_area = centered_rect(40, 45, area);
    let items: Vec<ListItem> = ThemeName::ALL
        .iter()
        .map(|theme| {
            let marker = if *theme == app.prefs.theme {
                "✓"
            } else {
                " "
            };
            ListItem::new(format!("[{marker}] {}", theme.label()))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.focus_border))
                .title(" Theme Picker ")
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut app.theme_state);
}

fn render_text_popup(f: &mut Frame, title: &str, lines: &[String], area: Rect, palette: Palette) {
    let popup_area = centered_rect(72, 85, area);
    let mut content = if lines.is_empty() {
        vec![Line::from("No data")]
    } else {
        lines.iter().cloned().map(Line::from).collect::<Vec<_>>()
    };
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "Esc or q to close",
        Style::default().fg(palette.muted),
    )));

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.focus_border))
                .title(title)
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

fn render_help_popup(f: &mut Frame, area: Rect, palette: Palette) {
    let popup_area = centered_rect(70, 95, area);

    let lines = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Issue List:",
            Style::default().fg(palette.tab_active),
        )),
        Line::from("  ↑/k       Move up"),
        Line::from("  ↓/j       Move down"),
        Line::from("  Enter     Open split detail view"),
        Line::from("  p         Open saved queries (run/create/edit/delete)"),
        Line::from("  T         Open theme picker"),
        Line::from("  S         Show server info"),
        Line::from("  g         Show config file"),
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
        Line::from(Span::styled(
            "Detail View:",
            Style::default().fg(palette.tab_active),
        )),
        Line::from("  Esc / q   Back to list"),
        Line::from("  ←/→ / Tab Switch detail tabs"),
        Line::from("  Summary / Comments / Worklog / Attachments / Subtasks / Links"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(palette.muted),
        )),
    ];

    f.render_widget(Clear, popup_area);
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.focus_border))
                .title(" Help ")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

fn owned_field_line(label: &str, value: String, palette: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label:<12}"), Style::default().fg(palette.muted)),
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

fn side_panel_rect(area: Rect) -> Rect {
    let [_, panel] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .areas(area);
    panel
}

use std::collections::HashSet;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, Wrap,
    },
    Frame,
};

use super::app::App;
use super::column::format_column_summary;
use super::modal::render_modal;
use super::mode::Mode;
use super::panel::{DetailTab, Focus, PickerHit};
use super::picker::PickerOption;
use super::theme::{Palette, ThemeName};
use super::version_format::{backlog_preview_lines, version_status_badges};

/// Build a `PickerHit` from a bordered list/table widget area.
/// `area` is the *outer* rect (including borders); the result's `area`
/// is shrunk by 1 cell on every side, matching where rows are actually drawn.
fn picker_hit_for_bordered(area: Rect, offset: usize, count: usize) -> PickerHit {
    let inner = Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    PickerHit {
        area: inner,
        offset,
        count,
    }
}

pub(super) fn ui(f: &mut Frame, app: &mut App) {
    app.hit_zones.clear();
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
        Mode::ProjectVersionBrowser => " Jira CLI — Project Versions ".to_string(),
        Mode::ColumnPicker => " Jira CLI — Columns ".to_string(),
        Mode::AssigneePicker => " Jira CLI — Assignee Picker ".to_string(),
        Mode::ComponentPicker => " Jira CLI — Component Picker ".to_string(),
        Mode::FixVersionPicker => " Jira CLI — Fix Version Picker ".to_string(),
        Mode::SprintPicker => " Jira CLI — Sprint Picker ".to_string(),
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
    // Footer buttons are recorded into hit_zones after the footer text is laid
    // out — see the end of render_footer.

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
            let popup = render_help_popup(f, size, palette, app.help_scroll);
            app.hit_zones.popup = Some(popup);
        }
        Mode::ProjectVersionBrowser => {
            render_browse(f, app, chunks[1], palette);
            render_project_version_browser_popup(f, app, size, palette);
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
        Mode::SprintPicker => {
            render_browse(f, app, chunks[1], palette);
            render_sprint_picker_popup(f, app, size, palette);
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
            let popup =
                render_text_popup(f, " Server Info ", &app.server_info_lines, size, palette);
            app.hit_zones.popup = Some(popup);
        }
        Mode::ConfigView => {
            render_browse(f, app, chunks[1], palette);
            let popup = render_text_popup(f, " Config View ", &app.config_lines, size, palette);
            app.hit_zones.popup = Some(popup);
        }
        Mode::Modal => {
            render_browse(f, app, chunks[1], palette);
            if let Some(modal) = app.modal.as_mut() {
                render_modal(f, modal, palette, size);
            }
        }
    }
}

fn render_footer(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let text = match &app.mode {
        Mode::Browse if app.focus == Focus::Detail => {
            " ↑/↓:scroll  PgUp/PgDn:fast scroll  Home:top  ←/→:tab  Esc:back  t:transition  e:edit  y:type  M:move  a:assign  ;:comment  ::bulk-comment  w:worklog  b:bulk-log  m:comps  v:versions  u:upload  o:browser  ?:help  q:quit"
                .to_string()
        }
        Mode::Browse => {
            " j/k:move  Enter:detail  p:queries  n:mentions  R:mark-read  T:theme  S:server  g:config  t:transition  C:columns  c:create  e:edit  y:type  M:move  a:assign  ;:comment  ::bulk-comment  w:worklog  b:bulk-log  l:labels  m:comps  v:versions  u:upload  o:browser  r:refresh  /:search  ?:help  q:quit"
                .to_string()
        }
        Mode::Search => " Type JQL  Enter:search  Esc:cancel".to_string(),
        Mode::Transition => " j/k:move  Enter:execute  Esc:cancel".to_string(),
        Mode::Help => " Any key: close".to_string(),
        Mode::ProjectVersionBrowser => {
            " type:filter  j/k:move  Enter:refresh  n:new  e:edit  Esc:close".to_string()
        }
        Mode::ColumnPicker => " ↑/↓:move  Space:toggle  type:filter  Tab:clear  Enter:save  Esc:cancel".to_string(),
        Mode::AssigneePicker => " type:search  j/k:move  Enter:assign  Esc:cancel".to_string(),
        Mode::ComponentPicker => " type:search  j/k:move  Space:toggle  Enter:save  Esc:cancel".to_string(),
        Mode::FixVersionPicker => " type:search  j/k:move  Space:toggle  Enter:save  Esc:cancel".to_string(),
        Mode::SprintPicker => " type:filter  j/k:move  Enter:assign to sprint  Esc:cancel".to_string(),
        Mode::SavedJqlPicker => " ↑/↓:move  Enter:run  type:filter  Tab:clear  c:new  e:edit  d:delete  Esc:cancel".to_string(),
        Mode::ThemePicker => " j/k:move  Enter:apply theme  Esc:cancel".to_string(),
        Mode::Modal => {
            " Tab:next field  Ctrl+S:submit  Enter:submit/non-multiline or newline/multiline  Esc:cancel".to_string()
        }
        _ => " Esc:back".to_string(),
    };

    let (fg, bg) = if let Some((_, true)) = &app.status {
        (Color::White, Color::Red)
    } else if let Some((msg, false)) = &app.status {
        let (fg, bg) = if msg.starts_with("Update available:") {
            (Color::Black, Color::Yellow)
        } else {
            (Color::Black, Color::Green)
        };
        let status_line = Paragraph::new(format!(" {msg}")).style(Style::default().fg(fg).bg(bg));
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

    // Split the footer row: hint text fills the left side, two button rects
    // sit at the right edge so they can be hit-tested by the mouse handler.
    let unread = app.notification_entries.iter().filter(|e| !e.read).count();
    let help_label = " [?] ";
    let notif_label = format!(" [🔔 {unread}] ");
    let help_w = help_label.chars().count() as u16;
    // emoji + digits — use string width approximation via char count + 1 for
    // the 2-cell wide bell glyph. Conservative: pad by 1.
    let notif_w = (notif_label.chars().count() as u16).saturating_add(1);

    let mut buttons_w = help_w;
    if unread > 0 {
        buttons_w = buttons_w.saturating_add(notif_w);
    }

    // Reserve room for the buttons; everything else is hint text.
    let text_w = area.width.saturating_sub(buttons_w);
    let text_rect = Rect {
        x: area.x,
        y: area.y,
        width: text_w,
        height: area.height,
    };

    let footer = Paragraph::new(text)
        .style(Style::default().fg(palette.muted))
        .wrap(Wrap { trim: false });
    f.render_widget(footer, text_rect);

    // Right-align: help button is rightmost, notif button (if any) sits just
    // to its left.
    let help_rect = Rect {
        x: area.x.saturating_add(area.width).saturating_sub(help_w),
        y: area.y,
        width: help_w,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(help_label).style(
            Style::default()
                .fg(palette.tab_active)
                .add_modifier(Modifier::BOLD),
        ),
        help_rect,
    );
    app.hit_zones.help_button = Some(help_rect);

    if unread > 0 {
        let notif_rect = Rect {
            x: help_rect.x.saturating_sub(notif_w),
            y: area.y,
            width: notif_w,
            height: 1,
        };
        let notif_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        f.render_widget(Paragraph::new(notif_label).style(notif_style), notif_rect);
        app.hit_zones.notif_button = Some(notif_rect);
    }
}

fn render_browse(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    if app.focus == Focus::Detail {
        render_master_detail(f, app, area, palette);
    } else {
        render_list(f, app, area, palette);
    }
}

fn render_master_detail(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let pct = app.split_pct.clamp(20, 80);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(pct),
            Constraint::Percentage(100 - pct),
        ])
        .split(area);
    app.hit_zones.master_detail_area = Some(area);
    // Splitter = rightmost column of the list pane (where the border is drawn).
    // Width 1, full height. Mouse hit-tests + drag updates split_pct.
    let splitter_x = cols[0].x.saturating_add(cols[0].width).saturating_sub(1);
    app.hit_zones.splitter = Some(Rect {
        x: splitter_x,
        y: area.y,
        width: 1,
        height: area.height,
    });
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
    app.hit_zones.list = Some(area);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let issue_key = match app.selected_issue() {
        Some(issue) => issue.key.clone(),
        None => return,
    };
    app.hit_zones.detail_pane = Some(area);

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

    // Record one Rect per tab header so mouse::handle_mouse can hit-test clicks.
    // Tabs are rendered on the single content row of a bordered Paragraph at
    // chunks[0], so the first tab starts at (x + 1, y + 1) and each tab's
    // width matches its formatted label (" {label} ") in columns.
    let tab_origin_x = chunks[0].x.saturating_add(1);
    let tab_origin_y = chunks[0].y.saturating_add(1);
    let inner_width = chunks[0].width.saturating_sub(2);
    let mut cursor_x = tab_origin_x;
    let tab_rects: Vec<(Rect, DetailTab)> = DetailTab::ALL
        .iter()
        .filter_map(|tab| {
            let label_width = (tab.label().chars().count() + 2) as u16;
            if cursor_x.saturating_add(label_width) > tab_origin_x.saturating_add(inner_width) {
                return None;
            }
            let rect = Rect {
                x: cursor_x,
                y: tab_origin_y,
                width: label_width,
                height: 1,
            };
            cursor_x = cursor_x.saturating_add(label_width);
            Some((rect, *tab))
        })
        .collect();
    app.hit_zones.detail_tabs = tab_rects;

    let tabs = Paragraph::new(Line::from(tab_titles)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.focus_border))
            .title(format!(" {issue_key} ")),
    );
    f.render_widget(tabs, chunks[0]);

    let body = match app.active_tab {
        DetailTab::Summary => {
            let issue = app.selected_issue().expect("issue exists");
            build_summary_lines(issue, app.detail.watchers.as_ref(), palette)
        }
        DetailTab::Comments => build_comment_lines(app, palette),
        DetailTab::Worklog => build_worklog_lines(app, palette),
        DetailTab::Attachments => {
            let issue = app.selected_issue().expect("issue exists");
            build_attachment_lines(issue)
        }
        DetailTab::Subtasks => {
            let issue = app.selected_issue().expect("issue exists");
            build_subtask_lines(issue)
        }
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

fn build_summary_lines(
    issue: &jira_core::model::Issue,
    watchers: Option<&jira_core::model::Watchers>,
    palette: Palette,
) -> Vec<Line<'static>> {
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

    if let Some(ws) = watchers {
        let marker = if ws.is_watching { " (watching)" } else { "" };
        lines.push(owned_field_line(
            "Watchers",
            format!("{}{}", ws.watch_count, marker),
            palette,
        ));
        if !ws.watchers.is_empty() {
            let names: Vec<String> = ws
                .watchers
                .iter()
                .take(5)
                .map(|w| w.display_name.clone())
                .collect();
            let mut joined = names.join(", ");
            if ws.watchers.len() > 5 {
                joined.push_str(&format!(" (+{} more)", ws.watchers.len() - 5));
            }
            lines.push(Line::from(format!("           {joined}")));
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
            for (idx, comment) in comments.iter().enumerate() {
                if idx > 0 {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(48),
                        Style::default().fg(palette.muted),
                    )));
                    lines.push(Line::from(""));
                }
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
                lines.push(Line::from(Span::styled(
                    link.object.title.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(format!("  {}", link.object.url)));
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
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        popup_area,
        app.transition_list_state.offset(),
        app.transitions.len(),
    ));
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
    let visible_count = filtered.len();
    f.render_stateful_widget(list, list_area, &mut app.column_picker_state);
    f.render_widget(hints, hint_area);
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        list_area,
        app.column_picker_state.offset(),
        visible_count,
    ));
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
    let assignee_count = app.assignee_options.len();
    f.render_stateful_widget(list, list_area, &mut app.assignee_state);
    f.render_widget(hints, hint_area);
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        list_area,
        app.assignee_state.offset(),
        assignee_count,
    ));

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
    let title = format!(
        " Components: {} ({}) ",
        app.component_issue_key, app.component_project_key
    );
    let option_count = app.component_options.len();
    let (popup_area, list_area) = render_multi_select_picker_popup(
        f,
        area,
        palette,
        &app.component_query,
        app.component_cursor,
        &app.component_options.clone(),
        &app.component_selected,
        &mut app.component_state,
        &title,
        &[
            "Type to filter project components",
            "↑/↓ move   Space toggle   Enter save",
            "Esc cancel",
        ],
    );
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        list_area,
        app.component_state.offset(),
        option_count,
    ));
}

fn render_fix_version_picker_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let title = format!(
        " Fix Versions: {} ({}) ",
        app.fix_version_issue_key, app.fix_version_project_key
    );
    let option_count = app.fix_version_options.len();
    let (popup_area, list_area) = render_multi_select_picker_popup(
        f,
        area,
        palette,
        &app.fix_version_query,
        app.fix_version_cursor,
        &app.fix_version_options.clone(),
        &app.fix_version_selected,
        &mut app.fix_version_state,
        &title,
        &[
            "Type to filter project fix versions",
            "↑/↓ move   Space toggle   Enter save",
            "Esc cancel",
        ],
    );
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        list_area,
        app.fix_version_state.offset(),
        option_count,
    ));
}

fn render_project_version_browser_popup(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    palette: Palette,
) {
    let popup_area = centered_rect(86, 82, area);
    let [left_area, right_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .areas(popup_area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(left_area);

    let query = Paragraph::new(app.project_version_query.clone())
        .block(
            Block::default()
                .title(format!(
                    " Fix Versions — {} ",
                    if app.project_version_project_key.is_empty() {
                        "project"
                    } else {
                        &app.project_version_project_key
                    }
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.focus_border)),
        )
        .style(Style::default().fg(palette.header_fg));

    let version_items: Vec<ListItem> = if app.project_version_options.is_empty() {
        vec![ListItem::new("No fix versions")]
    } else {
        app.project_version_options
            .iter()
            .map(|option| ListItem::new(option.label.clone()))
            .collect()
    };

    let versions = List::new(version_items)
        .block(
            Block::default()
                .title(" Versions ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.focus_border)),
        )
        .highlight_style(
            Style::default()
                .fg(palette.header_fg)
                .bg(palette.highlight)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut right_lines: Vec<Line<'static>> = Vec::new();
    if let Some(version) = app.selected_project_version() {
        right_lines.push(Line::from(Span::styled(
            version.name.clone(),
            Style::default()
                .fg(palette.tab_active)
                .add_modifier(Modifier::BOLD),
        )));
        let badges = version_status_badges(version);
        if !badges.is_empty() {
            right_lines.push(Line::from(badges));
        }
        if let Some(description) = version.description.as_deref() {
            let trimmed = description.trim();
            if !trimmed.is_empty() {
                right_lines.push(Line::from(""));
                right_lines.push(Line::from(trimmed.to_string()));
            }
        }
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(Span::styled(
            "Metadata:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        if let Some(date) = version.start_date.as_deref() {
            right_lines.push(Line::from(format!(
                "  Start: {}",
                &date[..10.min(date.len())]
            )));
        }
        if let Some(date) = version.release_date.as_deref() {
            right_lines.push(Line::from(format!(
                "  Release: {}",
                &date[..10.min(date.len())]
            )));
        }
        right_lines.push(Line::from("  Press e to edit, n to create a version"));
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(Span::styled(
            "Open backlog:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        right_lines.push(Line::from(""));

        match &app.project_version_preview {
            Some(preview) if preview.version.name == version.name => {
                for line in backlog_preview_lines(preview, 25) {
                    right_lines.push(Line::from(line));
                }
            }
            _ => right_lines.push(Line::from("Loading backlog preview...")),
        }
    } else {
        right_lines.push(Line::from("Select a fix version to preview its backlog."));
    }

    let detail = Paragraph::new(right_lines)
        .block(
            Block::default()
                .title(" Backlog Preview ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.focus_border)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, popup_area);
    f.render_widget(query, left_chunks[0]);
    let version_count = app.project_version_options.len();
    f.render_stateful_widget(versions, left_chunks[1], &mut app.project_version_state);
    f.render_widget(detail, right_area);
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        left_chunks[1],
        app.project_version_state.offset(),
        version_count,
    ));

    let before_cursor: String = app
        .project_version_query
        .chars()
        .take(app.project_version_cursor)
        .collect();
    let cursor_x = left_chunks[0].x + 1 + before_cursor.len() as u16;
    let cursor_y = left_chunks[0].y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_sprint_picker_popup(f: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let title = format!(" Sprint: {} ", app.sprint_issue_key);
    let option_count = app.sprint_options.len();
    let (popup_area, list_area) = render_single_select_picker_popup(
        f,
        area,
        palette,
        &app.sprint_query,
        app.sprint_cursor,
        &app.sprint_options.clone(),
        &mut app.sprint_state,
        &title,
        &[
            "Type to filter sprints",
            "↑/↓ move   Enter assign   Esc cancel",
        ],
    );
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        list_area,
        app.sprint_state.offset(),
        option_count,
    ));
}

#[allow(clippy::too_many_arguments)]
fn render_multi_select_picker_popup(
    f: &mut Frame,
    area: Rect,
    palette: Palette,
    query: &str,
    cursor: usize,
    options: &[PickerOption],
    selected: &HashSet<String>,
    state: &mut ListState,
    title: &str,
    hint_lines: &[&str],
) -> (Rect, Rect) {
    let popup_area = side_panel_rect(area);
    let [input_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .areas(popup_area);

    let input = Paragraph::new(query)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(palette.focus_border))
                .title(title.to_string())
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(palette.header_fg));

    let items: Vec<ListItem> = options
        .iter()
        .map(|option| {
            let checked = if selected.contains(&option.value) {
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

    let hint_text: Vec<Line> = hint_lines.iter().map(|l| Line::from(*l)).collect();
    let hints = Paragraph::new(hint_text)
        .block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(palette.focus_border))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(palette.muted));

    f.render_widget(Clear, popup_area);
    f.render_widget(input, input_area);
    f.render_stateful_widget(list, list_area, state);
    f.render_widget(hints, hint_area);

    let before_cursor: String = query.chars().take(cursor).collect();
    f.set_cursor_position((
        input_area.x + 1 + before_cursor.len() as u16,
        input_area.y + 1,
    ));

    (popup_area, list_area)
}

#[allow(clippy::too_many_arguments)]
fn render_single_select_picker_popup(
    f: &mut Frame,
    area: Rect,
    palette: Palette,
    query: &str,
    cursor: usize,
    options: &[PickerOption],
    state: &mut ListState,
    title: &str,
    hint_lines: &[&str],
) -> (Rect, Rect) {
    let popup_area = side_panel_rect(area);
    let [input_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .areas(popup_area);

    let input = Paragraph::new(query)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(palette.focus_border))
                .title(title.to_string())
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(palette.header_fg));

    let items: Vec<ListItem> = options
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

    let hint_text: Vec<Line> = hint_lines.iter().map(|l| Line::from(*l)).collect();
    let hints = Paragraph::new(hint_text)
        .block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(palette.focus_border))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(palette.muted));

    f.render_widget(Clear, popup_area);
    f.render_widget(input, input_area);
    f.render_stateful_widget(list, list_area, state);
    f.render_widget(hints, hint_area);

    let before_cursor: String = query.chars().take(cursor).collect();
    f.set_cursor_position((
        input_area.x + 1 + before_cursor.len() as u16,
        input_area.y + 1,
    ));

    (popup_area, list_area)
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
    let saved_count = filtered.len();
    f.render_stateful_widget(list, list_area, &mut app.saved_jql_state);
    f.render_widget(hints, hint_area);
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        list_area,
        app.saved_jql_state.offset(),
        saved_count,
    ));
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
    let theme_count = ThemeName::ALL.len();
    f.render_stateful_widget(list, popup_area, &mut app.theme_state);
    app.hit_zones.popup = Some(popup_area);
    app.hit_zones.picker = Some(picker_hit_for_bordered(
        popup_area,
        app.theme_state.offset(),
        theme_count,
    ));
}

fn render_text_popup(
    f: &mut Frame,
    title: &str,
    lines: &[String],
    area: Rect,
    palette: Palette,
) -> Rect {
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
    popup_area
}

fn render_help_popup(f: &mut Frame, area: Rect, palette: Palette, scroll_offset: u16) -> Rect {
    let popup_area = centered_rect(70, 95, area);

    // Header (rendered separately, always at top of popup).
    let header_lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Sections — each is rendered atomically (never split across columns).
    // Order matters: first section ends up in first column.
    let issue_list_section: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            "Issue List:",
            Style::default().fg(palette.tab_active),
        )),
        Line::from("  ↑/k       Move up"),
        Line::from("  ↓/j       Move down"),
        Line::from("  Enter     Open split detail view"),
        Line::from("  a         Assign selected issue"),
        Line::from("  b         Add bulk worklog (confirm before submit)"),
        Line::from("  c         Create new issue"),
        Line::from("  C         Column settings"),
        Line::from("  e         Edit selected issue"),
        Line::from("  g         Show config file"),
        Line::from("  l         Set labels"),
        Line::from("  m         Edit components"),
        Line::from("  M         Move issue to another project (native, not clone+delete)"),
        Line::from("  n         Scan and open Jira mention notifications"),
        Line::from("  o         Open issue in browser"),
        Line::from("  p         Open saved queries (run/create/edit/delete)"),
        Line::from("  q         Quit the TUI"),
        Line::from("  r         Refresh list"),
        Line::from("  R         Mark selected notification issue as read"),
        Line::from("  s         Add to sprint"),
        Line::from("  S         Show server info"),
        Line::from("  t         Transition issue"),
        Line::from("  T         Open theme picker"),
        Line::from("  u         Upload attachment"),
        Line::from("  v         Edit fix versions"),
        Line::from("  V         Browse project fix versions + backlog preview"),
        Line::from("            Enter refreshes preview, n creates, e edits metadata"),
        Line::from("  w         Add single worklog"),
        Line::from("  y         Change issue type (native Jira move semantics)"),
        Line::from("  /         Search with JQL"),
        Line::from("  :         Add the same comment to many issues (JQL or explicit keys)"),
        Line::from("  ;         Add comment"),
        Line::from("  ?         Show help"),
    ];

    let detail_view_section: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            "Detail View:",
            Style::default().fg(palette.tab_active),
        )),
        Line::from("  Esc / q   Back to list"),
        Line::from("  ←/→ / Tab Switch detail tabs"),
        Line::from("  Summary / Versions / Comments / Worklog / Attachments / Subtasks / Links"),
        Line::from("  e,y,M,a,;,w,b,m,v,s,u,t,o also work from detail view"),
    ];

    let mouse_section: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            "Mouse:",
            Style::default().fg(palette.tab_active),
        )),
        Line::from("  Click row             Select issue"),
        Line::from("  Double-click row      Open detail view"),
        Line::from("  Click detail tab      Switch tab"),
        Line::from("  Click detail pane     Focus detail (from list)"),
        Line::from("  Click picker option   Apply (single-select) / toggle (multi-select)"),
        Line::from("  Click outside popup   Close popup (same as Esc)"),
        Line::from("  Click [?] / [🔔]      Open help / notifications"),
        Line::from("  Drag splitter         Resize list/detail (saved in prefs)"),
        Line::from("  Scroll wheel          Move selection in list or picker"),
    ];

    let sections: Vec<Vec<Line<'static>>> =
        vec![issue_list_section, detail_view_section, mouse_section];

    // Flat single-column form (header + sections joined by blank lines + footer).
    let mut single_col: Vec<Line<'static>> = header_lines.clone();
    for (i, sec) in sections.iter().enumerate() {
        single_col.extend(sec.iter().cloned());
        if i + 1 < sections.len() {
            single_col.push(Line::from(""));
        }
    }
    single_col.push(Line::from(""));
    single_col.push(Line::from(Span::styled(
        "Press any key to close",
        Style::default().fg(palette.muted),
    )));
    let lines = single_col;

    f.render_widget(Clear, popup_area);
    let total_lines = lines.len() as u16;

    // Decide layout: if the single-column form fits, use it. Otherwise try a
    // 2-column section-aware split (sections stay atomic). If even that
    // overflows, fall back to scroll on single column.
    let probe_block = Block::default().borders(Borders::ALL);
    let probe_inner = probe_block.inner(popup_area);
    let inner_h = probe_inner.height;

    let needs_layout = total_lines > inner_h;

    // Greedy 2-col packing: keep header in col1, then fill col1 with whole
    // sections until the next one would overflow; remaining sections + the
    // close hint go to col2. Each section is followed by 1 blank separator
    // line in its column.
    let header_h = header_lines.len() as u16;
    let close_hint = [
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(palette.muted),
        )),
    ];

    let mut two_col_fits = false;
    let mut col1: Vec<Line<'static>> = header_lines.clone();
    let mut col2: Vec<Line<'static>> = Vec::new();
    if needs_layout {
        let mut col1_h: u16 = header_h;
        let mut split_at: usize = sections.len();
        for (i, sec) in sections.iter().enumerate() {
            let sec_h = sec.len() as u16 + 1; // +1 for blank separator
            if col1_h.saturating_add(sec_h) > inner_h {
                split_at = i;
                break;
            }
            col1.extend(sec.iter().cloned());
            col1.push(Line::from(""));
            col1_h = col1_h.saturating_add(sec_h);
        }
        if split_at < sections.len() && split_at > 0 {
            for (i, sec) in sections[split_at..].iter().enumerate() {
                col2.extend(sec.iter().cloned());
                if i + 1 < sections.len() - split_at {
                    col2.push(Line::from(""));
                }
            }
            col2.extend(close_hint.iter().cloned());
            let col2_h = col2.len() as u16;
            two_col_fits = col2_h <= inner_h;
        }
    }

    let title_bottom = if needs_layout && !two_col_fits {
        " ↑/↓ PgUp/PgDn: scroll   Esc/?: close "
    } else {
        " Esc / ?: close "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.focus_border))
        .title(" Help ")
        .title_bottom(title_bottom)
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    if two_col_fits {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);
        if let (Some(&left_rect), Some(&right_rect)) = (cols.first(), cols.get(1)) {
            let left = Paragraph::new(col1).wrap(Wrap { trim: false });
            let right = Paragraph::new(col2).wrap(Wrap { trim: false });
            f.render_widget(left, left_rect);
            f.render_widget(right, right_rect);
        }
    } else {
        let max_scroll = total_lines.saturating_sub(inner.height);
        let scroll = scroll_offset.min(max_scroll);
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        f.render_widget(paragraph, inner);
    }
    popup_area
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

    let Some(&middle) = popup_layout.get(1) else {
        return r;
    };

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(middle);

    horizontal.get(1).copied().unwrap_or(r)
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

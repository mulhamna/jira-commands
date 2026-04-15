use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use jira_core::{model::Issue, JiraClient};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    List,
    View,
    Help,
}

pub struct App {
    issues: Vec<Issue>,
    table_state: TableState,
    mode: Mode,
}

impl App {
    fn new(issues: Vec<Issue>) -> Self {
        let mut table_state = TableState::default();
        if !issues.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            issues,
            table_state,
            mode: Mode::List,
        }
    }

    fn selected_issue(&self) -> Option<&Issue> {
        self.table_state.selected().and_then(|i| self.issues.get(i))
    }

    fn next(&mut self) {
        if self.issues.is_empty() {
            return;
        }
        let i = self
            .table_state
            .selected()
            .map(|i| (i + 1).min(self.issues.len() - 1))
            .unwrap_or(0);
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.issues.is_empty() {
            return;
        }
        let i = self
            .table_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.table_state.select(Some(i));
    }
}

pub async fn run_tui(client: JiraClient, project: Option<String>) -> Result<()> {
    // Fetch issues
    let jql = if let Some(proj) = &project {
        format!("project = {proj} ORDER BY updated DESC")
    } else {
        "ORDER BY updated DESC".to_string()
    };

    let result = client.search_issues(&jql, None, Some(50)).await?;
    let issues = result.issues;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(issues);

    let res = run_event_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.mode {
                Mode::List => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Enter => {
                        if app.selected_issue().is_some() {
                            app.mode = Mode::View;
                        }
                    }
                    KeyCode::Char('?') => app.mode = Mode::Help,
                    _ => {}
                },
                Mode::View => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                        app.mode = Mode::List;
                    }
                    KeyCode::Char('?') => app.mode = Mode::Help,
                    _ => {}
                },
                Mode::Help => {
                    app.mode = Mode::List;
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout: header + content + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);

    // Header
    let title = match app.mode {
        Mode::List => " Jira CLI — Issue List ",
        Mode::View => " Jira CLI — Issue Detail ",
        Mode::Help => " Jira CLI — Help ",
    };
    let header = Paragraph::new(title).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(header, chunks[0]);

    // Footer / keybindings
    let help_text = match app.mode {
        Mode::List => " ↑/k: up  ↓/j: down  Enter: view  ?: help  q: quit",
        Mode::View => " Esc/Backspace: back  ?: help  q: quit",
        Mode::Help => " Any key: close help",
    };
    let footer = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[2]);

    match app.mode {
        Mode::List => render_list(f, app, chunks[1]),
        Mode::View => render_detail(f, app, chunks[1]),
        Mode::Help => {
            render_list(f, app, chunks[1]);
            render_help_popup(f, size);
        }
    }
}

fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["Key", "Type", "Priority", "Status", "Assignee", "Summary"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.issues.iter().map(|issue| {
        let summary = if issue.summary.len() > 40 {
            format!("{}…", &issue.summary[..39])
        } else {
            issue.summary.clone()
        };

        let cells = vec![
            Cell::from(issue.key.clone()).style(Style::default().fg(Color::Cyan)),
            Cell::from(issue.issue_type.clone()),
            Cell::from(issue.priority.clone().unwrap_or_else(|| "-".into())),
            Cell::from(issue.status.clone())
                .style(Style::default().fg(status_color(&issue.status))),
            Cell::from(issue.assignee.clone().unwrap_or_else(|| "-".into())),
            Cell::from(summary),
        ];

        Row::new(cells).height(1)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(18),
            Constraint::Length(20),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Issues ({}) ", app.issues.len())),
    )
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

    // Build static lines first, then extend with conditional ones
    #[allow(clippy::vec_init_then_push)]
    let lines: Vec<Line> = {
        let mut v = vec![
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
            v.push(field_line("Priority", p));
        }
        if let Some(a) = &issue.assignee {
            v.push(field_line("Assignee", a));
        }
        if let Some(r) = &issue.reporter {
            v.push(field_line("Reporter", r));
        }
        v.push(field_line("Created", created));
        v.push(field_line("Updated", updated));
        if let Some(desc) = &issue.description {
            let text = jira_core::adf::adf_to_text(desc);
            if !text.is_empty() {
                v.push(Line::from(""));
                v.push(Line::from(Span::styled(
                    "Description:",
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )));
                v.push(Line::from(""));
                for line in text.lines() {
                    v.push(Line::from(format!("  {line}")));
                }
            }
        }
        v
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Issue Detail "),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(50, 60, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "List View:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  ↑ / k     — Move up"),
        Line::from("  ↓ / j     — Move down"),
        Line::from("  Enter     — View issue detail"),
        Line::from("  ?         — Show this help"),
        Line::from("  q / Esc   — Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Detail View:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  Esc / Bsp — Back to list"),
        Line::from("  ?         — Show this help"),
        Line::from("  q         — Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false });

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

fn status_color(status: &str) -> Color {
    match status.to_lowercase().as_str() {
        s if s.contains("done") || s.contains("closed") || s.contains("resolved") => Color::Green,
        s if s.contains("progress") || s.contains("review") => Color::Yellow,
        s if s.contains("blocked") || s.contains("impediment") => Color::Red,
        _ => Color::White,
    }
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

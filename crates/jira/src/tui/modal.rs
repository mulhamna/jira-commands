use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use tui_textarea::TextArea;

use super::picker::PickerOption;
use super::theme::Palette;

#[derive(Debug, Clone)]
pub(super) enum ModalKind {
    EditIssue {
        key: String,
    },
    AddComment {
        key: String,
    },
    UploadAttachment {
        key: String,
    },
    AddWorklog {
        key: String,
    },
    AddBulkWorklog {
        key: String,
    },
    ChangeIssueType {
        key: String,
        current_project: String,
    },
    MoveIssue {
        key: String,
        current_issue_type: String,
    },
}

impl ModalKind {
    pub(super) fn title(&self) -> String {
        match self {
            ModalKind::EditIssue { key } => format!(" Edit {key} "),
            ModalKind::AddComment { key } => format!(" Comment on {key} "),
            ModalKind::UploadAttachment { key } => format!(" Attach to {key} "),
            ModalKind::AddWorklog { key } => format!(" Log Work on {key} "),
            ModalKind::AddBulkWorklog { key } => format!(" Bulk Worklog on {key} "),
            ModalKind::ChangeIssueType { key, .. } => format!(" Change Type: {key} "),
            ModalKind::MoveIssue { key, .. } => format!(" Move Issue: {key} "),
        }
    }

    pub(super) fn hint(&self) -> &'static str {
        match self {
            ModalKind::EditIssue { .. } => " Tab: next field   Ctrl+S: save   Esc: cancel ",
            ModalKind::AddComment { .. } => " Ctrl+S: send   Esc: cancel ",
            ModalKind::UploadAttachment { .. } => " Enter/Ctrl+S: upload   Esc: cancel ",
            ModalKind::AddWorklog { .. } => " Tab: next field   Ctrl+S: log   Esc: cancel ",
            ModalKind::AddBulkWorklog { .. } => {
                " Tab: next field   Ctrl+S: log range   Esc: cancel "
            }
            ModalKind::ChangeIssueType { .. } => {
                " Tab: next field   Ctrl+S: change type   Esc: cancel "
            }
            ModalKind::MoveIssue { .. } => " Tab: next field   Ctrl+S: move issue   Esc: cancel ",
        }
    }
}

pub(super) struct ModalField {
    pub label: &'static str,
    pub area: TextArea<'static>,
    pub multiline: bool,
}

pub(super) struct Modal {
    pub kind: ModalKind,
    pub fields: Vec<ModalField>,
    pub focus: usize,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub confirm_token: Option<String>,
    pub busy: bool,
    pub mention_active: bool,
    pub mention_query: String,
    pub mention_options: Vec<PickerOption>,
    pub mention_state: ListState,
    pub mention_map: Vec<(String, String)>,
    pub mention_cache: HashMap<String, Vec<PickerOption>>,
}

impl Modal {
    pub(super) fn edit_issue(key: String, summary: String, description: String) -> Self {
        let mut summary_area = TextArea::from(vec![summary]);
        summary_area.set_cursor_line_style(Style::default());

        let mut desc_lines: Vec<String> = description.split('\n').map(|s| s.to_string()).collect();
        if desc_lines.is_empty() {
            desc_lines.push(String::new());
        }
        let mut desc_area = TextArea::from(desc_lines);
        desc_area.set_cursor_line_style(Style::default());

        Self {
            kind: ModalKind::EditIssue { key },
            fields: vec![
                ModalField {
                    label: "Summary",
                    area: summary_area,
                    multiline: false,
                },
                ModalField {
                    label: "Description (markdown)",
                    area: desc_area,
                    multiline: true,
                },
            ],
            focus: 0,
            error: None,
            notice: None,
            confirm_token: None,
            busy: false,
            mention_active: false,
            mention_query: String::new(),
            mention_options: Vec::new(),
            mention_state: ListState::default(),
            mention_map: Vec::new(),
            mention_cache: HashMap::new(),
        }
    }

    pub(super) fn add_comment(key: String) -> Self {
        let mut area = TextArea::default();
        area.set_cursor_line_style(Style::default());
        Self {
            kind: ModalKind::AddComment { key },
            fields: vec![ModalField {
                label: "Comment",
                area,
                multiline: true,
            }],
            focus: 0,
            error: None,
            notice: None,
            confirm_token: None,
            busy: false,
            mention_active: false,
            mention_query: String::new(),
            mention_options: Vec::new(),
            mention_state: ListState::default(),
            mention_map: Vec::new(),
            mention_cache: HashMap::new(),
        }
    }

    pub(super) fn upload_attachment(key: String) -> Self {
        let mut area = TextArea::default();
        area.set_cursor_line_style(Style::default());
        Self {
            kind: ModalKind::UploadAttachment { key },
            fields: vec![ModalField {
                label: "File path",
                area,
                multiline: false,
            }],
            focus: 0,
            error: None,
            notice: None,
            confirm_token: None,
            busy: false,
            mention_active: false,
            mention_query: String::new(),
            mention_options: Vec::new(),
            mention_state: ListState::default(),
            mention_map: Vec::new(),
            mention_cache: HashMap::new(),
        }
    }

    pub(super) fn add_worklog(key: String) -> Self {
        let make = |placeholder: &'static str| {
            let mut a = TextArea::default();
            a.set_cursor_line_style(Style::default());
            a.set_placeholder_text(placeholder);
            a
        };
        Self {
            kind: ModalKind::AddWorklog { key },
            fields: vec![
                ModalField {
                    label: "Time spent  (e.g. 2h, 30m, 1d, 1h 30m)",
                    area: make("required"),
                    multiline: false,
                },
                ModalField {
                    label: "Date  (YYYY-MM-DD, blank = today)",
                    area: make("blank = today"),
                    multiline: false,
                },
                ModalField {
                    label: "Start time  (HH:MM, blank = now)",
                    area: make("blank = now"),
                    multiline: false,
                },
                ModalField {
                    label: "Comment  (optional)",
                    area: make(""),
                    multiline: true,
                },
            ],
            focus: 0,
            error: None,
            notice: None,
            confirm_token: None,
            busy: false,
            mention_active: false,
            mention_query: String::new(),
            mention_options: Vec::new(),
            mention_state: ListState::default(),
            mention_map: Vec::new(),
            mention_cache: HashMap::new(),
        }
    }

    pub(super) fn add_bulk_worklog(key: String) -> Self {
        let make = |placeholder: &'static str| {
            let mut a = TextArea::default();
            a.set_cursor_line_style(Style::default());
            a.set_placeholder_text(placeholder);
            a
        };
        Self {
            kind: ModalKind::AddBulkWorklog { key },
            fields: vec![
                ModalField {
                    label: "Time spent  (e.g. 2h, 30m, 1d, 1h 30m)",
                    area: make("required"),
                    multiline: false,
                },
                ModalField {
                    label: "From date  (YYYY-MM-DD)",
                    area: make("required"),
                    multiline: false,
                },
                ModalField {
                    label: "To date  (YYYY-MM-DD)",
                    area: make("required"),
                    multiline: false,
                },
                ModalField {
                    label: "Start time  (HH:MM, optional)",
                    area: make("blank = now"),
                    multiline: false,
                },
                ModalField {
                    label: "Exclude weekends  (y/N)",
                    area: make("n"),
                    multiline: false,
                },
                ModalField {
                    label: "Comment  (optional)",
                    area: make(""),
                    multiline: true,
                },
            ],
            focus: 0,
            error: None,
            notice: None,
            confirm_token: None,
            busy: false,
            mention_active: false,
            mention_query: String::new(),
            mention_options: Vec::new(),
            mention_state: ListState::default(),
            mention_map: Vec::new(),
            mention_cache: HashMap::new(),
        }
    }

    pub(super) fn change_issue_type(
        key: String,
        current_project: String,
        current_issue_type: String,
    ) -> Self {
        let mut area = TextArea::from(vec![current_issue_type.clone()]);
        area.set_cursor_line_style(Style::default());
        area.set_placeholder_text("Task / Story / Bug / Epic / ...");
        Self {
            kind: ModalKind::ChangeIssueType {
                key,
                current_project,
            },
            fields: vec![ModalField {
                label: "Target issue type name",
                area,
                multiline: false,
            }],
            focus: 0,
            error: None,
            notice: None,
            confirm_token: None,
            busy: false,
            mention_active: false,
            mention_query: String::new(),
            mention_options: Vec::new(),
            mention_state: ListState::default(),
            mention_map: Vec::new(),
            mention_cache: HashMap::new(),
        }
    }

    pub(super) fn move_issue(
        key: String,
        _current_project: String,
        current_issue_type: String,
    ) -> Self {
        let mut project_area = TextArea::default();
        project_area.set_cursor_line_style(Style::default());
        project_area.set_placeholder_text("target project key, e.g. OTHER");

        let mut type_area = TextArea::from(vec![current_issue_type.clone()]);
        type_area.set_cursor_line_style(Style::default());
        type_area.set_placeholder_text("blank = keep current issue type name");

        Self {
            kind: ModalKind::MoveIssue {
                key,
                current_issue_type,
            },
            fields: vec![
                ModalField {
                    label: "Target project key",
                    area: project_area,
                    multiline: false,
                },
                ModalField {
                    label: "Target issue type name  (blank = keep current type name)",
                    area: type_area,
                    multiline: false,
                },
            ],
            focus: 0,
            error: None,
            notice: None,
            confirm_token: None,
            busy: false,
            mention_active: false,
            mention_query: String::new(),
            mention_options: Vec::new(),
            mention_state: ListState::default(),
            mention_map: Vec::new(),
            mention_cache: HashMap::new(),
        }
    }

    pub(super) fn next_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.focus = (self.focus + 1) % self.fields.len();
    }

    pub(super) fn prev_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.focus = (self.focus + self.fields.len() - 1) % self.fields.len();
    }

    pub(super) fn field_text(&self, idx: usize) -> String {
        self.fields
            .get(idx)
            .map(|f| f.area.lines().join("\n"))
            .unwrap_or_default()
    }

    pub(super) fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
        self.notice = None;
        self.confirm_token = None;
        self.busy = false;
    }

    pub(super) fn set_notice(&mut self, msg: impl Into<String>, token: Option<String>) {
        self.notice = Some(msg.into());
        self.error = None;
        self.confirm_token = token;
        self.busy = false;
    }

    pub(super) fn clear_notice(&mut self) {
        self.notice = None;
        self.confirm_token = None;
    }
}

pub(super) enum ModalOutcome {
    Cancel,
    Submit,
    Continue,
    MentionQueryChanged,
    MentionSelected(usize),
}

/// Forward a key event to the focused textarea, with submit/cancel/nav handling.
pub(super) fn handle_modal_key(modal: &mut Modal, key: KeyEvent) -> ModalOutcome {
    if modal.busy {
        return ModalOutcome::Continue;
    }

    if modal.mention_active {
        return handle_mention_key(modal, key.code);
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('@'), _) if matches!(modal.kind, ModalKind::AddComment { .. }) => {
            modal.mention_active = true;
            modal.mention_query.clear();
            modal.mention_options.clear();
            ModalOutcome::MentionQueryChanged
        }
        (KeyCode::Esc, _) => ModalOutcome::Cancel,
        (KeyCode::Char('s'), KeyModifiers::CONTROL)
        | (KeyCode::Char('S'), KeyModifiers::CONTROL) => ModalOutcome::Submit,
        (KeyCode::Tab, _) => {
            modal.clear_notice();
            modal.next_field();
            ModalOutcome::Continue
        }
        (KeyCode::BackTab, _) => {
            modal.clear_notice();
            modal.prev_field();
            ModalOutcome::Continue
        }
        (KeyCode::Enter, _) => {
            let multiline = modal
                .fields
                .get(modal.focus)
                .map(|f| f.multiline)
                .unwrap_or(false);
            if !multiline {
                return ModalOutcome::Submit;
            }
            modal.clear_notice();
            forward_to_focus(modal, key);
            ModalOutcome::Continue
        }
        _ => {
            modal.clear_notice();
            forward_to_focus(modal, key);
            ModalOutcome::Continue
        }
    }
}

fn handle_mention_key(modal: &mut Modal, code: KeyCode) -> ModalOutcome {
    match code {
        KeyCode::Esc => {
            modal.mention_active = false;
            modal.mention_query.clear();
            modal.mention_options.clear();
            modal.mention_state = ListState::default();
            ModalOutcome::Continue
        }
        KeyCode::Down => {
            let i = modal
                .mention_state
                .selected()
                .map(|i| (i + 1).min(modal.mention_options.len().saturating_sub(1)))
                .unwrap_or(0);
            modal.mention_state.select(Some(i));
            ModalOutcome::Continue
        }
        KeyCode::Up => {
            let i = modal
                .mention_state
                .selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            modal.mention_state.select(Some(i));
            ModalOutcome::Continue
        }
        KeyCode::Enter => {
            if let Some(idx) = modal.mention_state.selected() {
                if idx < modal.mention_options.len() {
                    return ModalOutcome::MentionSelected(idx);
                }
            }
            ModalOutcome::Continue
        }
        KeyCode::Backspace => {
            modal.mention_query.pop();
            ModalOutcome::MentionQueryChanged
        }
        KeyCode::Char(c) => {
            modal.mention_query.push(c);
            ModalOutcome::MentionQueryChanged
        }
        _ => ModalOutcome::Continue,
    }
}

fn forward_to_focus(modal: &mut Modal, key: KeyEvent) {
    if let Some(field) = modal.fields.get_mut(modal.focus) {
        field.area.input(key);
    }
}

pub(super) fn render_modal(f: &mut Frame, modal: &mut Modal, palette: Palette, area: Rect) {
    let outer = side_modal_rect(area);
    f.render_widget(Clear, outer);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.focus_border))
        .title(modal.kind.title())
        .title_bottom(modal.kind.hint());
    let inner = block.inner(outer);
    f.render_widget(block, outer);

    let mut constraints: Vec<Constraint> = Vec::new();
    for field in &modal.fields {
        if field.multiline {
            constraints.push(Constraint::Min(5));
        } else {
            constraints.push(Constraint::Length(3));
        }
    }
    constraints.push(Constraint::Length(2)); // status row

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (idx, field) in modal.fields.iter().enumerate() {
        let focused = idx == modal.focus;
        let border_style = if focused {
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.focus_border)
        };

        let mut area_widget = field.area.clone();
        area_widget.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(format!(" {} ", field.label)),
        );
        if !focused {
            area_widget.set_cursor_style(Style::default());
        }
        f.render_widget(&area_widget, chunks[idx]);
    }

    let status_idx = chunks.len() - 1;
    let status_text = if modal.busy {
        "Working...".to_string()
    } else if let Some(err) = &modal.error {
        format!("⚠ {err}")
    } else if let Some(notice) = &modal.notice {
        format!("! {notice}")
    } else {
        String::new()
    };
    let status_style = if modal.error.is_some() {
        Style::default().fg(Color::Red)
    } else if modal.notice.is_some() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(palette.muted)
    };
    let status = Paragraph::new(status_text).style(status_style);
    f.render_widget(status, chunks[status_idx]);

    if modal.mention_active {
        render_mention_overlay(f, modal, palette, inner);
    }
}

fn render_mention_overlay(f: &mut Frame, modal: &mut Modal, palette: Palette, area: Rect) {
    let count = modal.mention_options.len() as u16;
    let height = (count + 2).clamp(3, 10);
    let y = area.y + area.height.saturating_sub(height + 2);
    let overlay = Rect {
        x: area.x,
        y,
        width: area.width,
        height,
    };

    let title = if modal.mention_query.is_empty() {
        " @mention — type 2+ chars ".to_string()
    } else if modal.mention_query.chars().count() < 2 {
        format!(
            " @{}  (type {} more) ",
            modal.mention_query,
            2 - modal.mention_query.chars().count()
        )
    } else {
        format!(" @{} ", modal.mention_query)
    };

    let items: Vec<ListItem> = modal
        .mention_options
        .iter()
        .map(|opt| ListItem::new(opt.label.clone()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.focus_border))
                .title(title)
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(palette.highlight)
                .fg(palette.header_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_widget(Clear, overlay);
    f.render_stateful_widget(list, overlay, &mut modal.mention_state);
}

fn side_modal_rect(area: Rect) -> Rect {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(area);
    panels[1]
}

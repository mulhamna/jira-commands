use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tui_textarea::TextArea;

use super::theme::Palette;

#[derive(Debug, Clone)]
pub(super) enum ModalKind {
    EditIssue { key: String },
    AddComment { key: String },
    UploadAttachment { key: String },
    AddWorklog { key: String },
}

impl ModalKind {
    pub(super) fn title(&self) -> String {
        match self {
            ModalKind::EditIssue { key } => format!(" Edit {key} "),
            ModalKind::AddComment { key } => format!(" Comment on {key} "),
            ModalKind::UploadAttachment { key } => format!(" Attach to {key} "),
            ModalKind::AddWorklog { key } => format!(" Log Work on {key} "),
        }
    }

    pub(super) fn hint(&self) -> &'static str {
        match self {
            ModalKind::EditIssue { .. } => " Tab: next field   Ctrl+S: save   Esc: cancel ",
            ModalKind::AddComment { .. } => " Ctrl+S: send   Esc: cancel ",
            ModalKind::UploadAttachment { .. } => " Enter/Ctrl+S: upload   Esc: cancel ",
            ModalKind::AddWorklog { .. } => " Tab: next field   Ctrl+S: log   Esc: cancel ",
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
    pub busy: bool,
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
            busy: false,
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
            busy: false,
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
            busy: false,
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
            busy: false,
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
        self.busy = false;
    }
}

pub(super) enum ModalOutcome {
    Cancel,
    Submit,
    Continue,
}

/// Forward a key event to the focused textarea, with submit/cancel/nav handling.
pub(super) fn handle_modal_key(modal: &mut Modal, key: KeyEvent) -> ModalOutcome {
    if modal.busy {
        return ModalOutcome::Continue;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Esc, _) => ModalOutcome::Cancel,
        (KeyCode::Char('s'), KeyModifiers::CONTROL)
        | (KeyCode::Char('S'), KeyModifiers::CONTROL) => ModalOutcome::Submit,
        (KeyCode::Tab, _) => {
            modal.next_field();
            ModalOutcome::Continue
        }
        (KeyCode::BackTab, _) => {
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
            forward_to_focus(modal, key);
            ModalOutcome::Continue
        }
        _ => {
            forward_to_focus(modal, key);
            ModalOutcome::Continue
        }
    }
}

fn forward_to_focus(modal: &mut Modal, key: KeyEvent) {
    if let Some(field) = modal.fields.get_mut(modal.focus) {
        field.area.input(key);
    }
}

pub(super) fn render_modal(f: &mut Frame, modal: &Modal, palette: Palette, area: Rect) {
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
    } else {
        String::new()
    };
    let status_style = if modal.error.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(palette.muted)
    };
    let status = Paragraph::new(status_text).style(status_style);
    f.render_widget(status, chunks[status_idx]);
}

fn side_modal_rect(area: Rect) -> Rect {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(area);
    panels[1]
}

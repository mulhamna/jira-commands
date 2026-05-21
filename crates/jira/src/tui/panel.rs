use jira_core::model::{Comment, RemoteLink, Worklog};
use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DetailTab {
    Summary,
    Comments,
    Worklog,
    Attachments,
    Subtasks,
    Links,
}

impl DetailTab {
    pub(super) const ALL: [DetailTab; 6] = [
        DetailTab::Summary,
        DetailTab::Comments,
        DetailTab::Worklog,
        DetailTab::Attachments,
        DetailTab::Subtasks,
        DetailTab::Links,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            DetailTab::Summary => "Summary",
            DetailTab::Comments => "Comments",
            DetailTab::Worklog => "Worklog",
            DetailTab::Attachments => "Attachments",
            DetailTab::Subtasks => "Subtasks",
            DetailTab::Links => "Links",
        }
    }

    pub(super) fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }

    pub(super) fn next(self) -> Self {
        let i = (self.index() + 1) % Self::ALL.len();
        Self::ALL[i]
    }

    pub(super) fn prev(self) -> Self {
        let i = (self.index() + Self::ALL.len() - 1) % Self::ALL.len();
        Self::ALL[i]
    }
}

#[derive(Debug, Default)]
pub(super) struct DetailData {
    pub(super) issue_key: String,
    pub(super) comments: Option<Vec<Comment>>,
    pub(super) worklogs: Option<Vec<Worklog>>,
    pub(super) remote_links: Option<Vec<RemoteLink>>,
}

impl DetailData {
    pub(super) fn reset_for(&mut self, key: &str) {
        if self.issue_key != key {
            *self = DetailData {
                issue_key: key.to_string(),
                ..Default::default()
            };
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    List,
    Detail,
}

/// Click targets captured during the previous frame's render pass.
/// Mouse events are hit-tested against these zones to translate pointer
/// coordinates into the same `AppAction` variants the keyboard handler emits.
#[derive(Debug, Default, Clone)]
pub(super) struct HitZones {
    /// Issue list table area (full bounding box including header + border).
    pub(super) list: Option<Rect>,
    /// Detail pane area (right side in master-detail layout).
    pub(super) detail_pane: Option<Rect>,
    /// One rect per visible detail tab header (Summary, Comments, ...).
    pub(super) detail_tabs: Vec<(Rect, DetailTab)>,
    /// The currently visible picker list (transition, assignee, etc.).
    /// `area` is the inner content area (no border) where rows are drawn.
    pub(super) picker: Option<PickerHit>,
    /// The currently visible popup bounding box. Clicks *outside* this
    /// rect while a popup mode is active should close the popup.
    pub(super) popup: Option<Rect>,
    /// Vertical splitter column between list + detail panes (1-col-wide rect).
    pub(super) splitter: Option<Rect>,
    /// Full master-detail area. Needed to recompute split percentage on drag.
    pub(super) master_detail_area: Option<Rect>,
    /// Footer "[?]" help button. Click synthesizes a `?` keypress.
    pub(super) help_button: Option<Rect>,
    /// Footer "[🔔 n]" notifications button — only set when unread > 0.
    /// Click synthesizes an `n` keypress.
    pub(super) notif_button: Option<Rect>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PickerHit {
    /// Inner area (excluding borders) where rows are rendered.
    pub(super) area: Rect,
    /// The list state's current scroll offset (0 = first option at top).
    pub(super) offset: usize,
    /// Total number of options in the picker (bounds-check upper limit).
    pub(super) count: usize,
}

impl HitZones {
    pub(super) fn clear(&mut self) {
        self.list = None;
        self.detail_pane = None;
        self.detail_tabs.clear();
        self.picker = None;
        self.popup = None;
        self.splitter = None;
        self.master_detail_area = None;
        self.help_button = None;
        self.notif_button = None;
    }
}

impl PickerHit {
    /// Translate a pointer row to an option index, or `None` if the row
    /// lands on a border / past the last option.
    pub(super) fn row_to_index(&self, row: u16) -> Option<usize> {
        if row < self.area.y || row >= self.area.y.saturating_add(self.area.height) {
            return None;
        }
        let visible = (row - self.area.y) as usize;
        let idx = self.offset.saturating_add(visible);
        (idx < self.count).then_some(idx)
    }
}

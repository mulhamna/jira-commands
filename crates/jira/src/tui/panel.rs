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
    /// Visible picker option rects, indexed by option position.
    pub(super) picker_rows: Vec<Rect>,
    /// The currently visible popup bounding box. Clicks *outside* this
    /// rect while a popup mode is active should close the popup.
    pub(super) popup: Option<Rect>,
}

impl HitZones {
    pub(super) fn clear(&mut self) {
        self.list = None;
        self.detail_pane = None;
        self.detail_tabs.clear();
        self.picker_rows.clear();
        self.popup = None;
    }
}

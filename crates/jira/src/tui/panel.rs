use jira_core::model::{Comment, Worklog};
use serde_json::Value;

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

    pub(super) fn from_index(idx: usize) -> Option<Self> {
        Self::ALL.get(idx).copied()
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
    pub(super) remote_links: Option<Vec<Value>>,
    pub(super) selected_comment: usize,
    pub(super) selected_worklog: usize,
    pub(super) selected_attachment: usize,
    pub(super) selected_subtask: usize,
    pub(super) selected_link: usize,
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

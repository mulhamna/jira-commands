pub mod api;
pub mod attachment;
pub mod auth;
pub mod board;
pub mod comment;
pub mod issue;
pub mod jql;
pub mod link;
pub mod meta;
pub mod notify;
pub mod plan;
pub mod project;
pub mod request;
pub mod shared;
pub mod sprint;
pub mod watcher;
pub mod worklog;

#[derive(Debug, Clone, Default)]
pub struct JiraApp;

#[cfg(test)]
mod tests;

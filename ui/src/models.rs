use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RepoInfo {
    pub(crate) path: String,
    pub(crate) branch: Option<String>,
    #[serde(rename = "lastActivityUtc")]
    pub(crate) last_activity_utc: Option<DateTime<Utc>>,
    pub(crate) source: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortBy {
    LastActivity,
    Path,
    Branch,
}

pub(crate) enum ScanMsg {
    Log(String),
    Done(Result<Vec<RepoInfo>, String>),
}

#[derive(Debug, Clone)]
pub(crate) struct CommitInfo {
    pub(crate) short_hash: String,
    pub(crate) full_hash: String,
    pub(crate) author: String,
    pub(crate) date: String,
    pub(crate) subject: String,
}

pub(crate) enum HistoryMsg {
    Done(Result<Vec<CommitInfo>, String>),
}

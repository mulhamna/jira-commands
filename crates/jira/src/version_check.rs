use std::time::Duration;

use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/mulhamna/jira-commands/releases/latest";

#[derive(Debug, Clone)]
pub struct UpdateNotice {
    pub latest: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    html_url: String,
}

pub async fn check_for_update() -> Option<UpdateNotice> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build()
        .ok()?;

    let release = client
        .get(LATEST_RELEASE_API)
        .header(USER_AGENT, format!("jirac/{CURRENT_VERSION}"))
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<ReleaseResponse>()
        .await
        .ok()?;

    let latest = normalize_version(&release.tag_name);
    if is_newer(&latest, CURRENT_VERSION) {
        Some(UpdateNotice {
            latest,
            url: release.html_url,
        })
    } else {
        None
    }
}

fn normalize_version(raw: &str) -> String {
    raw.trim().trim_start_matches('v').to_string()
}

fn is_newer(latest: &str, current: &str) -> bool {
    compare_versions(latest, current).is_gt()
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts = parse_version(a);
    let b_parts = parse_version(b);
    a_parts.cmp(&b_parts)
}

fn parse_version(input: &str) -> Vec<u64> {
    normalize_version(input)
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

pub fn cli_message(notice: &UpdateNotice) -> String {
    format!(
        "update available: jirac {} -> {} ({})",
        CURRENT_VERSION, notice.latest, notice.url
    )
}

pub fn tui_message(notice: &UpdateNotice) -> String {
    format!(
        "Update available: jirac {} → {}  (release: {})",
        CURRENT_VERSION, notice.latest, notice.url
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("0.22.1", "0.22.0"));
        assert!(is_newer("0.23.0", "0.22.9"));
        assert!(!is_newer("0.22.0", "0.22.0"));
        assert!(!is_newer("0.21.9", "0.22.0"));
        assert!(is_newer("v1.2.0", "1.1.9"));
    }
}

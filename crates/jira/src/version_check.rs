use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
// jirac (`vX.Y.Z`) and jirac-mcp (`jira-mcp-vX.Y.Z`) release from the same repo,
// so `releases/latest` can point at either lane. List recent releases and filter.
const RELEASES_API: &str =
    "https://api.github.com/repos/mulhamna/jira-commands/releases?per_page=10";

#[derive(Debug, Clone)]
pub struct UpdateNotice {
    pub latest: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Latest published release whose tag is `<tag_prefix>X.Y.Z`.
pub async fn latest_release(tag_prefix: &str, timeout: Duration) -> Result<UpdateNotice> {
    let releases = reqwest::Client::builder()
        .timeout(timeout)
        .build()?
        .get(RELEASES_API)
        .header(USER_AGENT, format!("jirac/{CURRENT_VERSION}"))
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .context("Failed to reach GitHub releases API")?
        .error_for_status()?
        .json::<Vec<ReleaseResponse>>()
        .await
        .context("Failed to parse GitHub releases response")?;

    pick_release(releases, tag_prefix)
        .with_context(|| format!("No published release found with tag prefix '{tag_prefix}'"))
}

fn pick_release(releases: Vec<ReleaseResponse>, tag_prefix: &str) -> Option<UpdateNotice> {
    releases.into_iter().find_map(|release| {
        if release.draft || release.prerelease {
            return None;
        }
        let version = release.tag_name.strip_prefix(tag_prefix)?;
        version
            .starts_with(|c: char| c.is_ascii_digit())
            .then(|| UpdateNotice {
                latest: version.to_string(),
                url: release.html_url,
            })
    })
}

pub async fn check_for_update() -> Option<UpdateNotice> {
    let release = latest_release("v", Duration::from_millis(1200))
        .await
        .ok()?;
    is_newer(&release.latest, CURRENT_VERSION).then_some(release)
}

fn normalize_version(raw: &str) -> String {
    raw.trim().trim_start_matches('v').to_string()
}

pub(crate) fn is_newer(latest: &str, current: &str) -> bool {
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

    #[test]
    fn picks_release_for_matching_lane_only() {
        let release = |tag: &str, prerelease: bool| ReleaseResponse {
            tag_name: tag.to_string(),
            html_url: format!("https://example.test/{tag}"),
            draft: false,
            prerelease,
        };
        let releases = || {
            vec![
                release("jira-mcp-v2.7.0", false),
                release("v2.12.0", true),
                release("v2.11.0", false),
                release("jira-mcp-v2.6.0", false),
            ]
        };

        assert_eq!(pick_release(releases(), "v").unwrap().latest, "2.11.0");
        assert_eq!(
            pick_release(releases(), "jira-mcp-v").unwrap().latest,
            "2.7.0"
        );
        assert!(pick_release(releases(), "nope-v").is_none());
    }
}

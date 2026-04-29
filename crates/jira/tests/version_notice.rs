use jira_commands::version_check::{cli_message, tui_message, UpdateNotice};

#[test]
fn cli_message_mentions_upgrade_path_and_release_url() {
    let notice = UpdateNotice {
        latest: "0.23.0".into(),
        url: "https://github.com/mulhamna/jira-commands/releases/tag/v0.23.0".into(),
    };

    let message = cli_message(&notice);

    assert!(message.contains("update available: jirac"));
    assert!(message.contains("-> 0.23.0"));
    assert!(message.contains("/releases/tag/v0.23.0"));
}

#[test]
fn tui_message_mentions_upgrade_path_and_release_url() {
    let notice = UpdateNotice {
        latest: "0.23.0".into(),
        url: "https://github.com/mulhamna/jira-commands/releases/tag/v0.23.0".into(),
    };

    let message = tui_message(&notice);

    assert!(message.contains("Update available: jirac"));
    assert!(message.contains("→ 0.23.0"));
    assert!(message.contains("/releases/tag/v0.23.0"));
}

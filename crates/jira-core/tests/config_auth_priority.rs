use jira_core::config::{JiraAuthType, JiraConfig, JiraDeployment};

#[test]
fn cloud_config_requires_user_identity_but_data_center_pat_does_not() {
    let cloud = JiraConfig {
        profile_name: Some("cloud".into()),
        base_url: "https://example.atlassian.net".into(),
        email: "dev@example.com".into(),
        token: Some("token".into()),
        project: None,
        timeout_secs: 30,
        deployment: JiraDeployment::Cloud,
        auth_type: JiraAuthType::CloudApiToken,
        api_version: 3,
    };
    let data_center = JiraConfig {
        profile_name: Some("dc".into()),
        base_url: "https://jira.example.com".into(),
        email: String::new(),
        token: Some("token".into()),
        project: None,
        timeout_secs: 30,
        deployment: JiraDeployment::DataCenter,
        auth_type: JiraAuthType::DataCenterPat,
        api_version: 2,
    };

    assert!(cloud.requires_user_identity());
    assert!(!data_center.requires_user_identity());
}

#[test]
fn token_presence_tracks_non_empty_trimmed_tokens() {
    let mut config = JiraConfig::default();
    assert!(!config.token_present());

    config.token = Some("   ".into());
    assert!(!config.token_present());

    config.token = Some("secret".into());
    assert!(config.token_present());
}

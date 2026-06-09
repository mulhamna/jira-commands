use std::time::Duration;

use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Client, Response, StatusCode,
};
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::{
    adf::markdown_to_adf,
    config::{JiraAuthType, JiraConfig},
    error::{JiraError, Result},
    model::{
        attachment::Attachment,
        comment::Comment,
        component::Component,
        field::Field,
        issue::{
            CreateIssueRequest, CreateIssueRequestV2, Issue, RawIssue, RawSearchResponse,
            SearchResult, UpdateIssueRequest,
        },
        issue_type::IssueType,
        link::{IssueLink, IssueLinkType},
        remote_link::RemoteLink,
        sprint::Sprint,
        transition::Transition,
        user::JiraUser,
        version::{CreateProjectVersionRequest, ProjectVersion, UpdateProjectVersionRequest},
        worklog::Worklog,
    },
};

const MAX_RETRIES: u32 = 3;

#[derive(serde::Deserialize)]
struct MyselfResponse {
    #[serde(rename = "accountId")]
    account_id: Option<String>,
    #[serde(rename = "timeZone")]
    time_zone: Option<String>,
}

#[derive(Clone)]
pub struct JiraClient {
    http: Client,
    config: JiraConfig,
}

impl JiraClient {
    pub fn new(config: JiraConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to build HTTP client");

        Self { http, config }
    }

    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    fn platform_url(&self, path: &str) -> String {
        format!(
            "{}/rest/api/{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.api_version,
            path
        )
    }

    fn auth_headers(&self) -> Result<HeaderMap> {
        self.build_auth_headers(true)
    }

    /// Auth headers without Content-Type — required for multipart uploads.
    fn auth_headers_no_content_type(&self) -> Result<HeaderMap> {
        self.build_auth_headers(false)
    }

    fn build_auth_headers(&self, include_content_type: bool) -> Result<HeaderMap> {
        let token = self.config.token.as_deref().ok_or_else(|| {
            JiraError::Auth("No token configured. Run `jirac auth login` first.".into())
        })?;

        let auth_value = match self.config.auth_type {
            JiraAuthType::CloudApiToken | JiraAuthType::DataCenterBasic => {
                let credentials = base64_encode(&format!("{}:{}", self.config.email, token));
                format!("Basic {credentials}")
            }
            JiraAuthType::DataCenterPat => format!("Bearer {token}"),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .map_err(|e| JiraError::Auth(format!("Invalid auth header: {e}")))?,
        );
        if include_content_type {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        Ok(headers)
    }

    async fn get_myself_info(&self) -> Result<MyselfResponse> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/myself");

        let http = &self.http;
        self.request(|| http.get(&url).headers(headers.clone()))
            .await
    }

    /// Get the current authenticated user's accountId.
    pub async fn get_myself(&self) -> Result<String> {
        let me = self.get_myself_info().await?;
        me.account_id.ok_or_else(|| JiraError::Api {
            status: 0,
            message: "Could not get accountId from /myself".into(),
        })
    }

    /// Get the current authenticated user's Jira timezone (IANA name), if available.
    pub async fn get_myself_timezone(&self) -> Result<Option<String>> {
        Ok(self.get_myself_info().await?.time_zone)
    }

    /// Resolve an assignee string to a Jira accountId.
    ///
    /// - `"me"` → current user's accountId via /myself
    /// - contains `@` → search by email, return first match's accountId
    /// - anything else → treated as a raw accountId and returned as-is
    async fn resolve_assignee_account_id(&self, s: &str) -> Result<String> {
        if s == "me" {
            return self.get_myself().await;
        }
        if !s.contains('@') {
            return Ok(s.to_string());
        }
        // Resolve email → accountId via user search
        let users = self.search_users(s).await?;
        users
            .iter()
            .find(|u| {
                u.email_address
                    .as_deref()
                    .map(|e| e.eq_ignore_ascii_case(s))
                    .unwrap_or(false)
            })
            .or_else(|| users.first())
            .map(|u| u.account_id.clone())
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: format!("User not found: {s}"),
            })
    }

    /// Send a request, retrying on HTTP 429 according to the `Retry-After` header.
    /// Returns the first non-429 response, or `RateLimit` after `MAX_RETRIES` attempts.
    async fn execute_with_retry(
        &self,
        builder_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let response = builder_fn().send().await?;

            if response.status() != StatusCode::TOO_MANY_REQUESTS {
                return Ok(response);
            }

            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60);

            warn!("Rate limited. Retrying after {}s", retry_after);

            if attempt >= MAX_RETRIES {
                return Err(JiraError::RateLimit { retry_after });
            }

            tokio::time::sleep(Duration::from_secs(retry_after)).await;
        }
    }

    /// Core request method with rate-limit retry logic.
    async fn request<T>(&self, builder_fn: impl Fn() -> reqwest::RequestBuilder) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self.execute_with_retry(builder_fn).await?;
        handle_response(response).await
    }

    /// Core request method for responses with no body (204 No Content).
    async fn request_no_body(
        &self,
        builder_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<()> {
        let response = self.execute_with_retry(builder_fn).await?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().await.unwrap_or_default();
        if status == StatusCode::NOT_FOUND {
            return Err(JiraError::NotFound(body));
        }
        Err(JiraError::Api {
            status: status.as_u16(),
            message: body,
        })
    }

    /// Multipart request with rate-limit retry (for attachment uploads).
    async fn request_multipart<T>(
        &self,
        builder_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self.execute_with_retry(builder_fn).await?;
        handle_response(response).await
    }

    /// Search issues using JQL with cursor-based pagination.
    pub async fn search_issues(
        &self,
        jql: &str,
        next_page_token: Option<&str>,
        max_results: Option<u32>,
    ) -> Result<SearchResult> {
        const DEFAULT_FIELDS: &[&str] = &[
            "summary",
            "status",
            "assignee",
            "reporter",
            "priority",
            "issuetype",
            "project",
            "created",
            "updated",
            "description",
        ];
        self.search_issues_with_fields(jql, next_page_token, max_results, DEFAULT_FIELDS)
            .await
    }

    /// Search issues with an explicit `fields` list (e.g. `["*all"]`, `["*navigable"]`,
    /// or specific field IDs like `["summary", "components", "customfield_10010"]`).
    pub async fn search_issues_with_fields(
        &self,
        jql: &str,
        next_page_token: Option<&str>,
        max_results: Option<u32>,
        fields: &[&str],
    ) -> Result<SearchResult> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/search/jql");

        let mut body = json!({
            "jql": jql,
            "maxResults": max_results.unwrap_or(50),
            "fields": fields,
        });

        if let Some(token) = next_page_token {
            body["nextPageToken"] = json!(token);
        }

        debug!("Searching JQL: {}", jql);

        let http = &self.http;
        let raw: RawSearchResponse = self
            .request(|| http.post(&url).headers(headers.clone()).json(&body))
            .await?;

        Ok(SearchResult {
            issues: raw.issues.into_iter().map(|r| r.into_issue()).collect(),
            next_page_token: raw.next_page_token,
            total: raw.total,
        })
    }

    /// Fetch all visible fields (system + custom) from the Jira instance.
    /// Endpoint: `GET /rest/api/3/field`.
    pub async fn list_fields(&self) -> Result<Vec<Field>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/field");

        #[derive(serde::Deserialize)]
        struct FieldEntry {
            id: String,
            name: String,
            #[serde(default)]
            schema: Option<Value>,
        }

        let http = &self.http;
        let raw: Vec<FieldEntry> = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(raw
            .into_iter()
            .map(|f| {
                let field_type = f
                    .schema
                    .as_ref()
                    .and_then(|s| s.get("type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Field {
                    id: f.id,
                    name: f.name,
                    field_type,
                    required: false,
                    schema: f.schema,
                    allowed_values: None,
                }
            })
            .collect())
    }

    /// Fetch a single issue by key.
    pub async fn get_issue(&self, key: &str) -> Result<Issue> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{key}"));

        let http = &self.http;
        let raw: RawIssue = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(raw.into_issue())
    }

    /// Create a new issue.
    ///
    /// Deprecated in favor of [`JiraClient::create_issue_v2`], which supports
    /// custom fields, labels, components, parent, fix versions, and ADF
    /// descriptions. This method is retained for backward compatibility and
    /// will be removed in a future release.
    #[deprecated(
        since = "0.40.0",
        note = "Use `create_issue_v2` — supports custom fields, labels, components, parent, fix versions"
    )]
    pub async fn create_issue(&self, req: CreateIssueRequest) -> Result<Issue> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/issue");

        let description_adf = req.description.as_deref().map(markdown_to_adf);

        let mut fields = json!({
            "project": { "key": req.project_key },
            "summary": req.summary,
            "issuetype": { "name": req.issue_type }
        });

        if let Some(adf) = description_adf {
            fields["description"] = adf;
        }

        if let Some(assignee) = &req.assignee {
            let account_id = self.resolve_assignee_account_id(assignee).await?;
            fields["assignee"] = json!({ "accountId": account_id });
        }

        if let Some(priority) = &req.priority {
            fields["priority"] = json!({ "name": priority });
        }

        let body = json!({ "fields": fields });

        #[derive(serde::Deserialize)]
        struct CreateResponse {
            key: String,
        }

        let http = &self.http;
        let resp: CreateResponse = self
            .request(|| http.post(&url).headers(headers.clone()).json(&body))
            .await?;

        // Fetch the full issue after creation
        self.get_issue(&resp.key).await
    }

    /// Update an existing issue.
    pub async fn update_issue(&self, key: &str, req: UpdateIssueRequest) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{key}"));

        let mut fields = json!({});

        if let Some(summary) = &req.summary {
            fields["summary"] = json!(summary);
        }
        if let Some(adf) = &req.description_adf {
            fields["description"] = adf.clone();
        } else if let Some(description) = &req.description {
            fields["description"] = markdown_to_adf(description);
        }
        if let Some(assignee) = &req.assignee {
            let account_id = self.resolve_assignee_account_id(assignee).await?;
            fields["assignee"] = json!({ "accountId": account_id });
        }
        if let Some(priority) = &req.priority {
            fields["priority"] = json!({ "name": priority });
        }
        if let Some(labels) = &req.labels {
            fields["labels"] = json!(labels);
        }
        if let Some(components) = &req.components {
            fields["components"] = json!(components
                .iter()
                .map(|c| json!({"name": c}))
                .collect::<Vec<_>>());
        }
        if let Some(fix_versions) = &req.fix_versions {
            fields["fixVersions"] = json!(fix_versions
                .iter()
                .map(|v| json!({"name": v}))
                .collect::<Vec<_>>());
        }
        if let Some(parent) = &req.parent {
            fields["parent"] = json!({ "key": parent });
        }
        for (field_id, value) in &req.custom_fields {
            fields[field_id] = value.to_api_json();
        }

        let body = json!({ "fields": fields });

        let http = &self.http;
        self.request_no_body(|| http.put(&url).headers(headers.clone()).json(&body))
            .await
    }

    /// Delete an issue.
    pub async fn delete_issue(&self, key: &str) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{key}"));

        let http = &self.http;
        self.request_no_body(|| http.delete(&url).headers(headers.clone()))
            .await
    }

    /// Get fields available for a project (runtime field resolution — no hardcoding).
    pub async fn get_project_fields(&self, project_key: &str) -> Result<Vec<Field>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/createmeta/{project_key}/issuetypes"));

        #[derive(serde::Deserialize)]
        struct IssueTypeMeta {
            #[serde(rename = "issueTypes")]
            issue_types: Vec<IssueTypeDetail>,
        }

        #[derive(serde::Deserialize)]
        struct IssueTypeDetail {
            fields: Option<std::collections::HashMap<String, FieldMeta>>,
        }

        #[derive(serde::Deserialize)]
        struct FieldMeta {
            name: String,
            required: bool,
            schema: Option<Value>,
        }

        let http = &self.http;
        let meta: IssueTypeMeta = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        let mut fields: Vec<Field> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for it in meta.issue_types {
            if let Some(field_map) = it.fields {
                for (id, meta) in field_map {
                    if seen.insert(id.clone()) {
                        let field_type = meta
                            .schema
                            .as_ref()
                            .and_then(|s| s.get("type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        fields.push(Field {
                            id,
                            name: meta.name,
                            field_type,
                            required: meta.required,
                            schema: meta.schema,
                            allowed_values: None,
                        });
                    }
                }
            }
        }

        Ok(fields)
    }

    /// Get server info (used to detect Jira tier).
    pub async fn get_server_info(&self) -> Result<Value> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/serverInfo");

        let http = &self.http;
        self.request(|| http.get(&url).headers(headers.clone()))
            .await
    }

    /// Transition an issue to a new status.
    pub async fn transition_issue(&self, key: &str, transition_id: &str) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{key}/transitions"));

        let body = json!({
            "transition": { "id": transition_id }
        });

        let http = &self.http;
        self.request_no_body(|| http.post(&url).headers(headers.clone()).json(&body))
            .await
    }

    /// Get available transitions for an issue.
    pub async fn get_transitions(&self, key: &str) -> Result<Vec<Transition>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{key}/transitions"));

        #[derive(serde::Deserialize)]
        struct TransitionsResponse {
            transitions: Vec<Transition>,
        }

        let http = &self.http;
        let resp: TransitionsResponse = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(resp.transitions)
    }

    /// Get available issue types for a project (id + name).
    pub async fn get_issue_types(&self, project_key: &str) -> Result<Vec<IssueType>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/createmeta/{project_key}/issuetypes"));

        #[derive(serde::Deserialize)]
        struct MetaResponse {
            #[serde(rename = "issueTypes")]
            issue_types: Vec<IssueType>,
        }

        let http = &self.http;
        let resp: MetaResponse = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(resp.issue_types)
    }

    /// Resolve an issue type by exact or case-insensitive name within a project.
    pub async fn get_issue_type_by_name(
        &self,
        project_key: &str,
        issue_type_name: &str,
    ) -> Result<IssueType> {
        let issue_types = self.get_issue_types(project_key).await?;

        issue_types
            .iter()
            .find(|it| it.name == issue_type_name)
            .or_else(|| {
                issue_types
                    .iter()
                    .find(|it| it.name.eq_ignore_ascii_case(issue_type_name))
            })
            .cloned()
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: format!(
                    "Issue type '{}' not found in project {}",
                    issue_type_name, project_key
                ),
            })
    }

    /// Get fields for a specific issue type within a project (with allowed values).
    pub async fn get_fields_for_issue_type(
        &self,
        project_key: &str,
        issue_type_id: &str,
    ) -> Result<Vec<Field>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!(
            "/issue/createmeta/{project_key}/issuetypes/{issue_type_id}"
        ));

        #[derive(serde::Deserialize)]
        struct FieldMetaResponse {
            fields: FieldCollection,
        }

        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum FieldCollection {
            Map(std::collections::HashMap<String, FieldMetaMap>),
            List(Vec<FieldMetaEntry>),
        }

        #[derive(serde::Deserialize)]
        struct FieldMetaMap {
            name: String,
            required: bool,
            schema: Option<Value>,
            #[serde(rename = "allowedValues")]
            allowed_values: Option<Vec<Value>>,
        }

        #[derive(serde::Deserialize)]
        struct FieldMetaEntry {
            #[serde(rename = "fieldId")]
            field_id: Option<String>,
            key: Option<String>,
            name: String,
            required: bool,
            schema: Option<Value>,
            #[serde(rename = "allowedValues")]
            allowed_values: Option<Vec<Value>>,
        }

        let http = &self.http;
        let resp: FieldMetaResponse = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        let fields = match resp.fields {
            FieldCollection::Map(fields) => fields
                .into_iter()
                .map(|(id, meta)| {
                    let field_type = meta
                        .schema
                        .as_ref()
                        .and_then(|s| s.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    Field {
                        id,
                        name: meta.name,
                        field_type,
                        required: meta.required,
                        schema: meta.schema,
                        allowed_values: meta.allowed_values,
                    }
                })
                .collect(),
            FieldCollection::List(fields) => fields
                .into_iter()
                .map(|meta| {
                    let field_type = meta
                        .schema
                        .as_ref()
                        .and_then(|s| s.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let id = meta.field_id.or(meta.key).unwrap_or_default();

                    Field {
                        id,
                        name: meta.name,
                        field_type,
                        required: meta.required,
                        schema: meta.schema,
                        allowed_values: meta.allowed_values,
                    }
                })
                .collect(),
        };

        Ok(fields)
    }

    /// Search Jira users by query string (for User field autocomplete).
    pub async fn search_users(&self, query: &str) -> Result<Vec<JiraUser>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/user/search");

        let http = &self.http;
        self.request(|| {
            http.get(&url)
                .headers(headers.clone())
                .query(&[("query", query), ("maxResults", "20")])
        })
        .await
    }

    /// List components available within a project.
    pub async fn get_project_components(&self, project_key: &str) -> Result<Vec<Component>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/project/{project_key}/components"));

        let http = &self.http;
        self.request(|| http.get(&url).headers(headers.clone()))
            .await
    }

    /// List fix versions available within a project.
    pub async fn get_project_versions(&self, project_key: &str) -> Result<Vec<ProjectVersion>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/project/{project_key}/versions"));

        let http = &self.http;
        let versions: Vec<ProjectVersion> = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(versions)
    }

    /// Create a project version in Jira.
    pub async fn create_project_version(
        &self,
        request: &CreateProjectVersionRequest,
    ) -> Result<ProjectVersion> {
        let path = format!("/rest/api/{}/version", self.config.api_version);
        let body = serde_json::to_value(request)?;
        let value = self
            .raw_request("POST", &path, Some(body))
            .await?
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Empty response when creating project version".into(),
            })?;
        let version: ProjectVersion =
            serde_json::from_value(value).map_err(|e| JiraError::Api {
                status: 0,
                message: format!("Failed to parse created project version: {e}"),
            })?;
        Ok(version)
    }

    /// Update project version metadata like release/start dates or release state.
    pub async fn update_project_version(
        &self,
        version_id: &str,
        request: &UpdateProjectVersionRequest,
    ) -> Result<ProjectVersion> {
        let path = format!(
            "/rest/api/{}/version/{}",
            self.config.api_version, version_id
        );
        let body = serde_json::to_value(request)?;
        let value = self
            .raw_request("PUT", &path, Some(body))
            .await?
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Empty response when updating project version".into(),
            })?;
        let version: ProjectVersion =
            serde_json::from_value(value).map_err(|e| JiraError::Api {
                status: 0,
                message: format!("Failed to parse updated project version: {e}"),
            })?;
        Ok(version)
    }

    fn parse_sprint_value(&self, value: &Value, board_id: Option<u64>) -> Option<Sprint> {
        let id = value.get("id").and_then(|v| v.as_u64())?;
        Some(Sprint {
            id,
            name: value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            state: value
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            board_id,
            goal: value
                .get("goal")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            start_date: value
                .get("startDate")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            end_date: value
                .get("endDate")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            complete_date: value
                .get("completeDate")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            })
    }

    fn paged_values<'a>(response: &'a Value) -> &'a [Value] {
        response
            .get("values")
            .and_then(|v| v.as_array())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn page_is_last(response: &Value, fetched_count: usize) -> bool {
        if let Some(is_last) = response.get("isLast").and_then(|v| v.as_bool()) {
            return is_last;
        }
        if let Some(total) = response.get("total").and_then(|v| v.as_u64()) {
            let start_at = response.get("startAt").and_then(|v| v.as_u64()).unwrap_or(0);
            return start_at + fetched_count as u64 >= total;
        }
        if let Some(max_results) = response.get("maxResults").and_then(|v| v.as_u64()) {
            return fetched_count < max_results as usize;
        }
        true
    }

    /// List project sprints for the given sprint states via the Agile API.
    pub async fn list_sprints_for_project_with_states(
        &self,
        project_key: &str,
        states: &[&str],
    ) -> Result<Vec<Sprint>> {
        let mut board_ids = Vec::new();
        let mut board_start_at = 0u64;

        loop {
            let boards_path = format!(
                "/rest/agile/1.0/board?projectKeyOrId={project_key}&maxResults=100&startAt={board_start_at}"
            );
            let boards_resp = self
                .raw_request("GET", &boards_path, None)
                .await?
                .unwrap_or_default();
            let boards = Self::paged_values(&boards_resp);

            board_ids.extend(
                boards
                    .iter()
                    .filter_map(|b| b.get("id").and_then(|id| id.as_u64())),
            );

            if Self::page_is_last(&boards_resp, boards.len()) {
                break;
            }
            board_start_at += boards.len() as u64;
        }

        let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut sprints: Vec<Sprint> = Vec::new();
        let state_filter = states.join(",");

        for board_id in board_ids {
            let mut sprint_start_at = 0u64;
            loop {
                let sprint_path = format!(
                    "/rest/agile/1.0/board/{board_id}/sprint?state={state_filter}&maxResults=200&startAt={sprint_start_at}"
                );
                let resp = match self.raw_request("GET", &sprint_path, None).await {
                    Ok(Some(resp)) => resp,
                    Ok(None) => break,
                    Err(JiraError::NotFound(_)) => break,
                    Err(err) => return Err(err),
                };
                let values = Self::paged_values(&resp);
                for value in values {
                    let Some(sprint) = self.parse_sprint_value(value, Some(board_id)) else {
                        continue;
                    };
                    if !seen.insert(sprint.id) {
                        continue;
                    }
                    sprints.push(sprint);
                }

                if Self::page_is_last(&resp, values.len()) {
                    break;
                }
                sprint_start_at += values.len() as u64;
            }
        }

        sprints.sort_by(|a, b| {
            let order = |s: &str| match s {
                "active" => 0u8,
                "future" => 1u8,
                _ => 2u8,
            };
            order(&a.state)
                .cmp(&order(&b.state))
                .then(a.name.cmp(&b.name))
        });

        Ok(sprints)
    }

    /// List active and future sprints for a project via the Agile API.
    pub async fn list_sprints_for_project(&self, project_key: &str) -> Result<Vec<Sprint>> {
        self.list_sprints_for_project_with_states(project_key, &["active", "future"])
            .await
    }

    /// Create a sprint on a Jira board.
    pub async fn create_sprint(
        &self,
        board_id: u64,
        name: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        goal: Option<&str>,
    ) -> Result<Sprint> {
        let body = json!({
            "name": name,
            "originBoardId": board_id,
            "startDate": start_date,
            "endDate": end_date,
            "goal": goal,
        });
        let value = self
            .raw_request("POST", "/rest/agile/1.0/sprint", Some(body))
            .await?
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Empty response when creating sprint".into(),
            })?;
        self.parse_sprint_value(&value, Some(board_id))
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Failed to parse created sprint".into(),
            })
    }

    /// Update sprint metadata or lifecycle state.
    pub async fn update_sprint(&self, sprint_id: u64, body: Value) -> Result<Sprint> {
        let value = self
            .raw_request(
                "PUT",
                &format!("/rest/agile/1.0/sprint/{sprint_id}"),
                Some(body),
            )
            .await?
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Empty response when updating sprint".into(),
            })?;
        self.parse_sprint_value(&value, None)
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Failed to parse updated sprint".into(),
            })
    }

    /// Delete a sprint.
    pub async fn delete_sprint(&self, sprint_id: u64) -> Result<()> {
        self.raw_request(
            "DELETE",
            &format!("/rest/agile/1.0/sprint/{sprint_id}"),
            None,
        )
        .await?;
        Ok(())
    }

    /// Add an issue to a sprint (Agile API).
    pub async fn add_issue_to_sprint(&self, sprint_id: u64, issue_key: &str) -> Result<()> {
        let path = format!("/rest/agile/1.0/sprint/{sprint_id}/issue");
        let body = json!({ "issues": [issue_key] });
        self.raw_request("POST", &path, Some(body)).await?;
        Ok(())
    }

    /// Add a comment using pre-built ADF JSON (skips Markdown conversion).
    pub async fn add_comment_adf(&self, issue_key: &str, adf: Value) -> Result<Comment> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/comment"));
        let payload = json!({ "body": adf });
        let http = &self.http;
        let raw: Value = self
            .request(|| http.post(&url).headers(headers.clone()).json(&payload))
            .await?;
        Comment::from_value(&raw, issue_key).ok_or_else(|| JiraError::Api {
            status: 0,
            message: "Failed to parse comment".into(),
        })
    }

    /// Upload a file as an attachment to an issue.
    pub async fn upload_attachment(
        &self,
        issue_key: &str,
        file_path: &std::path::Path,
    ) -> Result<Vec<Attachment>> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();
        let bytes = std::fs::read(file_path)?;
        let mime = mime_guess::from_path(file_path)
            .first_or_octet_stream()
            .to_string();

        self.upload_attachment_bytes(issue_key, &file_name, bytes, Some(&mime))
            .await
    }

    /// Upload an in-memory attachment to an issue.
    pub async fn upload_attachment_bytes(
        &self,
        issue_key: &str,
        file_name: &str,
        bytes: Vec<u8>,
        media_type: Option<&str>,
    ) -> Result<Vec<Attachment>> {
        use reqwest::{header::HeaderValue, multipart};

        let headers = self.auth_headers_no_content_type()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/attachments"));
        let mime = media_type
            .map(|value| value.to_string())
            .or_else(|| {
                mime_guess::from_path(file_name)
                    .first_raw()
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let http = &self.http;
        let raw_attachments: Vec<Value> = self
            .request_multipart(|| {
                let part = multipart::Part::bytes(bytes.clone())
                    .file_name(file_name.to_string())
                    .mime_str(&mime)
                    .expect("invalid mime type");
                let form = multipart::Form::new().part("file", part);

                let mut req_headers = headers.clone();
                req_headers.insert("X-Atlassian-Token", HeaderValue::from_static("no-check"));

                http.post(&url).headers(req_headers).multipart(form)
            })
            .await?;

        Ok(raw_attachments
            .iter()
            .filter_map(Attachment::from_value)
            .collect())
    }

    /// Create a new issue with dynamic custom fields.
    pub async fn create_issue_v2(&self, req: CreateIssueRequestV2) -> Result<Issue> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/issue");

        let description_adf = req
            .description_adf
            .or_else(|| req.description.as_deref().map(markdown_to_adf));

        let mut fields = json!({
            "project": { "key": req.project_key },
            "summary": req.summary,
            "issuetype": { "name": req.issue_type }
        });

        if let Some(adf) = description_adf {
            fields["description"] = adf;
        }
        if let Some(assignee) = &req.assignee {
            let account_id = self.resolve_assignee_account_id(assignee).await?;
            fields["assignee"] = json!({ "accountId": account_id });
        }
        if let Some(priority) = &req.priority {
            fields["priority"] = json!({ "name": priority });
        }
        if !req.labels.is_empty() {
            fields["labels"] = json!(req.labels);
        }
        if !req.components.is_empty() {
            fields["components"] = json!(req
                .components
                .iter()
                .map(|c| json!({"name": c}))
                .collect::<Vec<_>>());
        }
        if let Some(parent) = &req.parent {
            fields["parent"] = json!({ "key": parent });
        }
        if !req.fix_versions.is_empty() {
            fields["fixVersions"] = json!(req
                .fix_versions
                .iter()
                .map(|v| json!({"name": v}))
                .collect::<Vec<_>>());
        }
        for (field_id, value) in &req.custom_fields {
            fields[field_id] = value.to_api_json();
        }

        let body = json!({ "fields": fields });

        #[derive(serde::Deserialize)]
        struct CreateResponse {
            key: String,
        }

        let http = &self.http;
        let resp: CreateResponse = self
            .request(|| http.post(&url).headers(headers.clone()).json(&body))
            .await?;

        self.get_issue(&resp.key).await
    }

    // ── Comments ─────────────────────────────────────────────────────────────

    /// List all comments for an issue.
    pub async fn get_comments(&self, issue_key: &str) -> Result<Vec<Comment>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/comment"));

        #[derive(serde::Deserialize)]
        struct CommentResponse {
            comments: Vec<Value>,
        }

        let http = &self.http;
        let resp: CommentResponse = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(resp
            .comments
            .iter()
            .filter_map(|v| Comment::from_value(v, issue_key))
            .collect())
    }

    /// Add a comment to an issue.
    pub async fn add_comment(&self, issue_key: &str, body: &str) -> Result<Comment> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/comment"));

        let payload = json!({
            "body": markdown_to_adf(body)
        });

        let http = &self.http;
        let raw: Value = self
            .request(|| http.post(&url).headers(headers.clone()).json(&payload))
            .await?;

        Comment::from_value(&raw, issue_key).ok_or_else(|| JiraError::Api {
            status: 0,
            message: "Failed to parse comment".into(),
        })
    }

    // ── Worklog ──────────────────────────────────────────────────────────────

    /// List all worklogs for an issue.
    pub async fn get_worklogs(&self, issue_key: &str) -> Result<Vec<Worklog>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/worklog"));

        #[derive(serde::Deserialize)]
        struct WorklogResponse {
            worklogs: Vec<Value>,
        }

        let http = &self.http;
        let resp: WorklogResponse = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(resp
            .worklogs
            .iter()
            .filter_map(|v| Worklog::from_value(v, issue_key))
            .collect())
    }

    /// Add a worklog entry to an issue.
    /// `time_spent` uses Jira format: "2h 30m", "1d", "45m"
    /// `started` is optional ISO 8601 timestamp; defaults to now if None.
    pub async fn add_worklog(
        &self,
        issue_key: &str,
        time_spent: &str,
        comment: Option<&str>,
        started: Option<&str>,
    ) -> Result<Worklog> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/worklog"));

        // Jira requires started in "2006-01-02T15:04:05.000+0000" format
        let started_str = started
            .map(|s| s.to_string())
            .unwrap_or_else(current_jira_timestamp);

        let mut body = json!({
            "timeSpent": time_spent,
            "started": started_str,
        });

        if let Some(c) = comment {
            body["comment"] = markdown_to_adf(c);
        }

        let http = &self.http;
        let raw: Value = self
            .request(|| http.post(&url).headers(headers.clone()).json(&body))
            .await?;

        Worklog::from_value(&raw, issue_key).ok_or_else(|| JiraError::Api {
            status: 0,
            message: "Failed to parse worklog".into(),
        })
    }

    /// Delete a worklog entry.
    pub async fn delete_worklog(&self, issue_key: &str, worklog_id: &str) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/worklog/{worklog_id}"));

        let http = &self.http;
        self.request_no_body(|| http.delete(&url).headers(headers.clone()))
            .await
    }

    /// Delete a comment from an issue.
    pub async fn delete_comment(&self, issue_key: &str, comment_id: &str) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/comment/{comment_id}"));

        let http = &self.http;
        self.request_no_body(|| http.delete(&url).headers(headers.clone()))
            .await
    }

    /// Delete an attachment by ID.
    pub async fn delete_attachment(&self, attachment_id: &str) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/attachment/{attachment_id}"));

        let http = &self.http;
        self.request_no_body(|| http.delete(&url).headers(headers.clone()))
            .await
    }

    /// List remote links on an issue.
    pub async fn get_remote_links(&self, issue_key: &str) -> Result<Vec<RemoteLink>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/remotelink"));

        let http = &self.http;
        self.request(|| http.get(&url).headers(headers.clone()))
            .await
    }

    /// Add a remote link to an issue.
    pub async fn add_remote_link(
        &self,
        issue_key: &str,
        url_str: &str,
        title: &str,
    ) -> Result<Value> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/remotelink"));

        let payload = json!({
            "object": {
                "url": url_str,
                "title": title,
            }
        });

        let http = &self.http;
        self.request(|| http.post(&url).headers(headers.clone()).json(&payload))
            .await
    }

    /// Delete a remote link from an issue.
    pub async fn delete_remote_link(&self, issue_key: &str, link_id: &str) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{issue_key}/remotelink/{link_id}"));

        let http = &self.http;
        self.request_no_body(|| http.delete(&url).headers(headers.clone()))
            .await
    }

    // ── Bulk ops ─────────────────────────────────────────────────────────────

    /// Fetch ALL issues matching a JQL query using cursor-based pagination.
    /// Respects the Atlassian safeguard: max 500 pages.
    pub async fn get_all_issues(&self, jql: &str) -> Result<Vec<Issue>> {
        let mut all_issues = Vec::new();
        let mut next_page_token: Option<String> = None;
        let mut iterations = 0u32;
        const MAX_ITERATIONS: u32 = 500;

        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                break;
            }

            let result = self
                .search_issues(jql, next_page_token.as_deref(), Some(100))
                .await?;

            all_issues.extend(result.issues);

            match result.next_page_token {
                Some(token) => next_page_token = Some(token),
                None => break,
            }
        }

        Ok(all_issues)
    }

    /// Archive a batch of issues by key. Jira accepts up to 1000 per request.
    pub async fn archive_issues(&self, issue_keys: &[String]) -> Result<()> {
        if issue_keys.is_empty() {
            return Ok(());
        }
        let headers = self.auth_headers()?;
        let url = self.platform_url("/issue/archive");

        // Batch in chunks of 1000
        for chunk in issue_keys.chunks(1000) {
            let body = json!({ "issueIdsOrKeys": chunk });
            let http = &self.http;
            // Archive returns 200 with a body — use request() not request_no_body()
            let _: Value = self
                .request(|| http.put(&url).headers(headers.clone()).json(&body))
                .await?;
        }

        Ok(())
    }

    /// Move an issue using Jira's native bulk move API.
    pub async fn move_issue(
        &self,
        issue_key: &str,
        target_project_key: &str,
        target_issue_type_id: &str,
        target_parent: Option<&str>,
    ) -> Result<Issue> {
        let mapping_key = match target_parent {
            Some(parent) => format!("{target_project_key},{target_issue_type_id},{parent}"),
            None => format!("{target_project_key},{target_issue_type_id}"),
        };

        let body = json!({
            "sendBulkNotification": true,
            "targetToSourcesMapping": {
                mapping_key: {
                    "inferClassificationDefaults": true,
                    "inferFieldDefaults": true,
                    "inferStatusDefaults": true,
                    "inferSubtaskTypeDefault": true,
                    "issueIdsOrKeys": [issue_key]
                }
            }
        });

        let submitted = self
            .raw_request("POST", "/rest/api/3/bulk/issues/move", Some(body))
            .await?
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Bulk move returned an empty response".into(),
            })?;

        let task_id = submitted
            .get("taskId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Bulk move response did not include a taskId".into(),
            })?;

        const MAX_POLLS: usize = 60;
        for _ in 0..MAX_POLLS {
            tokio::time::sleep(Duration::from_secs(2)).await;

            let progress = self
                .raw_request("GET", &format!("/rest/api/3/bulk/queue/{task_id}"), None)
                .await?
                .ok_or_else(|| JiraError::Api {
                    status: 0,
                    message: "Bulk move progress returned an empty response".into(),
                })?;

            match progress
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
            {
                "COMPLETE" => return self.get_issue(issue_key).await,
                "FAILED" | "DEAD" | "CANCELLED" => {
                    return Err(JiraError::Api {
                        status: 0,
                        message: progress.to_string(),
                    });
                }
                _ => {}
            }
        }

        Err(JiraError::Api {
            status: 0,
            message: format!("Timed out waiting for Jira bulk move task {task_id}"),
        })
    }

    // ── Raw API passthrough ───────────────────────────────────────────────────

    /// Execute an arbitrary Jira REST API call and return the raw JSON response.
    /// Returns `None` for 204 No Content responses (success with no body).
    /// `path` should start with `/rest/...`
    pub async fn raw_request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Option<Value>> {
        let headers = self.auth_headers()?;
        let url = format!("{}{}", self.config.base_url.trim_end_matches('/'), path);

        let http = &self.http;
        let response = self
            .execute_with_retry(|| {
                let req = match method.to_uppercase().as_str() {
                    "GET" => http.get(&url),
                    "POST" => http.post(&url),
                    "PUT" => http.put(&url),
                    "DELETE" => http.delete(&url),
                    "PATCH" => http.patch(&url),
                    _ => http.get(&url),
                };
                let req = req.headers(headers.clone());
                if let Some(b) = &body {
                    req.json(b)
                } else {
                    req
                }
            })
            .await?;

        let status = response.status();

        // 204 No Content — success with empty body
        if status == StatusCode::NO_CONTENT {
            return Ok(None);
        }

        if status.is_success() {
            let value: Value = response.json().await?;
            return Ok(Some(value));
        }

        let body_text = response.text().await.unwrap_or_default();
        Err(match status {
            StatusCode::NOT_FOUND => JiraError::NotFound(body_text),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                JiraError::Auth(format!("HTTP {status}: {body_text}"))
            }
            _ => JiraError::Api {
                status: status.as_u16(),
                message: body_text,
            },
        })
    }

    // ── Plans API (Jira Premium) ──────────────────────────────────────────────

    /// Check if this Jira instance is Premium tier.
    pub async fn is_premium(&self) -> bool {
        match self.get_server_info().await {
            Ok(info) => {
                let license = info
                    .get("deploymentType")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                // "Cloud" with advanced features, or check licenseInfo
                let _ = license;
                // Simplest heuristic: try to call the plans endpoint
                let headers = match self.auth_headers() {
                    Ok(h) => h,
                    Err(_) => return false,
                };
                let url = self.platform_url("/plans/plan");
                let http = &self.http;
                matches!(
                    http.get(&url).headers(headers).send().await,
                    Ok(r) if r.status().is_success()
                )
            }
            Err(_) => false,
        }
    }

    /// List Jira Plans (requires Jira Premium / Advanced Roadmaps).
    pub async fn get_plans(&self) -> Result<Vec<Value>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/plans/plan");

        #[derive(serde::Deserialize)]
        struct PlansResponse {
            values: Vec<Value>,
        }

        let http = &self.http;
        let resp: PlansResponse = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(resp.values)
    }

    // ── Issue Links ──────────────────────────────────────────────────────────

    /// List available issue link types.
    pub async fn list_issue_link_types(&self) -> Result<Vec<IssueLinkType>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/issueLinkType");

        #[derive(serde::Deserialize)]
        struct LinkTypesResponse {
            #[serde(rename = "issueLinkTypes")]
            issue_link_types: Vec<IssueLinkType>,
        }

        let http = &self.http;
        let resp: LinkTypesResponse = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(resp.issue_link_types)
    }

    /// Create a link between two issues.
    /// `type_name` is the name of the link type (e.g., "Blocks").
    /// `outward_key` is the issue that is doing the action (e.g., the blocker).
    /// `inward_key` is the issue that is receiving the action (e.g., the blocked issue).
    pub async fn link_issues(
        &self,
        outward_key: &str,
        inward_key: &str,
        type_name: &str,
        comment: Option<&str>,
    ) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/issueLink");

        let mut payload = json!({
            "type": { "name": type_name },
            "inwardIssue": { "key": inward_key },
            "outwardIssue": { "key": outward_key }
        });

        if let Some(c) = comment {
            payload["comment"] = json!({
                "body": crate::adf::markdown_to_adf(c)
            });
        }

        let http = &self.http;
        self.request_no_body(|| http.post(&url).headers(headers.clone()).json(&payload))
            .await
    }

    /// Get a specific issue link by ID.
    pub async fn get_issue_link(&self, link_id: &str) -> Result<IssueLink> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issueLink/{link_id}"));

        let http = &self.http;
        self.request(|| http.get(&url).headers(headers.clone()))
            .await
    }

    /// Delete an issue link by ID.
    pub async fn delete_issue_link(&self, link_id: &str) -> Result<()> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issueLink/{link_id}"));

        let http = &self.http;
        self.request_no_body(|| http.delete(&url).headers(headers.clone()))
            .await
    }
}

async fn handle_response<T>(response: Response) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();

    if status.is_success() {
        // 204/205: no body — callers expecting a body should use request_no_body().
        // Defensive: try to deserialize from null (works for Value and Option<T>).
        if status == StatusCode::NO_CONTENT || status == StatusCode::RESET_CONTENT {
            return serde_json::from_value(serde_json::Value::Null).map_err(|_| JiraError::Api {
                status: status.as_u16(),
                message: "Unexpected empty response body".into(),
            });
        }
        let value: T = response.json().await?;
        return Ok(value);
    }

    let body = response.text().await.unwrap_or_default();

    match status {
        StatusCode::NOT_FOUND => Err(JiraError::NotFound(body)),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            Err(JiraError::Auth(format!("HTTP {status}: {body}")))
        }
        _ => Err(JiraError::Api {
            status: status.as_u16(),
            message: body,
        }),
    }
}

/// Returns current UTC time in Jira worklog format: "2006-01-02T15:04:05.000+0000"
fn current_jira_timestamp() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3f%z")
        .to_string()
}

fn base64_encode(input: &str) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() {
            bytes[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            bytes[i + 2] as u32
        } else {
            0
        };

        let _ = write!(result, "{}", CHARS[((b0 >> 2) & 0x3F) as usize] as char);
        let _ = write!(
            result,
            "{}",
            CHARS[(((b0 & 0x3) << 4) | ((b1 >> 4) & 0xF)) as usize] as char
        );
        if i + 1 < bytes.len() {
            let _ = write!(
                result,
                "{}",
                CHARS[(((b1 & 0xF) << 2) | ((b2 >> 6) & 0x3)) as usize] as char
            );
        } else {
            result.push('=');
        }
        if i + 2 < bytes.len() {
            let _ = write!(result, "{}", CHARS[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        i += 3;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{JiraAuthType, JiraDeployment},
        model::FieldValue,
    };
    use wiremock::{
        matchers::{body_json, header, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    fn cloud_client(server: &MockServer) -> JiraClient {
        JiraClient::new(JiraConfig {
            profile_name: Some("cloud-main".into()),
            base_url: server.uri(),
            email: "dev@example.com".into(),
            token: Some("cloud-token".into()),
            project: None,
            timeout_secs: 30,
            deployment: JiraDeployment::Cloud,
            auth_type: JiraAuthType::CloudApiToken,
            api_version: 3,
        })
    }

    fn cloud_auth() -> String {
        format!("Basic {}", base64_encode("dev@example.com:cloud-token"))
    }

    #[tokio::test]
    async fn data_center_pat_uses_bearer_and_api_v2() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/2/serverInfo"))
            .and(header("authorization", "Bearer dc-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "deploymentType": "Data Center",
                "version": "10.0.0"
            })))
            .mount(&server)
            .await;

        let client = JiraClient::new(JiraConfig {
            profile_name: Some("dc-main".into()),
            base_url: server.uri(),
            email: String::new(),
            token: Some("dc-token".into()),
            project: None,
            timeout_secs: 30,
            deployment: JiraDeployment::DataCenter,
            auth_type: JiraAuthType::DataCenterPat,
            api_version: 2,
        });

        let info = client.get_server_info().await.expect("server info");
        assert_eq!(info["deploymentType"], Value::String("Data Center".into()));
    }

    #[tokio::test]
    async fn cloud_auth_uses_basic_and_api_v3() {
        let server = MockServer::start().await;
        let expected = format!("Basic {}", base64_encode("dev@example.com:cloud-token"));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/serverInfo"))
            .and(header("authorization", expected.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "deploymentType": "Cloud",
                "version": "1001.0.0"
            })))
            .mount(&server)
            .await;

        let client = JiraClient::new(JiraConfig {
            profile_name: Some("cloud-main".into()),
            base_url: server.uri(),
            email: "dev@example.com".into(),
            token: Some("cloud-token".into()),
            project: None,
            timeout_secs: 30,
            deployment: JiraDeployment::Cloud,
            auth_type: JiraAuthType::CloudApiToken,
            api_version: 3,
        });

        let info = client.get_server_info().await.expect("server info");
        assert_eq!(info["deploymentType"], Value::String("Cloud".into()));
    }

    #[tokio::test]
    async fn get_fields_for_issue_type_supports_map_response() {
        let server = MockServer::start().await;
        let expected = format!("Basic {}", base64_encode("dev@example.com:cloud-token"));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/createmeta/TEST/issuetypes/10001"))
            .and(header("authorization", expected.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "fields": {
                    "summary": {
                        "name": "Summary",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = JiraClient::new(JiraConfig {
            profile_name: Some("cloud-main".into()),
            base_url: server.uri(),
            email: "dev@example.com".into(),
            token: Some("cloud-token".into()),
            project: None,
            timeout_secs: 30,
            deployment: JiraDeployment::Cloud,
            auth_type: JiraAuthType::CloudApiToken,
            api_version: 3,
        });

        let fields = client
            .get_fields_for_issue_type("TEST", "10001")
            .await
            .expect("map response should parse");

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].id, "summary");
        assert_eq!(fields[0].name, "Summary");
        assert!(fields[0].required);
        assert_eq!(fields[0].field_type, "string");
    }

    #[tokio::test]
    async fn get_fields_for_issue_type_supports_list_response() {
        let server = MockServer::start().await;
        let expected = format!("Basic {}", base64_encode("dev@example.com:cloud-token"));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/createmeta/TEST/issuetypes/10002"))
            .and(header("authorization", expected.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "fields": [
                    {
                        "fieldId": "customfield_10553",
                        "key": "customfield_10553",
                        "name": "Labels (OSS)",
                        "required": true,
                        "schema": {
                            "custom": "com.atlassian.jira.plugin.system.customfieldtypes:labels",
                            "items": "string",
                            "type": "array"
                        },
                        "allowedValues": []
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = JiraClient::new(JiraConfig {
            profile_name: Some("cloud-main".into()),
            base_url: server.uri(),
            email: "dev@example.com".into(),
            token: Some("cloud-token".into()),
            project: None,
            timeout_secs: 30,
            deployment: JiraDeployment::Cloud,
            auth_type: JiraAuthType::CloudApiToken,
            api_version: 3,
        });

        let fields = client
            .get_fields_for_issue_type("TEST", "10002")
            .await
            .expect("list response should parse");

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].id, "customfield_10553");
        assert_eq!(fields[0].name, "Labels (OSS)");
        assert!(fields[0].required);
        assert_eq!(fields[0].field_type, "array");
    }

    #[tokio::test]
    async fn search_users_supports_empty_query_and_no_results() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/api/3/user/search"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("query", ""))
            .and(query_param("maxResults", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let users = client
            .search_users("")
            .await
            .expect("empty query should work");

        assert!(users.is_empty());
    }

    #[tokio::test]
    async fn search_users_preserves_users_without_email() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/api/3/user/search"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("query", "alice"))
            .and(query_param("maxResults", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "accountId": "acct-1",
                    "displayName": "Alice Example"
                }
            ])))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let users = client
            .search_users("alice")
            .await
            .expect("search should parse");

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].account_id, "acct-1");
        assert_eq!(users[0].display_name.as_deref(), Some("Alice Example"));
        assert!(users[0].email_address.is_none());
    }

    #[tokio::test]
    async fn add_issue_to_sprint_posts_expected_payload() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("POST"))
            .and(path("/rest/agile/1.0/sprint/42/issue"))
            .and(header("authorization", expected_auth.as_str()))
            .and(body_json(json!({ "issues": ["TEST-123"] })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        client
            .add_issue_to_sprint(42, "TEST-123")
            .await
            .expect("add to sprint should succeed");
    }

    #[tokio::test]
    async fn add_issue_to_sprint_propagates_non_success_response() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("POST"))
            .and(path("/rest/agile/1.0/sprint/42/issue"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad sprint request"))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let err = client
            .add_issue_to_sprint(42, "TEST-123")
            .await
            .expect_err("non-success should return error");

        match err {
            JiraError::Api { status, message } => {
                assert_eq!(status, 400);
                assert!(message.contains("bad sprint request"));
            }
            other => panic!("expected JiraError::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_sprints_for_project_returns_empty_when_no_boards_exist() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("projectKeyOrId", "TEST"))
            .and(query_param("maxResults", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "values": [] })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let sprints = client
            .list_sprints_for_project("TEST")
            .await
            .expect("no boards should not error");

        assert!(sprints.is_empty());
    }

    #[tokio::test]
    async fn list_sprints_for_project_dedups_sorts_and_skips_missing_board_sprints() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("projectKeyOrId", "TEST"))
            .and(query_param("maxResults", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [
                    { "id": 1 },
                    { "id": 2 },
                    { "id": 3 }
                ]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/1/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("state", "active,future"))
            .and(query_param("maxResults", "200"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [
                    { "id": 20, "name": "Future Sprint", "state": "future" },
                    { "id": 10, "name": "Active Sprint", "state": "active" }
                ]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/2/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("state", "active,future"))
            .and(query_param("maxResults", "200"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [
                    { "id": 10, "name": "Active Sprint", "state": "active" },
                    { "id": 30, "name": "Alpha Future", "state": "future" }
                ]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/3/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("state", "active,future"))
            .and(query_param("maxResults", "200"))
            .respond_with(ResponseTemplate::new(404).set_body_string("board not found"))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let sprints = client
            .list_sprints_for_project("TEST")
            .await
            .expect("404 on one board should be skipped");

        assert_eq!(sprints.len(), 3);
        assert_eq!(sprints[0].id, 10);
        assert_eq!(sprints[0].state, "active");
        assert_eq!(sprints[1].id, 30);
        assert_eq!(sprints[1].state, "future");
        assert_eq!(sprints[2].id, 20);
        assert_eq!(sprints[2].state, "future");
    }

    #[tokio::test]
    async fn list_sprints_for_project_handles_large_sprint_pages() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        let sprint_values: Vec<Value> = (1..=60)
            .map(|id| {
                json!({
                    "id": id,
                    "name": format!("Sprint {id:02}"),
                    "state": if id == 1 { "active" } else { "future" }
                })
            })
            .collect();

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("projectKeyOrId", "TEST"))
            .and(query_param("maxResults", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{ "id": 9 }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/9/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("state", "active,future"))
            .and(query_param("maxResults", "200"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": sprint_values
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let sprints = client
            .list_sprints_for_project("TEST")
            .await
            .expect(">50 sprints in one response should parse");

        assert_eq!(sprints.len(), 60);
        assert_eq!(sprints[0].id, 1);
        assert_eq!(sprints[0].state, "active");
        assert_eq!(sprints[59].id, 60);
    }

    #[tokio::test]
    async fn list_sprints_for_project_paginates_boards_and_sprints() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("projectKeyOrId", "TEST"))
            .and(query_param("maxResults", "100"))
            .and(query_param("startAt", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{ "id": 1 }],
                "isLast": false
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("projectKeyOrId", "TEST"))
            .and(query_param("maxResults", "100"))
            .and(query_param("startAt", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{ "id": 2 }],
                "isLast": true
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/1/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("state", "active,future"))
            .and(query_param("maxResults", "200"))
            .and(query_param("startAt", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{
                    "id": 10,
                    "name": "Sprint 10",
                    "state": "active"
                }],
                "isLast": false
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/1/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("state", "active,future"))
            .and(query_param("maxResults", "200"))
            .and(query_param("startAt", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{
                    "id": 11,
                    "name": "Sprint 11",
                    "state": "future"
                }],
                "isLast": true
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/2/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("state", "active,future"))
            .and(query_param("maxResults", "200"))
            .and(query_param("startAt", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{
                    "id": 12,
                    "name": "Sprint 12",
                    "state": "future"
                }],
                "isLast": true
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let sprints = client
            .list_sprints_for_project("TEST")
            .await
            .expect("pagination should collect all pages");

        assert_eq!(sprints.len(), 3);
        assert_eq!(sprints[0].id, 10);
        assert_eq!(sprints[1].id, 11);
        assert_eq!(sprints[2].id, 12);
    }

    #[tokio::test]
    async fn list_sprints_for_project_propagates_non_not_found_board_errors() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("projectKeyOrId", "TEST"))
            .and(query_param("maxResults", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{ "id": 9 }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/9/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("state", "active,future"))
            .and(query_param("maxResults", "200"))
            .respond_with(ResponseTemplate::new(500).set_body_string("jira exploded"))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let err = client
            .list_sprints_for_project("TEST")
            .await
            .expect_err("500 should propagate");

        match err {
            JiraError::Api { status, message } => {
                assert_eq!(status, 500);
                assert!(message.contains("jira exploded"));
            }
            other => panic!("expected JiraError::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_sprint_posts_expected_payload() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("POST"))
            .and(path("/rest/agile/1.0/sprint"))
            .and(header("authorization", expected_auth.as_str()))
            .and(body_json(json!({
                "name": "Sprint 42",
                "originBoardId": 7,
                "startDate": "2026-05-20T00:00:00.000Z",
                "endDate": "2026-05-27T00:00:00.000Z",
                "goal": "Ship sprint lifecycle support"
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id": 42,
                "name": "Sprint 42",
                "state": "future",
                "goal": "Ship sprint lifecycle support",
                "startDate": "2026-05-20T00:00:00.000Z",
                "endDate": "2026-05-27T00:00:00.000Z"
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let sprint = client
            .create_sprint(
                7,
                "Sprint 42",
                Some("2026-05-20T00:00:00.000Z"),
                Some("2026-05-27T00:00:00.000Z"),
                Some("Ship sprint lifecycle support"),
            )
            .await
            .expect("create sprint should succeed");

        assert_eq!(sprint.id, 42);
        assert_eq!(sprint.board_id, Some(7));
        assert_eq!(sprint.state, "future");
        assert_eq!(
            sprint.goal.as_deref(),
            Some("Ship sprint lifecycle support")
        );
    }

    #[tokio::test]
    async fn update_sprint_puts_expected_payload() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("PUT"))
            .and(path("/rest/agile/1.0/sprint/42"))
            .and(header("authorization", expected_auth.as_str()))
            .and(body_json(json!({
                "state": "active",
                "startDate": "2026-05-20T00:00:00.000Z",
                "endDate": "2026-05-27T00:00:00.000Z"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 42,
                "name": "Sprint 42",
                "state": "active",
                "startDate": "2026-05-20T00:00:00.000Z",
                "endDate": "2026-05-27T00:00:00.000Z"
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let sprint = client
            .update_sprint(
                42,
                json!({
                    "state": "active",
                    "startDate": "2026-05-20T00:00:00.000Z",
                    "endDate": "2026-05-27T00:00:00.000Z"
                }),
            )
            .await
            .expect("update sprint should succeed");

        assert_eq!(sprint.id, 42);
        assert_eq!(sprint.state, "active");
    }

    #[tokio::test]
    async fn delete_sprint_uses_delete_endpoint() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("DELETE"))
            .and(path("/rest/agile/1.0/sprint/42"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        client
            .delete_sprint(42)
            .await
            .expect("delete sprint should succeed");
    }

    #[tokio::test]
    async fn create_issue_v2_prefers_adf_and_builds_extended_fields() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();
        let description_adf = json!({
            "type": "doc",
            "version": 1,
            "content": [{
                "type": "paragraph",
                "content": [{ "type": "text", "text": "ADF body" }]
            }]
        });

        Mock::given(method("GET"))
            .and(path("/rest/api/3/user/search"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("query", "alice@example.com"))
            .and(query_param("maxResults", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "accountId": "acct-1",
                    "emailAddress": "alice@example.com",
                    "displayName": "Alice"
                }
            ])))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/rest/api/3/issue"))
            .and(header("authorization", expected_auth.as_str()))
            .and(body_json(json!({
                "fields": {
                    "project": { "key": "TEST" },
                    "summary": "Ship feature",
                    "issuetype": { "name": "Task" },
                    "description": description_adf,
                    "assignee": { "accountId": "acct-1" },
                    "priority": { "name": "High" },
                    "labels": ["backend", "urgent"],
                    "components": [{ "name": "api" }, { "name": "worker" }],
                    "parent": { "key": "TEST-1" },
                    "fixVersions": [{ "name": "v1.0" }, { "name": "v1.1" }],
                    "customfield_10010": "hello",
                    "customfield_10011": { "value": "Blue" },
                    "customfield_10012": ["triage"]
                }
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "key": "TEST-123" })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-123"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "10001",
                "key": "TEST-123",
                "fields": {
                    "summary": "Ship feature",
                    "description": description_adf,
                    "status": { "name": "To Do" },
                    "issuetype": { "name": "Task" },
                    "project": { "key": "TEST" },
                    "created": "2023-01-01T00:00:00.000+0000",
                    "updated": "2023-01-01T00:00:00.000+0000"
                }
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let mut custom_fields = std::collections::HashMap::new();
        custom_fields.insert(
            "customfield_10010".to_string(),
            FieldValue::Text("hello".into()),
        );
        custom_fields.insert(
            "customfield_10011".to_string(),
            FieldValue::SelectName("Blue".into()),
        );
        custom_fields.insert(
            "customfield_10012".to_string(),
            FieldValue::Labels(vec!["triage".into()]),
        );

        let issue = client
            .create_issue_v2(CreateIssueRequestV2 {
                project_key: "TEST".into(),
                summary: "Ship feature".into(),
                description: Some("markdown body should be ignored".into()),
                description_adf: Some(description_adf.clone()),
                issue_type: "Task".into(),
                assignee: Some("alice@example.com".into()),
                priority: Some("High".into()),
                labels: vec!["backend".into(), "urgent".into()],
                components: vec!["api".into(), "worker".into()],
                parent: Some("TEST-1".into()),
                fix_versions: vec!["v1.0".into(), "v1.1".into()],
                custom_fields,
            })
            .await
            .expect("create issue v2 should succeed");

        assert_eq!(issue.key, "TEST-123");
        assert_eq!(issue.summary, "Ship feature");
        assert_eq!(issue.description, Some(description_adf));
    }

    #[tokio::test]
    async fn create_issue_v2_uses_markdown_description_when_adf_missing() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();
        let markdown_adf = markdown_to_adf("hello world");

        Mock::given(method("POST"))
            .and(path("/rest/api/3/issue"))
            .and(header("authorization", expected_auth.as_str()))
            .and(body_json(json!({
                "fields": {
                    "project": { "key": "TEST" },
                    "summary": "Plain issue",
                    "issuetype": { "name": "Task" },
                    "description": markdown_adf
                }
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "key": "TEST-124" })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-124"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "10002",
                "key": "TEST-124",
                "fields": {
                    "summary": "Plain issue",
                    "description": markdown_to_adf("hello world"),
                    "status": { "name": "To Do" },
                    "issuetype": { "name": "Task" },
                    "project": { "key": "TEST" },
                    "created": "2023-01-01T00:00:00.000+0000",
                    "updated": "2023-01-01T00:00:00.000+0000"
                }
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let issue = client
            .create_issue_v2(CreateIssueRequestV2 {
                project_key: "TEST".into(),
                summary: "Plain issue".into(),
                description: Some("hello world".into()),
                description_adf: None,
                issue_type: "Task".into(),
                assignee: None,
                priority: None,
                labels: vec![],
                components: vec![],
                parent: None,
                fix_versions: vec![],
                custom_fields: std::collections::HashMap::new(),
            })
            .await
            .expect("markdown fallback should succeed");

        assert_eq!(issue.key, "TEST-124");
        assert_eq!(issue.summary, "Plain issue");
        assert_eq!(issue.description, Some(markdown_to_adf("hello world")));
    }

    #[tokio::test]
    async fn get_comments_parses_comment_text_and_mentions() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-1/comment"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "comments": [
                    {
                        "id": "10001",
                        "author": {
                            "displayName": "Alice",
                            "accountId": "acct-1"
                        },
                        "body": {
                            "type": "doc",
                            "version": 1,
                            "content": [{
                                "type": "paragraph",
                                "content": [
                                    { "type": "text", "text": "Hi " },
                                    { "type": "mention", "attrs": { "id": "acct-2", "text": "@Bob" } }
                                ]
                            }]
                        },
                        "created": "2023-01-01T00:00:00.000+0000",
                        "updated": "2023-01-01T00:00:00.000+0000"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let comments = client
            .get_comments("TEST-1")
            .await
            .expect("comments should parse");

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].id, "10001");
        assert_eq!(comments[0].author.as_deref(), Some("Alice"));
        assert_eq!(comments[0].author_account_id.as_deref(), Some("acct-1"));
        assert_eq!(comments[0].body.as_deref(), Some("Hi @Bob"));
        assert_eq!(comments[0].mentions, vec!["acct-2"]);
    }

    #[tokio::test]
    async fn add_comment_adf_posts_prebuilt_adf_payload() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();
        let adf = json!({
            "type": "doc",
            "version": 1,
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "Hi " },
                    { "type": "mention", "attrs": { "id": "acct-2", "text": "@Bob" } }
                ]
            }]
        });

        Mock::given(method("POST"))
            .and(path("/rest/api/3/issue/TEST-1/comment"))
            .and(header("authorization", expected_auth.as_str()))
            .and(body_json(json!({ "body": adf.clone() })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id": "10002",
                "author": {
                    "displayName": "Alice",
                    "accountId": "acct-1"
                },
                "body": adf,
                "created": "2023-01-01T00:00:00.000+0000",
                "updated": "2023-01-01T00:00:00.000+0000"
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let comment = client
            .add_comment_adf(
                "TEST-1",
                json!({
                    "type": "doc",
                    "version": 1,
                    "content": [{
                        "type": "paragraph",
                        "content": [
                            { "type": "text", "text": "Hi " },
                            { "type": "mention", "attrs": { "id": "acct-2", "text": "@Bob" } }
                        ]
                    }]
                }),
            )
            .await
            .expect("add comment adf should succeed");

        assert_eq!(comment.id, "10002");
        assert_eq!(comment.body.as_deref(), Some("Hi @Bob"));
        assert_eq!(comment.mentions, vec!["acct-2"]);
    }

    #[tokio::test]
    async fn search_users_retries_after_429_then_succeeds() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/api/3/user/search"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("query", "alice"))
            .and(query_param("maxResults", "20"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/user/search"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("query", "alice"))
            .and(query_param("maxResults", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "accountId": "acct-1",
                    "displayName": "Alice Example"
                }
            ])))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let users = client
            .search_users("alice")
            .await
            .expect("request should retry and succeed");

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].account_id, "acct-1");
    }

    #[tokio::test]
    async fn search_users_returns_rate_limit_after_max_retries() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("GET"))
            .and(path("/rest/api/3/user/search"))
            .and(header("authorization", expected_auth.as_str()))
            .and(query_param("query", "alice"))
            .and(query_param("maxResults", "20"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let err = client
            .search_users("alice")
            .await
            .expect_err("request should fail after max retries");

        match err {
            JiraError::RateLimit { retry_after } => assert_eq!(retry_after, 0),
            other => panic!("expected JiraError::RateLimit, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn move_issue_submits_bulk_move_and_fetches_issue_on_complete() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("POST"))
            .and(path("/rest/api/3/bulk/issues/move"))
            .and(header("authorization", expected_auth.as_str()))
            .and(body_json(json!({
                "sendBulkNotification": true,
                "targetToSourcesMapping": {
                    "NEW,10002": {
                        "inferClassificationDefaults": true,
                        "inferFieldDefaults": true,
                        "inferStatusDefaults": true,
                        "inferSubtaskTypeDefault": true,
                        "issueIdsOrKeys": ["TEST-1"]
                    }
                }
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "taskId": "task-1" })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/bulk/queue/task-1"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "COMPLETE" })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-1"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "10001",
                "key": "TEST-1",
                "fields": {
                    "summary": "Moved issue",
                    "status": { "name": "To Do" },
                    "issuetype": { "name": "Task" },
                    "project": { "key": "NEW" },
                    "created": "2023-01-01T00:00:00.000+0000",
                    "updated": "2023-01-01T00:00:00.000+0000"
                }
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let issue = client
            .move_issue("TEST-1", "NEW", "10002", None)
            .await
            .expect("move issue should complete");

        assert_eq!(issue.key, "TEST-1");
        assert_eq!(issue.project_key, "NEW");
    }

    #[tokio::test]
    async fn move_issue_returns_api_error_when_bulk_task_fails() {
        let server = MockServer::start().await;
        let expected_auth = cloud_auth();

        Mock::given(method("POST"))
            .and(path("/rest/api/3/bulk/issues/move"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "taskId": "task-2" })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/bulk/queue/task-2"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "FAILED",
                "errors": ["cannot move issue"]
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let err = client
            .move_issue("TEST-1", "NEW", "10002", None)
            .await
            .expect_err("failed bulk task should return error");

        match err {
            JiraError::Api { message, .. } => assert!(message.contains("FAILED")),
            other => panic!("expected JiraError::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn issue_link_integration() {
        let server = MockServer::start().await;
        let expected_auth = format!("Basic {}", base64_encode("dev@example.com:cloud-token"));

        // 1. Mock list_issue_link_types
        Mock::given(method("GET"))
            .and(path("/rest/api/3/issueLinkType"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issueLinkTypes": [
                    {
                        "id": "10000",
                        "name": "Blocks",
                        "inward": "is blocked by",
                        "outward": "blocks"
                    }
                ]
            })))
            .mount(&server)
            .await;

        // 2. Mock link_issues
        Mock::given(method("POST"))
            .and(path("/rest/api/3/issueLink"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let client = JiraClient::new(JiraConfig {
            profile_name: Some("cloud-main".into()),
            base_url: server.uri(),
            email: "dev@example.com".into(),
            token: Some("cloud-token".into()),
            project: None,
            timeout_secs: 30,
            deployment: JiraDeployment::Cloud,
            auth_type: JiraAuthType::CloudApiToken,
            api_version: 3,
        });

        // Test list
        let link_types = client.list_issue_link_types().await.expect("list types");
        assert_eq!(link_types.len(), 1);
        assert_eq!(link_types[0].name, "Blocks");

        // Test link
        client
            .link_issues("TEST-1", "TEST-2", "Blocks", Some("Adding dependency"))
            .await
            .expect("link issues");
    }

    #[tokio::test]
    async fn get_issue_parses_links() {
        let server = MockServer::start().await;
        let expected_auth = format!("Basic {}", base64_encode("dev@example.com:cloud-token"));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-1"))
            .and(header("authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "10001",
                "key": "TEST-1",
                "fields": {
                    "summary": "Main Issue",
                    "status": { "name": "To Do" },
                    "issuetype": { "name": "Task" },
                    "project": { "key": "TEST" },
                    "created": "2023-01-01T00:00:00.000+0000",
                    "updated": "2023-01-01T00:00:00.000+0000",
                    "issuelinks": [
                        {
                            "id": "20000",
                            "type": {
                                "id": "10000",
                                "name": "Blocks",
                                "inward": "is blocked by",
                                "outward": "blocks"
                            },
                            "outwardIssue": {
                                "id": "10002",
                                "key": "TEST-2",
                                "fields": {
                                    "summary": "Blocked Issue",
                                    "status": { "name": "Open" },
                                    "priority": { "name": "High" },
                                    "issuetype": { "name": "Bug" }
                                }
                            }
                        }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client = JiraClient::new(JiraConfig {
            profile_name: Some("cloud-main".into()),
            base_url: server.uri(),
            email: "dev@example.com".into(),
            token: Some("cloud-token".into()),
            project: None,
            timeout_secs: 30,
            deployment: JiraDeployment::Cloud,
            auth_type: JiraAuthType::CloudApiToken,
            api_version: 3,
        });

        let issue = client.get_issue("TEST-1").await.expect("get issue");
        assert_eq!(issue.links.len(), 1);
        let link = &issue.links[0];
        assert_eq!(link.link_type.name, "Blocks");
        assert!(link.outward_issue.is_some());
        assert_eq!(link.outward_issue.as_ref().unwrap().key, "TEST-2");
        assert_eq!(
            link.outward_issue.as_ref().unwrap().summary,
            "Blocked Issue"
        );
    }

    #[tokio::test]
    async fn get_remote_links_parses_object_title_and_url() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-1/remotelink"))
            .and(header("authorization", cloud_auth().as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": 10001,
                    "self": "https://jira.example.com/rest/api/3/issue/TEST-1/remotelink/10001",
                    "globalId": "system=https://docs.example.com&id=42",
                    "relationship": "Wiki Page",
                    "object": {
                        "title": "Design Doc",
                        "url": "https://docs.example.com/design",
                        "summary": "Architecture overview"
                    }
                },
                {
                    "id": 10002,
                    "object": {
                        "title": "Tracking ticket",
                        "url": "https://tracker.example.com/4242"
                    }
                }
            ])))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let links = client
            .get_remote_links("TEST-1")
            .await
            .expect("remote links should parse");

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].id, 10001);
        assert_eq!(
            links[0].global_id.as_deref(),
            Some("system=https://docs.example.com&id=42")
        );
        assert_eq!(links[0].object.title, "Design Doc");
        assert_eq!(links[0].object.url, "https://docs.example.com/design");
        assert_eq!(
            links[0].object.summary.as_deref(),
            Some("Architecture overview")
        );
        assert_eq!(links[1].id, 10002);
        assert!(links[1].global_id.is_none());
        assert!(links[1].object.summary.is_none());
    }

    #[tokio::test]
    async fn get_remote_links_returns_empty_when_no_links() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-1/remotelink"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let links = client.get_remote_links("TEST-1").await.expect("parse");
        assert!(links.is_empty());
    }

    #[tokio::test]
    async fn get_project_components_parses_id_and_name() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/project/PROJ/components"))
            .and(header("authorization", cloud_auth().as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": "10100",
                    "name": "Backend",
                    "description": "Server-side code",
                    "self": "https://jira.example.com/rest/api/3/component/10100"
                },
                {
                    "id": "10101",
                    "name": "Frontend"
                }
            ])))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let components = client
            .get_project_components("PROJ")
            .await
            .expect("components should parse");

        assert_eq!(components.len(), 2);
        assert_eq!(components[0].id, "10100");
        assert_eq!(components[0].name, "Backend");
        assert_eq!(
            components[0].description.as_deref(),
            Some("Server-side code")
        );
        assert!(components[0].self_url.is_some());
        assert_eq!(components[1].name, "Frontend");
        assert!(components[1].description.is_none());
    }

    #[tokio::test]
    async fn get_transitions_parses_id_name_and_preserves_extra_fields() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-1/transitions"))
            .and(header("authorization", cloud_auth().as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "expand": "transitions",
                "transitions": [
                    {
                        "id": "21",
                        "name": "In Progress",
                        "hasScreen": false,
                        "isGlobal": false,
                        "isInitial": false,
                        "isAvailable": true,
                        "to": {
                            "id": "3",
                            "name": "In Progress",
                            "statusCategory": { "key": "indeterminate" }
                        }
                    },
                    {
                        "id": "31",
                        "name": "Done",
                        "to": { "id": "10001", "name": "Done" }
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let transitions = client
            .get_transitions("TEST-1")
            .await
            .expect("transitions should parse");

        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].id, "21");
        assert_eq!(transitions[0].name, "In Progress");
        assert_eq!(
            transitions[0].to.as_ref().and_then(|t| t.name.as_deref()),
            Some("In Progress")
        );

        // Extra Jira fields (hasScreen, isGlobal, ...) must round-trip via #[serde(flatten)]
        // so MCP consumers re-serializing the struct keep the full payload shape.
        let reserialized = serde_json::to_value(&transitions[0]).expect("serialize");
        assert_eq!(reserialized["hasScreen"], json!(false));
        assert_eq!(reserialized["isAvailable"], json!(true));

        assert_eq!(transitions[1].id, "31");
        assert!(transitions[1].to.is_some());
    }

    #[tokio::test]
    async fn get_transitions_returns_empty_when_no_transitions_available() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/TEST-1/transitions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transitions": []
            })))
            .mount(&server)
            .await;

        let client = cloud_client(&server);
        let transitions = client.get_transitions("TEST-1").await.expect("parse");
        assert!(transitions.is_empty());
    }
}

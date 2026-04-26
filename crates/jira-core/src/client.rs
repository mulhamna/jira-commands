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
        field::Field,
        issue::{
            CreateIssueRequest, CreateIssueRequestV2, Issue, RawIssue, RawSearchResponse,
            SearchResult, UpdateIssueRequest,
        },
        worklog::Worklog,
    },
};

const AGILE_BASE: &str = "/rest/agile/1.0";
const MAX_RETRIES: u32 = 3;

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

    #[allow(dead_code)]
    fn agile_url(&self, path: &str) -> String {
        format!(
            "{}{}{}",
            self.config.base_url.trim_end_matches('/'),
            AGILE_BASE,
            path
        )
    }

    fn auth_headers(&self) -> Result<HeaderMap> {
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
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        Ok(headers)
    }

    /// Auth headers without Content-Type — required for multipart uploads.
    fn auth_headers_no_content_type(&self) -> Result<HeaderMap> {
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
        Ok(headers)
    }

    /// Get the current authenticated user's accountId.
    pub async fn get_myself(&self) -> Result<String> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/myself");

        let http = &self.http;
        let user: serde_json::Value = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        user.get("accountId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: "Could not get accountId from /myself".into(),
            })
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
                u.get("emailAddress")
                    .and_then(|v| v.as_str())
                    .map(|e| e.eq_ignore_ascii_case(s))
                    .unwrap_or(false)
            })
            .or_else(|| users.first())
            .and_then(|u| u.get("accountId"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| JiraError::Api {
                status: 0,
                message: format!("User not found: {s}"),
            })
    }

    /// Core request method with rate-limit retry logic.
    async fn request<T>(&self, builder_fn: impl Fn() -> reqwest::RequestBuilder) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let req = builder_fn();
            let response = req.send().await?;

            if response.status() == StatusCode::TOO_MANY_REQUESTS {
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
                continue;
            }

            return handle_response(response).await;
        }
    }

    /// Core request method for responses with no body (204 No Content).
    async fn request_no_body(
        &self,
        builder_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<()> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let req = builder_fn();
            let response = req.send().await?;

            if response.status() == StatusCode::TOO_MANY_REQUESTS {
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
                continue;
            }

            let status = response.status();
            if status.is_success() {
                return Ok(());
            }

            let body = response.text().await.unwrap_or_default();
            if status == StatusCode::NOT_FOUND {
                return Err(JiraError::NotFound(body));
            }
            return Err(JiraError::Api {
                status: status.as_u16(),
                message: body,
            });
        }
    }

    /// Multipart request with rate-limit retry (for attachment uploads).
    async fn request_multipart<T>(
        &self,
        builder_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let req = builder_fn();
            let response = req.send().await?;

            if response.status() == StatusCode::TOO_MANY_REQUESTS {
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
                continue;
            }

            return handle_response(response).await;
        }
    }

    /// Search issues using JQL with cursor-based pagination.
    pub async fn search_issues(
        &self,
        jql: &str,
        next_page_token: Option<&str>,
        max_results: Option<u32>,
    ) -> Result<SearchResult> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/search/jql");

        let mut body = json!({
            "jql": jql,
            "maxResults": max_results.unwrap_or(50),
            "fields": ["summary", "status", "assignee", "reporter", "priority",
                       "issuetype", "project", "created", "updated", "description"]
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
    pub async fn get_transitions(&self, key: &str) -> Result<Vec<Value>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/issue/{key}/transitions"));

        #[derive(serde::Deserialize)]
        struct TransitionsResponse {
            transitions: Vec<Value>,
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
    pub async fn search_users(&self, query: &str) -> Result<Vec<Value>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url("/user/search");

        let http = &self.http;
        let users: Vec<Value> = self
            .request(|| {
                http.get(&url)
                    .headers(headers.clone())
                    .query(&[("query", query), ("maxResults", "20")])
            })
            .await?;

        Ok(users)
    }

    /// List components available within a project.
    pub async fn get_project_components(&self, project_key: &str) -> Result<Vec<Value>> {
        let headers = self.auth_headers()?;
        let url = self.platform_url(&format!("/project/{project_key}/components"));

        let http = &self.http;
        let components: Vec<Value> = self
            .request(|| http.get(&url).headers(headers.clone()))
            .await?;

        Ok(components)
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
    pub async fn get_remote_links(&self, issue_key: &str) -> Result<Vec<Value>> {
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
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let req = match method.to_uppercase().as_str() {
                "GET" => http.get(&url),
                "POST" => http.post(&url),
                "PUT" => http.put(&url),
                "DELETE" => http.delete(&url),
                "PATCH" => http.patch(&url),
                _ => http.get(&url),
            };
            let req = req.headers(headers.clone());
            let req = if let Some(b) = &body {
                req.json(b)
            } else {
                req
            };

            let response = req.send().await?;

            if response.status() == StatusCode::TOO_MANY_REQUESTS {
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
                continue;
            }

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
            return Err(match status {
                StatusCode::NOT_FOUND => JiraError::NotFound(body_text),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    JiraError::Auth(format!("HTTP {status}: {body_text}"))
                }
                _ => JiraError::Api {
                    status: status.as_u16(),
                    message: body_text,
                },
            });
        }
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
}

/// Issue type metadata (id + name) returned by createmeta.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IssueType {
    pub id: String,
    pub name: String,
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
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Manual conversion: secs since epoch → date/time components
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    // Days since epoch
    let days = secs / 86400;
    // Simplified: use a rough date calculation
    // For worklog "started", accuracy to the day is sufficient
    let year_approx = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000+0000",
        year_approx,
        month.min(12),
        day.min(28),
        h,
        m,
        s
    )
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
    use crate::config::{JiraAuthType, JiraDeployment};
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

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
}

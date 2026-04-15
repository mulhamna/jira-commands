use std::time::Duration;

use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Client, Response, StatusCode,
};
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::{
    adf::markdown_to_adf,
    config::JiraConfig,
    error::{JiraError, Result},
    model::{
        field::Field,
        issue::{CreateIssueRequest, Issue, RawIssue, RawSearchResponse, SearchResult, UpdateIssueRequest},
    },
};

const PLATFORM_BASE: &str = "/rest/api/3";
const AGILE_BASE: &str = "/rest/agile/1.0";
const MAX_RETRIES: u32 = 3;

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

    fn platform_url(&self, path: &str) -> String {
        format!("{}{}{}", self.config.base_url.trim_end_matches('/'), PLATFORM_BASE, path)
    }

    #[allow(dead_code)]
    fn agile_url(&self, path: &str) -> String {
        format!("{}{}{}", self.config.base_url.trim_end_matches('/'), AGILE_BASE, path)
    }

    fn auth_headers(&self) -> Result<HeaderMap> {
        let token = self
            .config
            .token
            .as_deref()
            .ok_or_else(|| JiraError::Auth("No token configured. Run `jira auth login` first.".into()))?;

        let credentials = base64_encode(&format!("{}:{}", self.config.email, token));
        let auth_value = format!("Basic {credentials}");

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .map_err(|e| JiraError::Auth(format!("Invalid auth header: {e}")))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        Ok(headers)
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
    async fn request_no_body(&self, builder_fn: impl Fn() -> reqwest::RequestBuilder) -> Result<()> {
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
            .request(|| {
                http.post(&url)
                    .headers(headers.clone())
                    .json(&body)
            })
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

        let description_adf = req
            .description
            .as_deref()
            .map(markdown_to_adf);

        let mut fields = json!({
            "project": { "key": req.project_key },
            "summary": req.summary,
            "issuetype": { "name": req.issue_type }
        });

        if let Some(adf) = description_adf {
            fields["description"] = adf;
        }

        if let Some(assignee) = &req.assignee {
            fields["assignee"] = json!({ "emailAddress": assignee });
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

        if let Some(description) = &req.description {
            fields["description"] = markdown_to_adf(description);
        }

        if let Some(assignee) = &req.assignee {
            fields["assignee"] = json!({ "emailAddress": assignee });
        }

        if let Some(priority) = &req.priority {
            fields["priority"] = json!({ "name": priority });
        }

        let body = json!({ "fields": fields });

        let http = &self.http;
        self.request_no_body(|| {
            http.put(&url).headers(headers.clone()).json(&body)
        })
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
        self.request_no_body(|| {
            http.post(&url).headers(headers.clone()).json(&body)
        })
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
}

async fn handle_response<T>(response: Response) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();

    if status.is_success() {
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

fn base64_encode(input: &str) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] as u32 } else { 0 };

        let _ = write!(result, "{}", CHARS[((b0 >> 2) & 0x3F) as usize] as char);
        let _ = write!(result, "{}", CHARS[(((b0 & 0x3) << 4) | ((b1 >> 4) & 0xF)) as usize] as char);
        if i + 1 < bytes.len() {
            let _ = write!(result, "{}", CHARS[(((b1 & 0xF) << 2) | ((b2 >> 6) & 0x3)) as usize] as char);
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

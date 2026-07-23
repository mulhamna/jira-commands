use rmcp::{model::ErrorCode, ErrorData};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone)]
pub struct AppError {
    rpc_code: ErrorCode,
    stable_code: &'static str,
    detail: String,
    data: Option<Value>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn validation(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::INVALID_PARAMS, "validation_error", detail, None)
    }

    pub fn unsafe_operation(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::INVALID_PARAMS, "unsafe_operation", detail, None)
    }

    pub fn auth_missing(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::INVALID_REQUEST, "auth_missing", detail, None)
    }

    pub fn config_error(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::INTERNAL_ERROR, "config_error", detail, None)
    }

    pub fn jira_api_error(detail: impl Into<String>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::INTERNAL_ERROR, "jira_api_error", detail, data)
    }

    /// Like `jira_api_error`, but with an explicit RPC code so a caller-side
    /// rejection (4xx) is reported as `INVALID_PARAMS` rather than masquerading
    /// as an internal server error.
    fn jira_api_error_with_code(
        rpc_code: ErrorCode,
        detail: impl Into<String>,
        data: Option<Value>,
    ) -> Self {
        Self::new(rpc_code, "jira_api_error", detail, data)
    }

    /// Map an HTTP status to the JSON-RPC error code: 5xx is genuinely
    /// server-side (`INTERNAL_ERROR`), everything else is the caller's input
    /// to fix (`INVALID_PARAMS`).
    fn rpc_code_for_status(status: u16) -> ErrorCode {
        if status >= 500 {
            ErrorCode::INTERNAL_ERROR
        } else {
            ErrorCode::INVALID_PARAMS
        }
    }

    /// Build a `jira_api_error`, lifting Jira's structured `errors` map and
    /// `errorMessages` array out of the raw response body into the error `data`
    /// when present. Falls back to the raw body as `detail` for non-JSON or
    /// non-standard responses (best-effort — never panics on a weird body).
    fn from_jira_api(status: u16, body: String) -> Self {
        let rpc_code = Self::rpc_code_for_status(status);
        if let Ok(Value::Object(parsed)) = serde_json::from_str::<Value>(&body) {
            let errors = parsed.get("errors").cloned();
            let error_messages = parsed.get("errorMessages").cloned();
            let has_errors = errors
                .as_ref()
                .and_then(Value::as_object)
                .is_some_and(|m| !m.is_empty());
            let has_messages = error_messages
                .as_ref()
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
            if has_errors || has_messages {
                let mut data = Map::new();
                data.insert("status".into(), json!(status));
                if let Some(e) = errors {
                    data.insert("errors".into(), e);
                }
                if let Some(m) = error_messages {
                    data.insert("errorMessages".into(), m);
                }
                let field_count = data
                    .get("errors")
                    .and_then(Value::as_object)
                    .map_or(0, Map::len);
                let detail = if field_count > 0 {
                    format!("Jira rejected the request: {field_count} field error(s)")
                } else {
                    "Jira rejected the request".to_string()
                };
                return Self::jira_api_error_with_code(rpc_code, detail, Some(Value::Object(data)));
            }
        }
        Self::jira_api_error_with_code(rpc_code, body, Some(json!({ "status": status })))
    }

    /// True when this error carries a non-empty Jira field-validation `errors` map.
    pub fn is_field_validation(&self) -> bool {
        self.stable_code == "jira_api_error"
            && self
                .data
                .as_ref()
                .and_then(|d| d.get("errors"))
                .and_then(Value::as_object)
                .is_some_and(|m| !m.is_empty())
    }

    /// Attach a key/value into this error's structured `data` payload.
    pub fn with_data_field(mut self, key: &str, value: Value) -> Self {
        let mut map = match self.data.take() {
            Some(Value::Object(m)) => m,
            Some(other) => {
                let mut m = Map::new();
                m.insert("context".into(), other);
                m
            }
            None => Map::new(),
        };
        map.insert(key.into(), value);
        self.data = Some(Value::Object(map));
        self
    }

    pub fn not_found(detail: impl Into<String>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::RESOURCE_NOT_FOUND, "not_found", detail, data)
    }

    pub fn rate_limited(retry_after: u64) -> Self {
        Self::new(
            ErrorCode::INTERNAL_ERROR,
            "rate_limited",
            format!("Jira rate limit encountered, retry after {retry_after}s"),
            Some(json!({ "retry_after": retry_after })),
        )
    }

    pub fn io_error(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::INTERNAL_ERROR, "io_error", detail, None)
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::INTERNAL_ERROR, "internal_error", detail, None)
    }

    pub fn to_mcp(self) -> ErrorData {
        let mut payload = match self.data {
            Some(Value::Object(map)) => map,
            Some(value) => {
                let mut map = Map::new();
                map.insert("context".into(), value);
                map
            }
            None => Map::new(),
        };
        payload.insert("details".into(), Value::String(self.detail));
        ErrorData::new(
            self.rpc_code,
            self.stable_code,
            Some(Value::Object(payload)),
        )
    }

    fn new(
        rpc_code: ErrorCode,
        stable_code: &'static str,
        detail: impl Into<String>,
        data: Option<Value>,
    ) -> Self {
        Self {
            rpc_code,
            stable_code,
            detail: detail.into(),
            data,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.stable_code, self.detail)
    }
}

impl std::error::Error for AppError {}

impl From<jira_core::JiraError> for AppError {
    fn from(value: jira_core::JiraError) -> Self {
        match value {
            jira_core::JiraError::Auth(message) => Self::auth_missing(message),
            jira_core::JiraError::Api { status, message } => Self::from_jira_api(status, message),
            jira_core::JiraError::Config(message) => Self::config_error(message),
            jira_core::JiraError::NotFound(message) => Self::not_found(message, None),
            jira_core::JiraError::RateLimit { retry_after } => Self::rate_limited(retry_after),
            jira_core::JiraError::Io(err) => Self::io_error(err.to_string()),
            jira_core::JiraError::Serialization(err) => Self::internal(err.to_string()),
            jira_core::JiraError::Http(err) => Self::jira_api_error(err.to_string(), None),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::internal(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::io_error(value.to_string())
    }
}

impl From<base64::DecodeError> for AppError {
    fn from(value: base64::DecodeError) -> Self {
        Self::validation(format!("Invalid base64 attachment payload: {value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jira_field_validation_body() {
        let body = r#"{"errorMessages":[],"errors":{"customfield_10011":"Epic Link is required"}}"#;
        let err = AppError::from(jira_core::JiraError::Api {
            status: 400,
            message: body.to_string(),
        });

        assert!(err.is_field_validation());
        let data = err.data.as_ref().expect("data present");
        assert_eq!(
            data["errors"]["customfield_10011"],
            json!("Epic Link is required")
        );
        assert_eq!(data["status"], json!(400));
        assert!(data.get("errorMessages").is_some());
    }

    #[test]
    fn non_json_body_passes_through() {
        let err = AppError::from(jira_core::JiraError::Api {
            status: 500,
            message: "<html>boom</html>".to_string(),
        });

        assert!(!err.is_field_validation());
        assert_eq!(err.detail, "<html>boom</html>");
    }

    #[test]
    fn client_errors_map_to_invalid_params() {
        let body = r#"{"errorMessages":[],"errors":{"summary":"required"}}"#;
        let err = AppError::from(jira_core::JiraError::Api {
            status: 400,
            message: body.to_string(),
        });
        assert_eq!(err.rpc_code, ErrorCode::INVALID_PARAMS);

        // Raw (non-standard) 4xx body still uses INVALID_PARAMS.
        let raw = AppError::from(jira_core::JiraError::Api {
            status: 409,
            message: "conflict".to_string(),
        });
        assert_eq!(raw.rpc_code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn server_errors_stay_internal() {
        let err = AppError::from(jira_core::JiraError::Api {
            status: 503,
            message: "<html>unavailable</html>".to_string(),
        });
        assert_eq!(err.rpc_code, ErrorCode::INTERNAL_ERROR);
    }
}

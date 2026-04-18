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

impl From<jira_core::JiraError> for AppError {
    fn from(value: jira_core::JiraError) -> Self {
        match value {
            jira_core::JiraError::Auth(message) => Self::auth_missing(message),
            jira_core::JiraError::Api { status, message } => {
                Self::jira_api_error(message, Some(json!({ "status": status })))
            }
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

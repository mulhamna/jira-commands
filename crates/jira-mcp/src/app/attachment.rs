use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::{
    error::{AppError, AppResult},
    models::{AttachmentDeleteArgs, AttachmentDownloadArgs, AttachmentListArgs},
};

use super::{
    shared::{require_confirm, to_value},
    JiraApp,
};

const FORBIDDEN_PREFIXES: &[&str] = &["/etc", "/System", "/usr", "/bin", "/sbin", "/var"];

impl JiraApp {
    pub async fn attachment_list(&self, args: AttachmentListArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let items = client.list_attachments(&args.issue_key).await?;
        to_value(items)
    }

    pub async fn attachment_download(&self, args: AttachmentDownloadArgs) -> AppResult<Value> {
        let path = validate_save_path(&args.save_path, args.force_path.unwrap_or(false))?;
        if path.exists() && !args.overwrite.unwrap_or(false) {
            return Err(AppError::validation(format!(
                "{} already exists. Pass overwrite=true to replace it.",
                path.display()
            )));
        }
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AppError::validation(format!(
                        "Failed to create directory {}: {e}",
                        parent.display()
                    ))
                })?;
            }
        }

        let client = self.build_client()?;
        let (_filename, bytes, mime) = client.download_attachment(&args.attachment_id).await?;
        std::fs::write(&path, &bytes).map_err(|e| {
            AppError::validation(format!("Failed to write {}: {e}", path.display()))
        })?;

        Ok(json!({
            "ok": true,
            "path": path.to_string_lossy(),
            "size": bytes.len(),
            "mime_type": mime,
        }))
    }

    pub async fn attachment_delete(&self, args: AttachmentDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_attachment(&args.attachment_id).await?;
        Ok(json!({ "ok": true, "attachment_id": args.attachment_id }))
    }
}

fn validate_save_path(raw: &str, force: bool) -> AppResult<PathBuf> {
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(AppError::validation(
            "save_path must be an absolute filesystem path",
        ));
    }
    if path.components().any(|c| c.as_os_str() == "..") {
        return Err(AppError::validation(
            "save_path must not contain '..' segments",
        ));
    }
    let display = path.to_string_lossy().to_string();
    for forbidden in FORBIDDEN_PREFIXES {
        if display.starts_with(forbidden) {
            return Err(AppError::validation(format!(
                "Refusing to write under {forbidden}; pick a path under $HOME"
            )));
        }
    }
    if !force {
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            if !path.starts_with(&home) {
                return Err(AppError::validation(format!(
                    "save_path must be inside $HOME ({}); set force_path=true to override",
                    home.display()
                )));
            }
        }
    }
    Ok(path.to_path_buf())
}

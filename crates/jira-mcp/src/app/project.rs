use serde_json::{json, Value};

use jira_core::model::{CreateProjectVersionRequest, UpdateProjectVersionRequest};

use crate::{
    error::{AppError, AppResult},
    models::{ProjectKeyArgs, ProjectVersionCreateArgs, ProjectVersionUpdateArgs},
};

use super::{shared::to_value, JiraApp};

impl JiraApp {
    pub async fn project_component_list(&self, args: ProjectKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let components = client.get_project_components(&args.project_key).await?;
        Ok(json!({
            "project_key": args.project_key,
            "components": components
        }))
    }

    pub async fn project_version_list(&self, args: ProjectKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let versions = client.get_project_versions(&args.project_key).await?;
        Ok(json!({
            "project_key": args.project_key,
            "versions": versions
        }))
    }

    pub async fn project_version_create(&self, args: ProjectVersionCreateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let version = client
            .create_project_version(&CreateProjectVersionRequest {
                name: args.name,
                project: args.project_key,
                description: args.description,
                archived: args.archived.unwrap_or(false),
                released: args.released.unwrap_or(false),
                release_date: args.release_date,
                start_date: args.start_date,
            })
            .await?;
        to_value(version)
    }

    pub async fn project_version_update(&self, args: ProjectVersionUpdateArgs) -> AppResult<Value> {
        let has_changes = args.name.is_some()
            || args.description.is_some()
            || args.archived.is_some()
            || args.released.is_some()
            || args.release_date.is_some()
            || args.start_date.is_some();
        if !has_changes {
            return Err(AppError::validation(
                "Provide at least one project version field to update",
            ));
        }

        let client = self.build_client()?;
        let version = client
            .update_project_version(
                &args.version_id,
                &UpdateProjectVersionRequest {
                    name: args.name,
                    description: args.description,
                    archived: args.archived,
                    released: args.released,
                    release_date: args.release_date,
                    start_date: args.start_date,
                },
            )
            .await?;
        to_value(version)
    }
}

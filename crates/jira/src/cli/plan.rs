use anyhow::{Context, Result};
use clap::Subcommand;
use jira_core::JiraClient;

#[derive(Debug, Subcommand)]
pub enum PlanCommand {
    /// List all Jira Plans / Advanced Roadmaps
    ///
    /// Jira Plans (formerly Advanced Roadmaps) is a Jira Premium feature.
    /// This command will return an error on Standard and Free tiers.
    ///
    /// To check your Jira tier:
    ///   jirac api get /rest/api/3/serverInfo
    ///
    /// Shows: plan ID, name, and status.
    List,
}

pub async fn handle(cmd: PlanCommand, client: JiraClient) -> Result<()> {
    match cmd {
        PlanCommand::List => list_plans(client).await,
    }
}

async fn list_plans(client: JiraClient) -> Result<()> {
    let plans = client
        .get_plans()
        .await
        .context("Failed to fetch plans — this feature requires Jira Premium")?;

    if plans.is_empty() {
        println!("No plans found.");
        return Ok(());
    }

    println!("{:<8} {:<40} STATUS", "ID", "NAME");
    println!("{}", "─".repeat(60));

    for plan in &plans {
        let id = plan.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let name = plan
            .get("title")
            .or_else(|| plan.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("—");
        let status = plan.get("status").and_then(|v| v.as_str()).unwrap_or("—");

        println!("{:<8} {:<40} {}", id, name, status);
    }

    Ok(())
}

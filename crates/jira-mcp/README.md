# jira-mcp

`jirac-mcp` exposes Jira operations over the Model Context Protocol (MCP) using the shared `jira-core` client.

This crate is published from the shared Rust workspace in the `mulhamna/jira-commands` repository.

## Install

```bash
cargo install jira-mcp
```

## Run MCP

```bash
# Local MCP clients
jirac-mcp serve --transport stdio

# Remote / HTTP MCP clients
jirac-mcp serve --transport streamable-http --host 127.0.0.1 --port 8787 --path /mcp
```

## Configuration

The server reuses the same configuration as `jirac`:

- Environment variables: `JIRA_URL`, `JIRA_EMAIL`, `JIRA_TOKEN`
- Config file: `~/.config/jira/config.toml`

You can manage credentials through MCP tools or the existing CLI:

```bash
jirac auth login
```

## Tools

- `jira_auth_status`
- `jira_auth_set_credentials`
- `jira_auth_logout`
- `jira_issue_list`
- `jira_issue_view`
- `jira_issue_types_list`
- `jira_issue_fields`
- `jira_issue_transitions_list`
- `jira_issue_create`
- `jira_issue_update`
- `jira_issue_delete`
- `jira_issue_transition`
- `jira_issue_attach`
- `jira_worklog_list`
- `jira_worklog_add`
- `jira_worklog_delete`
- `jira_issue_bulk_transition`
- `jira_issue_bulk_update`
- `jira_issue_archive`
- `jira_plan_list`
- `jira_api_request`

## Notes

- The server is tools-only in v1. It does not expose MCP prompts, resources, or the TUI.
- Destructive operations require `confirm: true`.
- Attachment uploads support either local file paths or inline base64 payloads.

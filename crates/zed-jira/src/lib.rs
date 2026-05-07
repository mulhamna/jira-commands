use std::path::Path;

use zed::serde_json::Value;
use zed::settings::ContextServerSettings;
use zed_extension_api as zed;

const CONTEXT_SERVER_ID: &str = "jira";
const REPO: &str = "mulhamna/jira-commands";

struct JiraExtension;

impl zed::Extension for JiraExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_configuration(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> zed::Result<Option<zed::ContextServerConfiguration>> {
        Ok(Some(zed::ContextServerConfiguration {
            installation_instructions: INSTALLATION_INSTRUCTIONS.to_string(),
            settings_schema: include_str!("../configuration/schema.json").to_string(),
            default_settings: include_str!("../configuration/default-settings.json").to_string(),
        }))
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        project: &zed::Project,
    ) -> zed::Result<zed::Command> {
        let settings = ContextServerSettings::for_project(CONTEXT_SERVER_ID, project)?;
        let binary = settings
            .command
            .as_ref()
            .and_then(|command| command.path.clone())
            .unwrap_or(ensure_binary()?);

        let args = settings
            .command
            .as_ref()
            .and_then(|command| command.arguments.clone())
            .unwrap_or_else(default_args);

        let mut env = env_from_settings(settings.settings.as_ref())?;
        if let Some(extra_env) = settings.command.and_then(|command| command.env) {
            env.extend(extra_env);
        }

        Ok(zed::Command {
            command: binary,
            args,
            env,
        })
    }
}

fn ensure_binary() -> zed::Result<String> {
    let release = zed::latest_github_release(
        REPO,
        zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )?;

    let asset_name = asset_name_for_platform()?;
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| format!("No release asset found for {asset_name}"))?;

    let relative_path = format!("jirac-mcp/{}/{}", release.version, asset.name);
    if !Path::new(&relative_path).exists() {
        zed::download_file(
            &asset.download_url,
            &relative_path,
            zed::DownloadedFileType::Uncompressed,
        )?;
        if !relative_path.ends_with(".exe") {
            zed::make_file_executable(&relative_path)?;
        }
    }

    Ok(relative_path)
}

fn asset_name_for_platform() -> zed::Result<&'static str> {
    let (os, arch) = zed::current_platform();

    match (os, arch) {
        (zed::Os::Linux, zed::Architecture::X8664) => Ok("jirac-mcp-linux-x86_64"),
        (zed::Os::Linux, zed::Architecture::Aarch64) => Ok("jirac-mcp-linux-aarch64"),
        (zed::Os::Mac, zed::Architecture::X8664) => Ok("jirac-mcp-macos-x86_64"),
        (zed::Os::Mac, zed::Architecture::Aarch64) => Ok("jirac-mcp-macos-aarch64"),
        (zed::Os::Windows, zed::Architecture::X8664) => Ok("jirac-mcp-windows-x86_64.exe"),
        _ => Err(format!("Unsupported platform: {os:?} {arch:?}")),
    }
}

fn default_args() -> Vec<String> {
    vec!["serve".into(), "--transport".into(), "stdio".into()]
}

fn env_from_settings(settings: Option<&Value>) -> zed::Result<Vec<(String, String)>> {
    let Some(Value::Object(settings)) = settings else {
        return Ok(vec![]);
    };

    let mut env = Vec::new();
    push_string(&mut env, settings.get("jira_url"), "JIRA_URL")?;
    push_string(&mut env, settings.get("jira_email"), "JIRA_EMAIL")?;
    push_string(&mut env, settings.get("jira_token"), "JIRA_TOKEN")?;
    push_string(&mut env, settings.get("default_project"), "JIRA_PROJECT")?;
    Ok(env)
}

fn push_string(
    env: &mut Vec<(String, String)>,
    value: Option<&Value>,
    key: &str,
) -> zed::Result<()> {
    let Some(value) = value else {
        return Ok(());
    };

    let string = value
        .as_str()
        .ok_or_else(|| format!("{key} must be a string"))?
        .trim()
        .to_string();

    if !string.is_empty() {
        env.push((key.to_string(), string));
    }

    Ok(())
}

const INSTALLATION_INSTRUCTIONS: &str = r#"Install the Jira extension from the Zed marketplace, then add your Jira credentials in Zed settings:

```json
{
  \"context_servers\": {
    \"jira\": {
      \"settings\": {
        \"jira_url\": \"https://yourcompany.atlassian.net\",
        \"jira_email\": \"you@example.com\",
        \"jira_token\": \"<JIRA_API_TOKEN>\",
        \"default_project\": \"MYPROJ\"
      }
    }
  }
}
```

You can also seed the same settings from your saved `jirac` auth profile with:

```bash
jirac mcp install --client zed
```
"#;

zed::register_extension!(JiraExtension);

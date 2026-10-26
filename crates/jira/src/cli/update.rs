use anyhow::{bail, Context, Result};
use reqwest::header::USER_AGENT;
use sha2::{Digest, Sha256};
use std::env::consts::{ARCH, EXE_SUFFIX, OS};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::version_check::{is_newer, latest_release};

/// Names one binary uses across every install channel.
pub struct Tool {
    pub binary: &'static str,
    tag_prefix: &'static str,
    crate_name: &'static str,
    brew: &'static str,
    npm: &'static str,
    scoop: &'static str,
    winget: &'static str,
}

pub const JIRAC: Tool = Tool {
    binary: "jirac",
    tag_prefix: "v",
    crate_name: "jira-commands",
    brew: "jira-commands",
    npm: "@mulham28/jirac",
    scoop: "jirac",
    winget: "mulhamna.jirac",
};

pub const JIRAC_MCP: Tool = Tool {
    binary: "jirac-mcp",
    tag_prefix: "jira-mcp-v",
    crate_name: "jira-mcp",
    brew: "jira-mcp",
    npm: "@mulham28/jirac-mcp",
    scoop: "jirac-mcp",
    winget: "mulhamna.jirac-mcp",
};

const DOWNLOAD_BASE: &str = "https://github.com/mulhamna/jira-commands/releases/download";

/// Update `tool` installed at `exe` (currently `current`) to the latest release.
pub async fn run(tool: &Tool, exe: &Path, current: &str, check_only: bool) -> Result<()> {
    let exe = exe
        .canonicalize()
        .with_context(|| format!("Failed to resolve {}", exe.display()))?;
    let release = latest_release(tool.tag_prefix, Duration::from_secs(30)).await?;

    if !is_newer(&release.latest, current) {
        println!("{} {current} is already the latest version.", tool.binary);
        return Ok(());
    }
    println!(
        "Update available: {} {current} -> {} ({})",
        tool.binary, release.latest, release.url
    );
    if check_only {
        return Ok(());
    }

    match package_manager_command(tool, &exe) {
        Some(args) => run_package_manager(&args),
        None => replace_binary(tool, &exe, &release.latest).await,
    }?;
    println!("{} updated to {}.", tool.binary, release.latest);
    Ok(())
}

/// Version reported by `<exe> --version`, e.g. `jirac-mcp 2.6.0` -> `2.6.0`.
pub fn installed_version(exe: &Path) -> Result<String> {
    let output = Command::new(exe)
        .arg("--version")
        .output()
        .with_context(|| format!("Failed to run {} --version", exe.display()))?;
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .last()
        .map(str::to_string)
        .with_context(|| format!("Unexpected `{} --version` output", exe.display()))
}

/// Install channels own their files, so hand the update to the matching tool.
/// `None` means a standalone binary (install script, manual download).
fn package_manager_command(tool: &Tool, exe: &Path) -> Option<Vec<String>> {
    // ponytail: channel guessed from the resolved path; add an install marker if this misfires
    let path = exe.to_string_lossy().replace('\\', "/").to_lowercase();
    let npm_spec = format!("{}@latest", tool.npm);
    let args: &[&str] = if path.contains("/cellar/") {
        &["brew", "upgrade", tool.brew]
    } else if path.contains("/node_modules/") {
        &["npm", "install", "-g", &npm_spec]
    } else if path.contains("/scoop/") {
        &["scoop", "update", tool.scoop]
    } else if path.contains("/winget/") {
        &["winget", "upgrade", "--id", tool.winget]
    } else if path.contains("/.cargo/bin/") {
        &["cargo", "install", tool.crate_name, "--locked"]
    } else {
        return None;
    };
    Some(args.iter().map(|s| s.to_string()).collect())
}

fn run_package_manager(args: &[String]) -> Result<()> {
    println!("Running: {}", args.join(" "));
    let status = Command::new(&args[0])
        .args(&args[1..])
        .status()
        .with_context(|| format!("Failed to run `{}`", args[0]))?;
    if !status.success() {
        bail!("`{}` exited with {status}", args.join(" "));
    }
    Ok(())
}

async fn replace_binary(tool: &Tool, exe: &Path, version: &str) -> Result<()> {
    let asset = format!("{}-{OS}-{ARCH}{EXE_SUFFIX}", tool.binary);
    let base = format!("{DOWNLOAD_BASE}/{}{version}", tool.tag_prefix);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    println!("Downloading {asset}...");
    let binary = download(&client, &format!("{base}/{asset}")).await?;
    let checksums = download(&client, &format!("{base}/checksums.txt")).await?;
    verify_checksum(&binary, &String::from_utf8_lossy(&checksums), &asset)?;

    let staged = sibling(exe, "new");
    std::fs::write(&staged, &binary).map_err(|err| permission_hint(err, exe))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }
    // Windows cannot overwrite a running exe, but it can rename it out of the way.
    #[cfg(windows)]
    let old = {
        let old = sibling(exe, "old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(exe, &old).map_err(|err| permission_hint(err, exe))?;
        old
    };
    std::fs::rename(&staged, exe).map_err(|err| {
        #[cfg(windows)]
        let _ = std::fs::rename(&old, exe);
        permission_hint(err, exe)
    })
}

async fn download(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let bytes = client
        .get(url)
        .header(USER_AGENT, concat!("jirac/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .with_context(|| format!("Failed to download {url}"))?
        .bytes()
        .await?;
    Ok(bytes.to_vec())
}

/// `checksums.txt` lines look like `<sha256>  artifacts/<asset>/<asset>`.
fn verify_checksum(binary: &[u8], checksums: &str, asset: &str) -> Result<()> {
    let expected = checksums
        .lines()
        .filter_map(|line| line.split_once(char::is_whitespace))
        .find(|(_, file)| file.trim().rsplit('/').next() == Some(asset))
        .map(|(hash, _)| hash.to_lowercase())
        .with_context(|| format!("No checksum for {asset} in checksums.txt"))?;
    let actual = format!("{:x}", Sha256::digest(binary));
    if actual != expected {
        bail!("Checksum mismatch for {asset}: expected {expected}, got {actual}");
    }
    Ok(())
}

fn sibling(exe: &Path, suffix: &str) -> PathBuf {
    let mut name = exe.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    exe.with_file_name(name)
}

fn permission_hint(err: std::io::Error, exe: &Path) -> anyhow::Error {
    let hint = if err.kind() == std::io::ErrorKind::PermissionDenied {
        " (try again with elevated permissions)"
    } else {
        ""
    };
    anyhow::anyhow!("Failed to replace {}: {err}{hint}", exe.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_install_channel_from_path() {
        let cmd =
            |path: &str| package_manager_command(&JIRAC, Path::new(path)).map(|a| a[0].clone());
        assert_eq!(
            cmd("/opt/homebrew/Cellar/jira-commands/2.11.0/bin/jirac").as_deref(),
            Some("brew")
        );
        assert_eq!(
            cmd("/usr/lib/node_modules/@mulham28/jirac/bin/jirac").as_deref(),
            Some("npm")
        );
        assert_eq!(cmd("/home/me/.cargo/bin/jirac").as_deref(), Some("cargo"));
        assert_eq!(
            cmd(r"C:\Users\me\scoop\apps\jirac\current\jirac.exe").as_deref(),
            Some("scoop")
        );
        assert_eq!(cmd("/home/me/.local/bin/jirac"), None);
    }

    #[test]
    fn verifies_checksum_by_asset_name() {
        let binary = b"jirac";
        let hash = format!("{:x}", Sha256::digest(binary));
        let checksums = format!(
            "{hash}  artifacts/jirac-linux-x86_64/jirac-linux-x86_64\n\
             deadbeef  artifacts/jirac-linux-x86_64/jirac-linux-x86_64.tar.gz\n"
        );
        assert!(verify_checksum(binary, &checksums, "jirac-linux-x86_64").is_ok());
        assert!(verify_checksum(b"tampered", &checksums, "jirac-linux-x86_64").is_err());
        assert!(verify_checksum(binary, &checksums, "jirac-macos-aarch64").is_err());
    }
}

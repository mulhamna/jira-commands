//! Non-interactive guard shared across CLI flows.
//!
//! Set once from `main` and read by a small guard helper, so interactive flows
//! can refuse to prompt (and give an actionable message) when there is no TTY —
//! the agent/CI/MCP case — instead of blocking or emitting inquire's generic
//! error. Mirrors the `stdout().is_terminal()` pattern in `progress.rs`.

use std::io::IsTerminal;
use std::sync::OnceLock;

use anyhow::{bail, Result};

static NON_INTERACTIVE: OnceLock<bool> = OnceLock::new();

/// Called once from `main`. Non-interactive if the flag is set, the
/// `JIRAC_NON_INTERACTIVE` env var is set, or stdin is not a TTY.
pub fn init(flag: bool) {
    let val = flag
        || std::env::var_os("JIRAC_NON_INTERACTIVE").is_some()
        || !std::io::stdin().is_terminal();
    let _ = NON_INTERACTIVE.set(val);
}

pub fn is_non_interactive() -> bool {
    *NON_INTERACTIVE.get().unwrap_or(&false)
}

/// Guard placed at the entry of an interactive flow. `what` names the thing being
/// asked for, `flag` names the CLI flag that supplies it non-interactively.
pub fn require_interactive(what: &str, flag: &str) -> Result<()> {
    if is_non_interactive() {
        bail!("cannot prompt for {what}: no interactive terminal. Pass {flag} instead (or run without --non-interactive on a terminal).");
    }
    Ok(())
}

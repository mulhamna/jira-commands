//! Helpers for tests that change the process environment.

use std::{
    ffi::OsString,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Serialises tests that change `HOME` or `JIRA_*`. A failing test does not poison it.
pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

const HOME_VARS: [&str; 5] = [
    "HOME",
    "XDG_CONFIG_HOME",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
];
const JIRA_VARS: [&str; 9] = [
    "JIRA_PROFILE",
    "JIRA_URL",
    "JIRA_EMAIL",
    "JIRA_TOKEN",
    "JIRA_PROJECT",
    "JIRA_DEPLOYMENT",
    "JIRA_AUTH_TYPE",
    "JIRA_API_VERSION",
    "JIRA_TEXT_FORMAT",
];

/// Temporary config home with `JIRA_*` cleared; restores the environment on drop.
pub(crate) struct TempHome {
    dir: PathBuf,
    saved: Vec<(&'static str, Option<OsString>)>,
    _guard: MutexGuard<'static, ()>,
}

impl TempHome {
    pub(crate) fn new(name: &str) -> Self {
        let guard = env_lock();
        let dir = std::env::temp_dir().join(format!("jirac-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp home");
        let saved = HOME_VARS
            .iter()
            .chain(JIRA_VARS.iter())
            .map(|key| (*key, std::env::var_os(key)))
            .collect();
        for key in HOME_VARS {
            std::env::set_var(key, &dir);
        }
        for key in JIRA_VARS {
            std::env::remove_var(key);
        }
        Self {
            dir,
            saved,
            _guard: guard,
        }
    }

    pub(crate) fn config_text(&self) -> String {
        std::fs::read_to_string(jira_core::config::config_file_path()).unwrap_or_default()
    }

    pub(crate) fn write_config(&self, content: &str) {
        let path = jira_core::config::config_file_path();
        std::fs::create_dir_all(path.parent().expect("config dir")).expect("mkdir");
        std::fs::write(path, content).expect("write config");
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

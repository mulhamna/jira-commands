use std::io::IsTerminal;

use indicatif::{ProgressBar, ProgressStyle};

/// Spinner that hides itself when stdout is not a TTY (piped/redirected output).
pub fn spinner_new(msg: impl Into<String>) -> ProgressBar {
    if !std::io::stdout().is_terminal() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .expect("spinner template is a compile-time constant"),
    );
    pb.set_message(msg.into());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Determinate progress bar that hides itself when stdout is not a TTY.
pub fn progress_bar(len: u64) -> ProgressBar {
    if !std::io::stdout().is_terminal() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40}] {pos}/{len} {msg}")
            .expect("progress bar template is a compile-time constant")
            .progress_chars("=> "),
    );
    pb
}

use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

pub fn status(message: impl AsRef<str>) {
    println!("{}", message.as_ref());
}

pub fn install_start(package: &str, version: &str) {
    status(format!("==> Installing {package}@{version}"));
}

pub fn install_success(package: &str, version: &str) {
    status(format!("Installed {package}@{version}"));
}

pub fn using_version(package: &str, version: &str) {
    status(format!("Using {package}@{version}"));
}

pub fn already_installed(package: &str, version: &str) {
    status(format!("Already installed {package}@{version}"));
}

pub fn already_up_to_date() {
    status("Already up to date");
}

pub fn spinner(message: impl Into<String>) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message(message.into());
    spinner
}

pub fn download_bar(total: u64, message: impl Into<String>) -> ProgressBar {
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template(
            "{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} ETA {eta}",
        )
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("=>-"),
    );
    bar.set_message(message.into());
    bar
}

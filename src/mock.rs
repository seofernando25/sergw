// no extra imports
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::os::unix::io::OwnedFd;

#[cfg(target_os = "linux")]
use anyhow::Result;
#[cfg(target_os = "linux")]
use nix::pty::{openpty, OpenptyResult};

// no cli imports needed here

#[cfg(target_os = "linux")]
fn create_pty_pair() -> Result<(OwnedFd, OwnedFd, String)> {
    let OpenptyResult { master, slave, .. } = openpty(None, None)?;
    // Resolve stable path to the slave PTY for use as a serial device path
    let slave_symlink = format!("/proc/self/fd/{}", slave.as_raw_fd());
    let slave_path = std::fs::read_link(&slave_symlink)?;
    Ok((master, slave, slave_path.to_string_lossy().into_owned()))
}

#[cfg(target_os = "linux")]
pub fn run_mock_serial() -> Result<()> {
    let (master, _slave_fd, slave_path) = create_pty_pair()?;
    // If user requested an alias symlink, create it (best effort)
    if let Ok(alias) = std::env::var("SERGW_PTY_ALIAS") {
        let _ = std::fs::remove_file(&alias);
        let _ = std::os::unix::fs::symlink(&slave_path, &alias);
    }
    // Show compact one-line header inside the UI
    super::tui_chat_mock::run_mock_chat_with_title(master, format!("mock serial | {}", slave_path))?;

    Ok(())
}



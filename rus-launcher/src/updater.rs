//! Swapping this copy of the launcher for the one GitHub just published.
//!
//! On Linux and macOS a file can be replaced while the process running it is
//! still alive: the new build takes the old one's name, the old inode lives on
//! until the process leaves, and the new build is started straight away.
//! Windows keeps a running .exe locked, so there the swap is handed to a small
//! script that waits for us to be gone, moves the file and opens it again.

use std::path::{Path, PathBuf};

/// Where the running launcher lives.
///
/// An AppImage keeps its own path in $APPIMAGE, because `current_exe()` only
/// sees the copy mounted inside it, and that is not the file to replace.
pub fn current() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("APPIMAGE") {
        if !value.is_empty() {
            return Some(PathBuf::from(value));
        }
    }
    std::env::current_exe().ok()
}

/// Puts the download where this copy is running from and starts it. A
/// successful call hands the window over to the new build, so the caller
/// should save its state and leave.
pub fn replace_and_relaunch(downloaded: &Path) -> Result<(), String> {
    let target = current()
        .ok_or_else(|| "We cannot tell where this copy of the launcher lives.".to_string())?;
    let target = std::fs::canonicalize(&target).unwrap_or(target);
    let staged = beside(&target);
    // a download in another file system cannot be renamed across, so copy it
    // the long way when that happens
    if std::fs::rename(downloaded, &staged).is_err() {
        std::fs::copy(downloaded, &staged)
            .map_err(|_| "The new build could not be placed next to this one.".to_string())?;
        let _ = std::fs::remove_file(downloaded);
    }
    make_runnable(&staged);
    swap_and_start(&staged, &target)
}

fn beside(target: &Path) -> PathBuf {
    let mut name = target
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "rus-launcher".to_string());
    name.push_str(".new");
    target.with_file_name(name)
}

#[cfg(unix)]
fn make_runnable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
}

#[cfg(not(unix))]
fn make_runnable(_path: &Path) {}

#[cfg(unix)]
fn swap_and_start(staged: &Path, target: &Path) -> Result<(), String> {
    // from this moment the name belongs to the new build; the process that is
    // still running keeps the old inode until it exits
    std::fs::rename(staged, target)
        .map_err(|_| "The old build could not be replaced.".to_string())?;
    start(target);
    Ok(())
}

#[cfg(windows)]
fn swap_and_start(staged: &Path, target: &Path) -> Result<(), String> {
    let script = std::env::temp_dir().join("rus-launcher-update.cmd");
    let body = format!(
        "@echo off\r\n\
         ping -n 3 127.0.0.1 >nul\r\n\
         move /Y \"{new}\" \"{old}\"\r\n\
         start \"\" \"{old}\"\r\n\
         del \"%~f0\"\r\n",
        new = staged.display(),
        old = target.display()
    );
    std::fs::write(&script, body)
        .map_err(|_| "The update script could not be written.".to_string())?;
    use std::os::windows::process::CommandExt;
    std::process::Command::new("cmd")
        .args(["/C", &script.to_string_lossy()])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|_| "The update could not be started.".to_string())?;
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn swap_and_start(_staged: &Path, _target: &Path) -> Result<(), String> {
    Err("Updating is not wired up on this system yet.".to_string())
}

#[cfg(unix)]
fn start(path: &Path) {
    let mut command = std::process::Command::new(path);
    command.args(std::env::args_os().skip(1));
    // if it does not start, the new build is still in place for next time
    let _ = command.spawn();
}

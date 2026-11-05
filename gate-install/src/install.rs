use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub const RELEASES: &str = "https://github.com/obsifox/rus-launcher/releases/latest/download";
pub const VERSION_FILE: &str = "version.txt";

#[cfg(windows)]
pub const LAUNCHER: &str = "rus-launcher-windows-x86_64.exe";
#[cfg(target_os = "linux")]
pub const LAUNCHER: &str = "rus-launcher-x86_64.AppImage";
#[cfg(target_os = "macos")]
pub const LAUNCHER: &str = "rus-launcher-macos";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Default)]
pub struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    pub fn line(&self, text: impl Into<String>) {
        let mut lines = self.0.lock().expect("log");
        let text: String = text.into();
        if lines.last().map(String::as_str) != Some(text.as_str()) {
            lines.push(text);
        }
    }
    pub fn lines(&self) -> Vec<String> {
        self.0.lock().expect("log").clone()
    }
}

pub fn app_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("GateInstall")
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        home.join("Library/Application Support/GateInstall")
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("gate-install")
    }
}

fn current_exe() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

#[cfg(windows)]
fn quiet(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn quiet(cmd: &mut Command) {
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
}

#[cfg(windows)]
fn register(dir: &Path, exe: &Path) {
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\GateInstall";
    let exe = exe.to_string_lossy().replace('\\', "\\\\");
    let dir = dir.to_string_lossy().replace('\\', "\\\\");
    let values: [(&str, &str, &str); 6] = [
        ("DisplayName", "REG_SZ", "Gate Install"),
        ("Publisher", "REG_SZ", "Obsifox Studio"),
        ("DisplayVersion", "REG_SZ", env!("CARGO_PKG_VERSION")),
        ("InstallLocation", "REG_SZ", &dir),
        ("DisplayIcon", "REG_SZ", &exe),
        ("UninstallString", "REG_SZ", &format!("\"{}\" /uninstall", exe)),
    ];
    for (name, kind, value) in values {
        let mut cmd = Command::new("reg");
        quiet(cmd.args(["add", key, "/v", name, "/t", kind, "/d", value, "/f"]));
        let _ = cmd.output();
    }
}

#[cfg(windows)]
fn shortcuts(dir: &Path, exe: &Path) {
    let script = dir.join("gate-shortcuts.ps1");
    let ps = format!(
        "$ws = New-Object -ComObject WScript.Shell\n\
         $target = '{exe}'\n\
         $work = '{dir}'\n\
         $paths = @()\n\
         $paths += Join-Path ([Environment]::GetFolderPath('Desktop')) 'Gate Install.lnk'\n\
         $paths += Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs\\Gate Install.lnk'\n\
         foreach ($p in $paths) {{\n\
           $s = $ws.CreateShortcut($p)\n\
           $s.TargetPath = $target\n\
           $s.WorkingDirectory = $work\n\
           $s.Description = 'Gate Install'\n\
           $s.Save()\n\
         }}\n",
        exe = exe.to_string_lossy(),
        dir = dir.to_string_lossy()
    );
    if fs::write(&script, ps).is_err() {
        return;
    }
    let mut cmd = Command::new("powershell");
    quiet(cmd.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &script.to_string_lossy(),
    ]));
    let _ = cmd.output();
    let _ = fs::remove_file(script);
}

#[cfg(not(windows))]
fn entry_point(dir: &Path, exe: &Path) {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(home) = home {
        let apps = home.join(".local/share/applications");
        let _ = fs::create_dir_all(&apps);
        let file = apps.join("gate-install.desktop");
        let body = format!(
            "[Desktop Entry]\nType=Application\nName=Gate Install\nExec=\"{}\"\nPath={}\nTerminal=false\nCategories=Utility;\n",
            exe.to_string_lossy(),
            dir.to_string_lossy()
        );
        let _ = fs::write(file, body);
    }
}

/// Puts a copy of this program in the app directory, registers it, and leaves a shortcut.
pub fn ensure_installed(log: &Log) -> PathBuf {
    let dir = app_dir();
    if fs::create_dir_all(&dir).is_err() {
        log.line("Could not create the app folder.");
        return PathBuf::from(".");
    }

    if let Some(exe) = current_exe() {
        let installed = dir.join(file_name(&exe));
        let same = fs::canonicalize(&exe)
            .ok()
            .zip(fs::canonicalize(&installed).ok())
            .map(|(a, b)| a == b)
            .unwrap_or(false);

        if !same {
            log.line("Installing Gate Install.");
            if let Ok(bytes) = fs::read(&exe) {
                if fs::write(&installed, bytes).is_err() {
                    log.line("Could not copy the installer.");
                }
            }
            #[cfg(windows)]
            {
                register(&dir, &installed);
                log.line("Registered in the Windows registry.");
                shortcuts(&dir, &installed);
                log.line("Shortcuts added to the desktop and start menu.");
            }
            #[cfg(not(windows))]
            {
                entry_point(&dir, &installed);
                log.line("Application entry added.");
            }
            return dir;
        }
        log.line("Gate Install is already installed.");
        return dir;
    }

    log.line("Could not find this program on disk.");
    dir
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "gate-install".to_string())
}

/// Fetches a file with what the system already has: PowerShell on Windows, curl or wget elsewhere.
fn fetch(url: &str, dest: &Path) -> bool {
    let _ = fs::remove_file(dest);

    #[cfg(windows)]
    {
        let out = dest.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$ErrorActionPreference='Stop'; \
             [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; \
             Invoke-WebRequest -Uri '{url}' -OutFile '{out}'",
            url = url.replace('\'', "''"),
            out = out
        );
        let mut cmd = Command::new("powershell");
        quiet(cmd.args(["-NoProfile", "-Command", &script]));
        match cmd.output() {
            Ok(done) => done.status.success() && dest.exists(),
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    {
        let out = dest.to_string_lossy().to_string();
        for program in ["curl", "wget"] {
            let mut cmd = Command::new(program);
            if program == "curl" {
                cmd.args(["-fsSL", "--max-time", "600", "-o", &out, url]);
            } else {
                cmd.args(["-q", "-T", "600", "-O", &out, url]);
            }
            quiet(&mut cmd);
            if let Ok(done) = cmd.status() {
                if done.success() && dest.exists() {
                    return true;
                }
            }
        }
        false
    }
}

fn remote_version(dir: &Path) -> Option<String> {
    let probe = dir.join("latest-version.txt");
    let url = format!("{}/{}", RELEASES, VERSION_FILE);
    let got = fetch(&url, &probe);
    let text = fs::read_to_string(&probe).ok().map(|s| s.trim().to_string());
    let _ = fs::remove_file(&probe);
    if !got {
        return None;
    }
    text.filter(|v| !v.is_empty())
}

/// Downloads the launcher when it is missing or out of date. Returns the path to run.
pub fn sync_launcher(dir: &Path, log: &Log) -> Option<PathBuf> {
    let target = dir.join(LAUNCHER);
    let stamp = dir.join("launcher-version.txt");

    let installed: Option<String> = fs::read_to_string(&stamp).ok().map(|s| s.trim().to_string());

    log.line("Checking for updates.");
    match remote_version(dir) {
        None => {
            log.line("No connection. Nothing to do.");
        }
        Some(latest) => {
            let fresh = installed.as_deref() == Some(latest.as_str()) && target.exists();
            if !fresh {
                log.line(if target.exists() {
                    "A newer launcher is available."
                } else {
                    "Downloading the launcher."
                });
                let url = format!("{}/{}", RELEASES, LAUNCHER);
                if fetch(&url, &target) {
                    log.line("Launcher downloaded.");
                    let _ = fs::write(&stamp, latest);
                } else {
                    log.line("The download did not finish.");
                    let _ = fs::remove_file(&target);
                }
            } else {
                log.line("The launcher is up to date.");
            }
        }
    }

    if target.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target, fs::Permissions::from_mode(0o755));
        }
        Some(target)
    } else {
        None
    }
}

/// Starts the launcher and leaves it running on its own.
pub fn launch(path: &Path) -> bool {
    let mut cmd = Command::new(path);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn().is_ok()
}

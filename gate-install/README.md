# Gate Install

English desktop installer and updater for Rus Launcher.

On launch the studio image is centered for a few seconds, with no window background. Then the
window opens and the gate does its work:

1. **Install** — copies itself into the app folder (`%LOCALAPPDATA%\GateInstall` on Windows,
   `~/.local/share/gate-install` on Linux).
2. **Register** — on Windows writes the uninstall entry into
   `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\GateInstall` and leaves a shortcut on
   the desktop and in the start menu. On Linux it writes an application entry.
3. **Download** — fetches `version.txt` from the latest release. When the launcher is missing or
   a newer one is out, it downloads it into the app folder and marks it executable.
4. **Launch** — starts Rus Launcher and closes itself.

The shortcut always points at Gate Install, so every start is a check for updates: something new
comes down, otherwise the launcher just opens. With no connection nothing is downloaded, and
without a launcher nothing starts.

Downloads use what the system already has: PowerShell on Windows, `curl` or `wget` elsewhere.

```
cargo build --release
```

## Uninstall

Windows: the registry entry uninstalls with `gate-install.exe /uninstall`.

## Download

- Windows: `gate-install-windows-x86_64.exe`
- Linux: `gate-install-linux-x86_64`

Every asset is embedded in the binary, so it runs on its own.

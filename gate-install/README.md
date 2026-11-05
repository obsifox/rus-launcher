# Gate Install

English desktop installer.

On launch the studio image is centered for 6 seconds, with no window background. Then the Gate Install window opens: the character fills the frame, the ObsiFox Studio mark sits at the top, and Install is the main action.

```
cargo run --release
```

## Build

```
cargo build --release
```

Both binaries are also built on every push by CI.

## Download

- Windows: `gate-install-windows-x86_64.exe` from the releases page
- Linux: `gate-install-linux-x86_64`

Every asset is embedded in the binary, so the exe runs on its own.

#!/usr/bin/env bash
# Brings a fresh sandbox back to a state where the launcher builds and runs.
# The sandbox forgets installed packages and the Rust toolchain between turns,
# so every session starts with:  bash scripts/sandbox.sh
set -e

export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"
export PATH="$HOME/.cargo/bin:$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"

# the snapshot keeps the rustup binary but drops its executable bit
[ -x "$HOME/.cargo/bin/rustup" ] || chmod +x "$HOME/.cargo/bin/rustup"

command -v rustc >/dev/null 2>&1 || rustup toolchain install stable --profile minimal

need_apt=0
for tool in Xvfb xdotool matchbox-window-manager import convert; do
    command -v "$tool" >/dev/null 2>&1 || need_apt=1
done
if [ "$need_apt" = 1 ]; then
    sudo apt-get update -qq
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
        xvfb xdotool matchbox-window-manager tesseract-ocr imagemagick \
        libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
        libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
        libgl1-mesa-dev libgtk-3-dev
fi

cd "$(dirname "$0")/../rus-launcher"
cargo build --release

echo
echo "Build ready: rus-launcher/target/release/rus-launcher"
echo "Run it with:  bash scripts/run.sh"

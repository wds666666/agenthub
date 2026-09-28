# Development and verification

## Ubuntu 24.04 / WSL2

Install the Tauri 2 native dependencies from a terminal where you can enter the sudo password:

```bash
sudo apt-get update
sudo apt-get install -y build-essential libwebkit2gtk-4.1-dev libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Install Rust stable, rustfmt, clippy, and the Tauri CLI:

```bash
rustup toolchain install stable --profile minimal
rustup component add rustfmt clippy
cargo install tauri-cli --version '^2' --locked
```

Then run:

```bash
pnpm install
pnpm lint
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy -p agenthub-core -p agenthub-cli --all-targets -- -D warnings
cargo test -p agenthub-core
cargo check -p agenthub-desktop
cargo tauri build
```

All adapter integration tests must use a temporary `HOME`. Never run `sync` during development without inspecting its generated Plan and confirming that `HOME` points to a disposable fixture.


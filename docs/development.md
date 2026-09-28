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

## Windows 10/11 x64 packages

Install the current Git for Windows, Node.js 24, pnpm 12, Rust stable with the MSVC toolchain, Microsoft C++ Build Tools, and WebView2 development prerequisites. Install the matching Tauri CLI with `cargo install tauri-cli --version 2.12.0 --locked`. The application uses Git at runtime for Canonical history, so `git.exe` must remain available on `PATH` after installation.

Build both the current-user NSIS installer and MSI package from PowerShell:

```powershell
./scripts/package-windows.ps1
```

Use `-SkipInstall` when the lockfile dependencies are already installed, or `-Bundle nsis` / `-Bundle msi` to build one installer type. Outputs are written to:

```text
target/release/bundle/nsis/*-setup.exe
target/release/bundle/msi/*.msi
target/release/bundle/SHA256SUMS.windows.txt
target/release/agenthub.exe
target/release/agenthub-desktop.exe
```

The installer is per-user and does not require administrator rights. It includes the MSVC runtime, an embedded WebView2 bootstrapper, Chinese and English NSIS strings, the desktop application, and the `agenthub.exe` CLI sidecar. The bootstrapper may still need network access if WebView2 is absent; supported Windows 10/11 installations normally already include WebView2.

The same build runs through `.github/workflows/windows-build.yml` for manual dispatches and `v*` tags. Windows artifacts are unsigned in v0.1; SmartScreen may warn until an Authenticode certificate and timestamp service are configured. MSI generation also requires the Windows VBSCRIPT optional feature, which is enabled on standard GitHub-hosted Windows runners.

Do not treat Linux-to-Windows cross-compilation as the release path. Tauri supports NSIS cross-compilation with caveats, but MSI/WiX remains Windows-only; the native Windows workflow is the reproducible source for both installers.

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

The installer is per-user and does not require administrator rights. It includes the MSVC runtime, the offline WebView2 installer, Chinese and English NSIS strings, the desktop application, and the `agenthub.exe` CLI sidecar. The larger offline bundle prevents installation from depending on a WebView2 download. Release `agenthub-desktop.exe` uses the Windows GUI subsystem and must not open a console; `agenthub.exe` remains a console CLI. The packaging script reads both PE headers and rejects a desktop Console subsystem or a CLI GUI subsystem regression.

The Windows-only build runs through `.github/workflows/windows-build.yml` for manual CI dispatches. The combined **Build packages** workflow remains at `.github/workflows/release.yml` and defaults to build-only: `gh workflow run release.yml --ref main` verifies/builds both platforms and uploads temporary `linux-x64` and `windows-x64` artifacts. Each successful upload adds its direct artifact download link to the run summary. Artifacts expire after 14 days and downloading them requires a GitHub login; report both the artifact links and run URL. A build request or request for a download link is not permission to create a Release.

Only after explicit user approval for a specific release, dispatch `gh workflow run release.yml --ref main -f publish_release=true -f tag=vX.Y.Z`. The tag must match the package version and `docs/releases/vX.Y.Z.md` must describe implemented features and known gaps. This builds the selected ref and creates the release at that exact commit; it refuses an existing release or a tag pointing elsewhere. Tag pushes do not trigger publication. Default builds have read-only repository permissions; only the explicitly enabled publish job gets write permission. Prior publication approval does not authorize later releases.

Ubuntu 24.04 x64 produces a `.deb`, the desktop executable, and the CLI. AppImage is not produced: `linuxdeploy` cannot run on GitHub-hosted Ubuntu 24.04. Windows x64 produces NSIS, MSI, the portable desktop executable, and the CLI, plus SHA-256 checksums. Windows artifacts are unsigned in v0.1; SmartScreen may warn until an Authenticode certificate and timestamp service are configured. MSI generation also requires the Windows VBSCRIPT optional feature, which is enabled on standard GitHub-hosted Windows runners.

Do not treat Linux-to-Windows cross-compilation as the release path. Tauri supports NSIS cross-compilation with caveats, but MSI/WiX remains Windows-only; the native Windows workflow is the reproducible source for both installers.

Windows smoke testing must cover more than process startup:

- launch the installed desktop app from Explorer and confirm no console window appears;
- run initialization with empty, UTF-16/non-UTF-8, oversized, CRLF, duplicate-name, and Windows-reserved-name resources;
- verify invalid discoveries are visible but not selectable and that a failed import leaves Canonical empty;
- test user homes containing spaces and non-ASCII characters, and keep Canonical/WebView2 data on a writable local profile path rather than UNC or network storage;
- run without Git on `PATH` and confirm Doctor reports the dependency instead of presenting an opaque process error;
- inspect `%LOCALAPPDATA%/dev.agenthub.desktop/logs` after a successful run and a forced import failure, confirming secret values are redacted;
- verify both NSIS and MSI install/uninstall, offline WebView2 provisioning, SmartScreen behavior for unsigned builds, and SHA-256 checksums.

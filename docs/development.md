# 开发与编译

先安装 Node.js 24、pnpm 12、Rust stable 和系统 Git。项目使用 Tauri 2，原生依赖按平台安装。

## Ubuntu / WSL2

```bash
sudo apt-get install -y build-essential libwebkit2gtk-4.1-dev libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
cargo install tauri-cli --version 2.12.0 --locked
pnpm install --frozen-lockfile
pnpm tauri dev
```

本地安装包与程序：

```bash
cargo build --release -p agenthub-cli
cargo tauri build --bundles deb
```

输出位于 `target/release/agenthub-desktop`、`target/release/agenthub` 和 `target/release/bundle/deb/`。

## Windows

安装 MSVC Rust 工具链、Microsoft C++ Build Tools、WebView2 开发依赖和 Tauri CLI。执行：

```powershell
./scripts/package-windows.ps1
```

生成 NSIS、MSI、完整便携 ZIP、独立 CLI 和校验文件，位于 `target/release/` 及其 `bundle/` 子目录。安装版包含 CLI、管理 Skill 和离线 WebView2，注册当前用户的命令路径；桌面程序采用 GUI 子系统，CLI 保留控制台子系统。便携 ZIP 包含两者及命令定位脚本。Ubuntu `.deb` 同时安装 `/usr/bin/agenthub` 和管理 Skill。当前 Windows 安装包未签名。

双平台构建在临时 Windows runner 上验证 NSIS/MSI 安装与卸载、同版本升级、长 PATH 保留、路径归属、旧环境下的 Skill 定位及 ZIP 完整性。`scripts/test-windows-packages.ps1` 仅允许隔离的 GitHub Actions 环境执行，避免修改开发者的环境变量。

## 检查

```bash
python3 scripts/check_project.py
pnpm lint
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy -p agenthub-core -p agenthub-cli --all-targets -- -D warnings
cargo test -p agenthub-core -p agenthub-cli
cargo check -p agenthub-desktop
```

宿主测试使用 `TempDir` 与显式 `AgentHubPaths::for_home`，不使用真实用户目录。`AGENTHUB_HOME` 仅改变能力库位置，不会隔离宿主扫描目录。涉及清理、同步或恢复的测试必须使用临时宿主布局。

自动检查、临时构建与发布条件见 [工作流规则](workflow.md)。普通文档修改无需构建安装包；用户未指定版本号时，保留当前版本。

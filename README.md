# AgentHub

AgentHub v0.1 is a local-first canonical manager for user-global Skills, MCP servers, Plugins and Rules across Cursor, Codex and Claude Code.

```bash
pnpm install
pnpm test
pnpm build
cargo test --workspace
cargo run -p agenthub-cli -- doctor
pnpm tauri dev
```

The runtime root defaults to `~/.agenthub`. For tests and isolated trials, set `HOME` to a temporary directory and optionally set `AGENTHUB_HOME` explicitly. Review a Plan before the first manual sync, or explicitly enable an automatic-sync profile after checking its target and selected capability scope. Automatic sync is triggered only by Canonical mutations performed through AgentHub; it is not a background filesystem watcher.

See `docs/architecture.md` and `docs/sync-contract.md` before changing storage or adapter behavior.
Ubuntu/WSL setup and the complete verification commands are documented in `docs/development.md`.

## Windows packages

Windows 10/11 x64 builds produce a current-user NSIS setup executable, an MSI, the standalone desktop executable, the `agenthub.exe` CLI, and SHA-256 checksums. Run `./scripts/package-windows.ps1` from PowerShell on Windows, or start the **Windows packages** GitHub Actions workflow. See [`docs/development.md`](docs/development.md#windows-1011-x64-packages) for prerequisites, output paths, WebView2 behavior, and the current unsigned-build limitation.

## External agent skill

The reusable external-agent integration is in [`skills/agenthub-manager`](skills/agenthub-manager/SKILL.md). It teaches an agent how to inspect AgentHub, review scoped Plans, request confirmation before sync/rollback, and preserve the Canonical/Git/SQLite/backups boundaries.

### 重置、清理与多设备同步（0.1.1）

- **设置 → 重置 AgentHub**：输入 `AGENTHUB` 确认后，当前库、版本、本机状态与自动同步设置会移出活动目录，返回扫描导入向导。完整恢复副本位于库目录同级的 `.agenthub-reset-*`，包含密钥；确认不需要后再手动清理。不会删除 Cursor、Codex 或 Claude Code 中的内容。
- **扫描导入**：先按来源工具选择，再按 Skills/MCP/Plugins/Rules 筛选。跨工具选择始终保留，显示总选择数及去重后预计导入数。当前来源的全选不会影响其他来源。
- **工具资源 → 快速清理**：清理当前工具全部可删除资源。确认弹窗列出路径并要求勾选确认；先备份，失败时恢复。官方插件市场、缓存及受保护目录仍不能直接删除。自动同步可能在下一次库修改时重新创建清理掉的库资源。
- **版本记录 → 多设备同步**：填写专用 AgentHub 仓库的 HTTPS/SSH 地址和分支，验证读取权限后连接。每次保存版本都会先保存本机，再获取并合并远端，最后上传。其他设备保存自己的初始版本后点击“同步远端”即可接收；接收后请按需预览并同步到工具。无轮询后台任务。
- 仓库须专用于 AgentHub，不能包含 README、项目代码、运行状态、密钥、符号链接或子模块。建议创建空仓库。断开连接仅停止自动上传，本机版本保留。
- 登录使用系统 Git 凭据管理器或 SSH Agent 已保存的授权；姓名、邮箱只标记版本作者。请先在系统凭据管理器授权，或配置 SSH Agent 和可信主机。应用不会弹终端、密码提示或浏览器登录窗口。读取成功不保证写入权限；首次推送会验证写入权限。
- 网络或认证失败时，本机版本仍已保存，修复凭据后点击“同步远端”重试。同一资源的合并冲突会列出路径并停止，完整保留本地提交；请协调两台设备的资源内容后再保存和重试。推送不会强制覆盖，遇到并发远端更新最多再合并重试一次。

合并及凭据处理参考 [Git merge](https://git-scm.com/docs/git-merge) 和 [Git credentials](https://git-scm.com/docs/gitcredentials)。

CLI 同样支持 `agenthub git connect <url> --branch main`、`git remote-status`、`git sync` 和 `git disconnect`。`git commit` 返回 JSON，区分本机保存成功与远端同步失败。

上传会检查版本历史中的运行目录及可识别的 MCP/JSON 明文凭据；发现后仅显示路径并阻止上传。仅删除当前文件中的密钥不足以清除历史，需先处理敏感历史。任意 Markdown 或二进制中嵌入的秘密无法完全自动识别，请在共享前检查库内容。

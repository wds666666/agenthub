# AgentHub

[中文](README.md) · [English](docs/README.en.md)

集中管理 Cursor、Codex 和 Claude Code 的用户级全局能力，通过 Git 保存版本并在多台设备间共享。

## 功能

- 按工具与类别选择导入，自动跳过相同内容。
- 管理 Skills、MCP 和规则，支持搜索、预览、多选及批量删除。
- 同步前预览变更，写入时备份和校验，支持恢复与按范围自动同步。
- 通过 Git 仓库保存及同步版本；备份、密钥和设备设置留在本机。

**插件支持尚未完成**：Codex、Claude Code 官方插件缓存目前仅扫描展示，不支持完整导入与同步。其他限制见 [发行说明](docs/releases/v0.1.3.md)。

## 下载与使用

从 [发行版](https://github.com/wds666666/agenthub/releases/latest) 下载 Windows x64 或 Ubuntu x64 安装包。临时构建可在 [Actions](https://github.com/wds666666/agenthub/actions) 下载，保留 14 天，需要登录 GitHub。使用前安装系统 Git；Windows 安装包包含 WebView2。

1. 首次启动，扫描并选择要导入的全局能力。
2. 在“同步到工具”选择目标与范围，检查变更预览后应用。
3. 在“版本记录”保存修改；需要多设备同步时连接专用 Git 仓库。

仅管理用户级配置。清理和重置前会提示影响范围；请确认后再执行。

## 开发与文档

```bash
pnpm install
pnpm tauri dev
```

- [开发与编译](docs/development.md) · [验证、构建与发布规则](docs/workflow.md)
- [架构](docs/architecture.md) · [存储标准](docs/agenthub-standard.md) · [同步契约](docs/sync-contract.md)
- [安全边界](docs/security.md) · [外部 Agent 管理 skill](skills/agenthub-manager/SKILL.md)

默认沿用当前版本号；只有明确同意发布时才创建发行版。

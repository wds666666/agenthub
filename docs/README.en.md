# AgentHub

[中文](../README.md) · [English](README.en.md)

Manage user-global capabilities for Cursor, Codex and Claude Code. Keep versions in Git and share the library across devices.

## Features

- Select imports by tool and category; deduplicate identical content.
- Search, preview and manage Skills, MCP servers and rules, including bulk deletion.
- Review synchronization changes, back up writes, verify results and recover previous tool state.
- Save and synchronize library versions through Git. Backups, keys and device settings stay local.

**Plugin support is incomplete.** Official Codex and Claude Code plugin caches are currently displayed only; complete import and synchronization are unavailable. See the [release notes](releases/v0.1.4-patch.1.md) for the current scope.

## Download and use

Get Windows x64 or Ubuntu x64 packages from [Releases](https://github.com/wds666666/agenthub/releases/latest). **One installer supplies the UI and CLI; no separate CLI download is needed.** Prefer the Windows `-setup.exe`, which registers the user command path and includes WebView2, or the Ubuntu `.deb`. Install system Git before use.

The Windows [portable ZIP](portable-windows.md) contains both executables. Separate CLI downloads are for command-line-only use. Temporary [Actions artifacts](https://github.com/wds666666/agenthub/actions) require GitHub sign-in and expire after 14 days.

1. Scan and select global capabilities during setup.
2. Choose a tool and scope, review the change preview, then apply.
3. Save a version; connect a dedicated Git repository to synchronize between devices.

AgentHub manages user-global configuration. Review the stated impact before cleanup or reset.

## Development and documentation

```bash
pnpm install
pnpm tauri dev
```

- [Development](development.md) · [Workflow policy](workflow.md)
- [Architecture](architecture.md) · [Canonical storage](agenthub-standard.md) · [Sync contract](sync-contract.md)
- [Security](security.md) · [Agent integration skill](../skills/agenthub-manager/SKILL.md)

Read the [migration guide](../skills/agenthub-manager/references/migration.md) before replacing an existing backup workflow. Obtain the optional management skill separately and copy it manually into your tool's user-level Skills directory. It covers command discovery, formats, selected previews and Git conflict resolution, and identifies operations that still require the Desktop.

The existing version is retained unless a new version is explicitly requested. Publishing requires explicit approval for each release.

GitHub and self-hosted Git/Gitea support access-token sign-in in the app; encrypted credentials stay local and are shared with CLI. Cloud packages are built from locally tested version tags only. Ordinary pushes run lightweight checks; release publication requires explicit approval and reuses successful build artifacts.

The [manager Skill](../skills/agenthub-manager/SKILL.md) is optional: obtain its complete source folder and copy it manually to a tool only when needed. Installers do not create tool Skills or Rules. Synchronization defaults to preserving all unselected host content; replacement is explicit and limited to selected categories.

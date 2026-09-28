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

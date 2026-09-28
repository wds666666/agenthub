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

The runtime root defaults to `~/.agenthub`. For tests and isolated trials, set `HOME` to a temporary directory and optionally set `AGENTHUB_HOME` explicitly. Do not run synchronization against a real user home until the generated Plan has been reviewed.

See `docs/architecture.md` and `docs/sync-contract.md` before changing storage or adapter behavior.
Ubuntu/WSL setup and the complete verification commands are documented in `docs/development.md`.

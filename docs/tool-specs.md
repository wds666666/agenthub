# Host capability specifications

Checked: 2026-09-29. These are adapter inputs, not permission to scan project scope.

## Portable Agent / Codex

- Skills use `SKILL.md` and the Agent Skills folder convention. AgentHub scans user roots only: `~/.agents/skills` and `~/.codex/skills`.
- Portable plugins use `plugin.json`; Codex-compatible packages may use `.codex-plugin/plugin.json`. Components may include `skills/` and `mcp.json`.
- Codex user MCP configuration is the `mcp_servers` table in `~/.codex/config.toml`; all other TOML keys are preserved. Stdio environment values project to `env`, while remote fixed headers project to the documented `http_headers` field.
- AgentHub projects Canonical Rules to the global `~/.codex/AGENTS.md` file.
- Personal marketplace metadata lives at `~/.agents/plugins/marketplace.json`; common personal plugin sources live below `~/.codex/plugins/<plugin-name>`, installed copies are cached below `~/.codex/plugins/cache/<marketplace>/<plugin>/<version>`, and enablement/configuration is recorded in `~/.codex/config.toml`. These locations form one CLI-managed state machine, not a replaceable plugin folder. AgentHub v0.1 therefore inventories recognized plugin packages but preserves marketplace/cache/config state during Apply and cleanup, reporting a constraint until a transactional marketplace/CLI adapter is implemented. System and organization policy is read-only.

Official references: [Codex plugins](https://developers.openai.com/plugins/build/plugins), [Codex plugin skills](https://developers.openai.com/plugins/build/skills), [Agent Skills](https://agentskills.io/specification).

## Cursor

- User skills are read from `~/.cursor/skills` and the shared `~/.agents/skills` convention.
- Cursor recursively discovers nested Skill folders and also recognizes Claude/Codex compatibility directories. AgentHub therefore scans nested user Skill roots and treats `~/.agents/skills` as an independent shared projection rather than a Cursor alias.
- User MCP is `~/.cursor/mcp.json`, under the `mcpServers` key. Other top-level JSON values are not part of AgentHub's capability domain.
- Plugins use the Agent Plugin format or `.cursor-plugin/plugin.json`; writable local plugins are under `~/.cursor/plugins/local`.
- User Rules do not have a stable file API equivalent to project `.cursor/rules`; AgentHub therefore projects Rules through one generated global local plugin.
- Cloud, team and built-in rules/plugins are reported as host constraints and are never claimed as writable.

Official references: [Cursor Skills](https://cursor.com/docs/skills), [Cursor MCP](https://cursor.com/docs/mcp), [Cursor Plugins](https://cursor.com/docs/plugins), [Cursor Rules](https://cursor.com/docs/rules).

## Claude Code

- User skills live at `~/.claude/skills`; vendor-reserved or synced capability directories are host constraints.
- `~/.claude/skills/synced`, `.trash`, and other vendor-managed/synced locations are preserved and never projected as ordinary local Skills.
- User rules live at `~/.claude/rules`.
- Plugins use `.claude-plugin/plugin.json` and can contain skills, MCP, agents, commands, hooks and related components. Installed state under `~/.claude/plugins` is explicitly managed by `claude plugin`; the official directory guide warns against deleting or rewriting that store. AgentHub v0.1 therefore scans it read-only, preserves it during Apply, and reports Canonical Claude plugin projection as a host constraint until a transactional CLI adapter is available.
- MCP supports `stdio`, HTTP and SSE transports. User scope is managed by Claude's user configuration/CLI; v0.1 can read and deterministically replace the user `mcpServers` collection while preserving unrelated settings.
- Managed, organization, cloud-synced and built-in capabilities are read-only constraints.

Official references: [Claude Code Skills](https://code.claude.com/docs/en/skills), [MCP](https://code.claude.com/docs/en/mcp), [memory and rules](https://code.claude.com/docs/en/memory), [plugin components](https://code.claude.com/docs/en/plugins/components).

## Adapter rule

Only the documented user-global paths above may be inspected. Adapters must not search ancestors, the current directory, Git repositories, arbitrary HOME directories, project `AGENTS.md`, project `CLAUDE.md`, or project `.cursor`, `.codex`, `.claude`, or `.agents` folders.


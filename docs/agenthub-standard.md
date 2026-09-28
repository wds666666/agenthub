# AgentHub Canonical Standard v0.1

## Root

The default root is `~/.agenthub` (override with `AGENTHUB_HOME` for tests). It is mode `0700` and is an independent Git repository.

```text
skills/<id>/SKILL.md
plugins/<id>/agenthub.plugin.json
plugins/<id>/payload/**
rules/<id>/rule.md
rules/<id>/rule.json
mcp/<id>/server.json
agenthub.toml
state/agenthub.db
secrets/master.key
backups/<transaction-id>/**
projections/**
```

Git tracks only `skills/`, `plugins/`, `rules/`, `mcp/`, and `agenthub.toml`. Runtime directories and `*.log` are ignored.

## Versioned IR

Every JSON manifest contains `schemaVersion: 1`.

- `Capability`: `id`, `kind`, `displayName`, `digest`, compatibility metadata and timestamps.
- `McpServer`: `transport` (`stdio`, `http`, `sse`), command/args or URL, headers/env/OAuth, and `SecretRef` values.
- `Rule`: Markdown body in `rule.md` plus `schemaVersion`, immutable lowercase slug `id`, `displayName`, `activation`, `paths`, and target compatibility in `rule.json`. Desktop Markdown import initializes this structure as a draft before the user saves it.
- `Plugin`: `agenthub.plugin.json` describes skills, MCP, rules, agents, commands, hooks, LSP and assets stored below `payload/`.
- `TargetProfile`: target enablement, compatibility, last synchronization summary, and an optional SQLite-only automatic-sync profile containing the explicitly selected Skills, Plugins, MCP IDs and Rules toggle. Automatic-sync configuration is business state and is not part of Canonical Git content.
- `Plan`: deterministic difference from one Canonical snapshot to a target's entire writable capability domain.
- `Transaction`: immutable record of the Plan, Git state, backup, execution, verification and rollback.

IDs are lowercase ASCII slugs. Paths in manifests are normalized relative paths. Absolute paths, `..`, escaping symlinks, unknown component kinds and unrepresentable manifests are rejected. Migrations are deterministic functions from one schema version to the next.

## Full replacement semantics

After initialization, Canonical Desired State is authoritative. A target projection contains every compatible Canonical capability and no other writable user capability. Host-only additions are drift and are deleted on the next Apply. Incompatible items are skipped with an explicit warning; read-only host constraints are outside the writable domain.

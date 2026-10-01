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

Git is the portable content store, not a machine backup. Skill resources and plugin payload dependencies accompany their manifests; host scan paths, enabled targets, automatic-sync profiles, import selection, transaction records, SQLite, credentials, encryption keys, projections, local deletion backups and repository connection/author settings stay on each device. Host paths displayed by discovery are not persisted as portable content. MCP command paths and environment references can still need device-specific setup; synchronization does not make installed executables or credentials portable. Remote upload checks all reachable history for excluded root paths and recognizable JSON secrets, but arbitrary payload content still needs review.

Bulk library deletion first validates every `(kind, id)`, then renames directories into `backups/library-delete-<uuid>/<kind>/<id>` on the same filesystem. A staging failure restores moved directories. Successful archives stay local (not Git-tracked) for recovery of uncommitted content; saving a version records the deletions. Host synchronization runs only after the complete batch.

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

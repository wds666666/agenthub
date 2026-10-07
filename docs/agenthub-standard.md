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

Skill imports retain `SKILL.md`, scripts, references, assets, dependency manifests/lockfiles and nested instructions. They exclude local `.venv`/`venv` directories (and other directories identified by `pyvenv.cfg`), `node_modules`, `.git`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, and generated `.pyc`/`.pyo` files. These exclusions apply to Skills only, never to complete Plugin payloads. Dependency environments are recreated on the destination device from the retained manifests; source installations are untouched. This policy does not delete runtime files from existing libraries or change host backup/Apply semantics.

Bulk library deletion first validates every `(kind, id)`, then renames directories into `backups/library-delete-<uuid>/<kind>/<id>` on the same filesystem. A staging failure restores moved directories. Successful archives stay local (not Git-tracked) for recovery of uncommitted content; saving a version records the deletions. Deleting a batch remains pending and never triggers host delivery.

The generated Git ignore rules anchor `.gitignore`, `state/`, `secrets/`, `backups/`, `projections/` and `runtime/` at the library root. Skill-internal `.gitignore` files and supporting directories with those names remain portable content. Existing generated unanchored root rules migrate without removing custom ignore rules.

## Versioned IR

Every JSON manifest contains `schemaVersion: 1`.

Current MCP JSON uses `display_name` and string-valued `env`/`headers`; Rule and Plugin metadata use `displayName`. The richer IR below describes the intended model; current MCP serialization does not implement a general OAuth/SecretRef resolver. Actual file examples and implemented limits are maintained in the [manager format reference](../skills/agenthub-manager/references/formats.md). `agenthub validate --json` checks structure, schemas, directory/manifest identities and disallowed file types; it does not evaluate instruction semantics or scan every possible secret.

- `Capability`: `id`, `kind`, `displayName`, `digest`, compatibility metadata and timestamps.
- `McpServer`: `transport` (`stdio`, `http`, `sse`), command/args or URL, headers/env/OAuth, and `SecretRef` values.
- `Rule`: Markdown body in `rule.md` plus `schemaVersion`, immutable lowercase slug `id`, `displayName`, `activation`, `paths`, and target compatibility in `rule.json`. Desktop Markdown import initializes this structure as a draft before the user saves it.
- `Plugin`: `agenthub.plugin.json` describes skills, MCP, rules, agents, commands, hooks, LSP and assets stored below `payload/`.
- `TargetProfile`: target enablement, compatibility, last synchronization summary, and an optional SQLite-only automatic-sync profile containing the explicitly selected Skills, Plugins, MCP and Rules IDs plus preserve/replace mode. Automatic-sync configuration is business state and is not part of Canonical Git content.
- `Plan`: deterministic difference from one Canonical snapshot to a target's entire writable capability domain.
- `Transaction`: immutable record of the Plan, Git state, backup, execution, verification and rollback.

IDs are lowercase ASCII slugs. Paths in manifests are normalized relative paths. Absolute paths, `..`, escaping symlinks, unknown component kinds and unrepresentable manifests are rejected. Migrations are deterministic functions from one schema version to the next.

## Selected capability authority

Canonical is the content source for selected capabilities. Preserve mode is the default and keeps unselected tool content. Replace mode removes extras only in explicitly managed categories. Host constraints stay outside either writable scope. Plans snapshot the selected IDs and mode; transaction hashes remain separate from normalized comparison digests.

## Selected capability synchronization (current contract)

Preserve is the default: add/replace only explicitly selected capabilities; leave every unselected host resource intact, including retired selections and library deletions. Replace reconciles only explicitly managed categories to the selected subset. Neither mode expands scope. Rules have individual IDs; creating a rule saves only to Canonical and never distributes it or selects it. Existing selected rules may be projected after an explicitly requested committed save. Old automatic profiles pause for mode/rule-scope review. Package installation supplies Desktop and CLI, with the manager Skill available separately for manual installation.

Discovery uses a shared comparison digest of portable capability content, separate from transaction file digests. MCP compares normalized target-representable configuration, Rules exclude generated wrappers, Plugins compare their complete payload, and Skills retain runtime exclusions. Generated rule containers are identified explicitly. Equal content is blocked from reverse import in the backend as well as the UI; same-name differences remain reviewable. Parsing/identity uncertainty blocks destructive rewriting rather than treating the configuration as empty.

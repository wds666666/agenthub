# Portable file formats (schema 1)

Use the root reported by `doctor`. Resource IDs are lowercase ASCII slugs containing letters, digits, hyphens or underscores, at most 80 characters, excluding Windows reserved names. Keep the directory ID and manifest ID equal; do not rename a resource as an incidental edit.

| Kind | Required content |
| --- | --- |
| Skill | `skills/<id>/SKILL.md`, with its scripts/references/assets retained |
| MCP | `mcp/<id>/server.json` |
| Rule | `rules/<id>/rule.md` and `rule.json` |
| Plugin | `plugins/<id>/agenthub.plugin.json` and complete `payload/` |

Skills retain their existing Markdown and optional YAML frontmatter. For new interoperable skills include `name` and `description`; AgentHub's structural validator checks a nonempty UTF-8 `SKILL.md`, not all downstream skill conventions. Never discard support files during backup or conflict resolution.

MCP uses `display_name` (snake case in the current implementation):

```json
{
  "schemaVersion": 1,
  "id": "example",
  "display_name": "Example",
  "transport": "stdio",
  "command": "example-mcp-server",
  "args": [],
  "url": null,
  "env": {},
  "headers": {}
}
```

`stdio` requires a command; `http`/`sse` require an HTTP(S) URL. Env and headers are string maps. Placeholders such as `${SERVICE_TOKEN}` may be stored, but target substitution is not guaranteed; this is not a universal SecretRef/OAuth resolver. Actual imported env/header strings can still be plaintext. Do not paste them into chat or assume they were encrypted automatically. Installed commands and authentication remain device-specific.

Rule metadata uses camel case and the body is a separate Markdown file:

```json
{
  "schemaVersion": 1,
  "id": "example",
  "displayName": "Example",
  "activation": "always",
  "paths": [],
  "targets": ["cursor", "codex", "claude"]
}
```

`activation` is `always`, `manual` or `paths`; path activation needs path patterns. Rules require at least one supported host target and a nonempty body. Do not add shared `agents` as a Rule target.

Imported Plugin metadata includes `schemaVersion`, `id`, `displayName`, `sourceFormat`, `sourceManifest` and a component-name list. The original recognized manifest and entire directory are preserved under `payload/`. Editing that wrapper does not make an unsupported vendor cache importable or install/register a plugin in another tool. Follow the currently stated plugin limits.

`agenthub.toml` contains `schema_version = 1`. Run `agenthub validate --json` after edits; it checks required files, identities, supported schemas, recognized plugin payloads and disallowed symlinks/special files without executing payloads. Review instructions, credentials and cross-device command portability separately.

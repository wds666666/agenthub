# Explicit synchronization scope

Always inspect `agenthub inventory --json` and the installed CLI help first. Store selection files outside versioned content, such as the library's ignored `runtime/` directory.

```json
{
  "mode": "preserve",
  "skills_managed": true,
  "skills": ["example"],
  "mcp_managed": true,
  "mcp": ["server"],
  "plugins_managed": false,
  "plugins": [],
  "rules_managed": true,
  "rule_ids": ["safety"]
}
```

Use `agenthub plan cursor --selection <file> --json`, review the named changes, then apply the exact Plan with explicit confirmation. Unknown fields and invalid/duplicate IDs are rejected. Skills, MCP, Plugins and Rules are individually selected. `agents` supports Skills only; Codex and Claude official plugin stores remain protected.

`preserve` is the default: add or replace only selected capabilities, leaving all unselected content intact. Removing an ID from the selection or deleting it from the library does not remove its host copy. Use explicit host cleanup for deletion.

`replace` reconciles only managed categories to the selected subset, including deletion of host extras. A managed category with an empty list is cleared only in this mode. Categories with no management flag and no selected IDs are untouched. Neither mode expands scope to other library content.

New Rules are saved only to AgentHub and are not automatically selected. Explicitly selected Rules may participate in automatic sync after user confirmation. Old automatic profiles pause on upgrade for mode and individual-rule review; legacy `rules` and `authoritative` switches cannot authorize a new Plan.

Plan generation is read-only. Apply detects changed library/target state, backs up, writes, verifies and rolls back on failure. Automatic profiles are device-local, not Git content. Their saved mode and selection do not change when the default mode in Settings changes. New library capabilities do not silently enter an existing profile.

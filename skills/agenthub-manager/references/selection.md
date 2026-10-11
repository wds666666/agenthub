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

`replace` reconciles only managed categories to the selected subset, including deletion of host extras. A managed category with an empty list is cleared only in this mode. Categories with no management flag and no selected IDs are untouched. Neither mode expands scope to other categories. Fixed lists remain fixed unless `all_kinds` explicitly opts into a category.

New Rules write AgentHub first; fixed profiles do not select them, while an explicitly opted all-Rules profile does. Explicitly selected Rules may participate in automatic sync after user confirmation. Old automatic profiles pause on upgrade for mode and individual-rule review; legacy `rules` and `authoritative` switches cannot authorize a new Plan.

Plan generation is read-only. Apply detects changed library/target state, backs up, writes, verifies and rolls back on failure. Automatic profiles are device-local, not Git content. Their saved mode and selection do not change when the default mode in Settings changes. New capabilities enter a profile only when their category is explicitly in `all_kinds`.

CLI can enable or replace a device-local scope after review: `agenthub auto-sync enable <target> --plan-id <id> --confirm`. It applies that exact host Plan with backup and verification before saving its selection. This may write tool files; authorizing only a library commit or push does not authorize it. `auto-sync disable <target>` keeps the selection and host content but disables later delivery. `auto-sync run` reads committed HEAD only. Imports and Rule saves trigger pending delivery only for enabled, reviewed profiles with `sync_on_change: true`.

## All current and future capabilities

Opt in explicitly in a reviewed host selection, for example:

```json
{"mode":"preserve","all_kinds":["skill","mcp"],"sync_on_change":true}
```

`all_kinds` uses singular kinds (`skill`, `mcp`, `rule`, `plugin`). It overrides fixed IDs for those categories, includes future additions, and stays symbolic in the saved local profile. Other categories remain untouched. Shared Agents supports only Skills; dynamic Plugins are supported only for Cursor. Preserve with an empty category is a no-op; replace may delete host extras, so always review the Plan.

`sync_on_change` authorizes immediate delivery after imports and Rule edits, including pending content; legacy profiles default to false. Disabling the profile stops it. Tool drift still blocks overwrites, and failed tool delivery does not undo a successful import. `auto-sync run --include-pending` explicitly projects current local content; plain `auto-sync run` stays HEAD-only. A selective version-save projection remains limited to the saved capabilities, even when an enabled scope uses `all_kinds`.

With Desktop running, the same opt-in observes portable library file events with metadata fallback and waits for two matching changed observations before validating and delivering external edits. Initial startup, tool-only edits and deletion-only library changes never trigger it. CLI has no persistent watcher.

# Synchronization contract

## State machine

`Draft → Planned → Confirmed → BackedUp → Applying → Verifying → Applied`.

Any Apply or verification failure enters `RollingBack`, then `RolledBack` or `RollbackFailed`. A Plan never writes host files. Non-interactive Apply requires the exact Plan ID plus an explicit confirmation flag.

The desktop offers a reviewed one-off path and an automatic-sync path. Enabling automatic sync first reconciles the selected target and scope through the same deterministic Plan, backup, Apply and verification pipeline; only after success is the profile stored in SQLite. Immediate delivery of pending mutations requires an explicitly reviewed `sync_on_change` profile. Plain explicit runs read HEAD; selected saves project only committed capabilities when requested. Desktop also observes portable library edits for immediate profiles, without watching tool directories.

If an automatic run has no steps, it returns `changed=false` without persisting the transient Plan, creating a backup, or recording a transaction. If any host path changes, the transaction is always persisted with `mode=auto_sync` so rollback remains possible. Targets run independently: one failure is reported without preventing the other enabled targets or reverting the already-saved Canonical mutation. History may visually group adjacent successful automatic-sync records, but storage never coalesces or discards them.

## Deterministic Plan

A Plan records Canonical digest, Git HEAD, dirty state, target, selected Skills/Plugins/MCP IDs, the individual Rules, expected projection digest, warnings and ordered file steps. Steps classify create, update, replace, delete, skip and constraint. Secrets are redacted before persistence, logs or JSON output. Apply re-generates the same selected projection and blocks if the selection or any other Plan input has drifted.

Plan presentation must expose a capability-level change list before raw file details. For Skills, Plugins and individually projected Rules this names the exact Canonical or host capability ID and whether it will be created, replaced or deleted, with the affected file count. Combined host documents such as an MCP collection may be labeled as a writable-domain replacement when the adapter cannot safely attribute a file diff to one server. Raw paths remain a secondary expandable diagnostic view.

## Explicit sync scope

Target Sync begins with a scope selector. Skills, Plugins and MCP servers are selected by Canonical ID; Rules are selected by individual Canonical ID even when an adapter renders them as a combined file or plugin. Preserve mode updates only selected capabilities and retains all host extras. Replace mode reconciles the selected category to the selected subset and previews host-extra deletions. An unmanaged category is not touched. An empty managed category retains content in Preserve and explicitly clears writable content in Replace; protected content remains. The selection snapshot is persisted in the Plan and is part of the Apply contract.

Each list domain also carries an explicit managed flag. This separates "do not touch" from "manage with an empty desired set". In Replace mode the latter produces deletion steps and is how a user intentionally clears `~/.agents/skills` or another writable host domain. Legacy selections remain managed whenever they contain selected IDs.

The automatic-sync profile uses the same visible selector. The UI shows the saved target scope and requires the user to review named Skills, Plugins, MCP servers and the individual Rules when enabling or changing it; automatic sync is never represented as an opaque all-capabilities switch.

CLI `plan <target> --selection <json-file> --json` accepts the same selection, rejects unknown fields and missing/duplicate resource IDs, and persists it in the Plan. Omitting `--selection` retains full compatible scope. CLI initialization cannot overwrite an initialized library; whole-scan first import uses atomic staging and skips nonimportable discoveries.

## Target domains

| Target | Skills | MCP | Plugins | Rules |
| --- | --- | --- | --- | --- |
| Shared Agents | merge selected or explicitly replace `~/.agents/skills` | unsupported | unsupported | unsupported |
| Cursor | merge selected or replace `~/.cursor/skills` | merge selected or replace `mcpServers`, preserve other JSON | merge selected or replace writable local plugins | selected files in generated Rules container |
| Codex | merge selected or replace `~/.codex/skills` | merge selected or replace `mcp_servers`, preserve other TOML | preserve CLI/marketplace-managed state and report v0.1 constraint | selected identifiable blocks in `AGENTS.md` |
| Claude | merge selected or replace writable Skills, except reserved dirs | merge selected or replace user-scope servers | preserve CLI-managed plugin store and report v0.1 constraint | merge selected or replace `~/.claude/rules` |

Before writing, the executor resolves all paths and records type, mode, hash and symlink target. It then creates a complete transaction backup, writes the projection, reads the target again and compares it with Expected State. Verification failure automatically restores the backup. Cleanup is manual and warns that rollback ability will be lost.

On Unix, nested host symlinks are retained in local recovery copies without following their targets, so an installed `.venv` does not prevent sync or cleanup. Rollback compares link metadata as well as regular files and empty directories. Backup roots remain ordinary paths; Canonical import and projection still reject retained symlinks. Platforms without this recovery implementation refuse symlink-containing backups before any host write.

A verified Apply atomically records the applied transaction and marks that target enabled with its last-sync timestamp. Dashboard target state must never diverge from a successful Apply. Existing databases migrate historical applied transactions only when no explicit target row exists, preserving a later explicit disable.

## Manual rollback

Manual rollback is a new audited transaction, not a mutation of the historical Apply record. Given an Apply transaction, AgentHub restores the writable target domains from that transaction's pre-Apply backup. Before restoration it creates a fresh backup of the target's current writable domains so the rollback itself can be reversed. The restored paths are re-read and compared with the selected backup; failure restores the pre-rollback backup automatically.

Rollback never changes Canonical files or Git history. The restored host can therefore intentionally drift from Canonical, and the next Plan must show that drift before another Apply. The desktop confirmation names the target and full transaction ID and labels the action as restoring the state before that Apply.

`scan` is an import discovery operation only before initialization. Afterwards it is a read-only drift audit; it never merges or imports host changes.

The desktop exposes a separate **Scan and import** action after initialization. Discovery is read-only; exact normalized portable-content duplicates already in Canonical are disabled, duplicates across hosts are grouped, and only explicit confirmation imports selected candidates. Import remains an unsaved Canonical mutation, never commits or pushes, and projects only through previously reviewed immediate profiles. The legacy reverse-import delivery flag is retained only for compatibility and no longer executes.

## Enforcement policy

The default mode is Preserve: only explicitly selected capabilities are added/replaced, and every unselected resource remains. Replace reconciles only explicitly managed categories; it never expands selection into other categories. New library items enter only an explicitly opted all-category scope. Changing the default in Settings does not change saved profiles. Legacy profiles are paused for review. Protected vendor stores, organization policy and undocumented plugin storage remain excluded.

Post-initialization reverse import is a separate user action. Exact normalized portable-content duplicates are skipped and all selected items are staged and validated before Canonical activation. By default it does not write back to any host. CLI discovery and import use the same backend preflight and require a reviewed import Plan. Later publication and delivery are separate explicit actions.

## Host inventory and cleanup

The Host Resources page is diagnostic and operational, not another source of truth. It scans only the documented user-global allowlist and labels every item as `canonical_match`, `host_only`, or `constraint`. A Canonical match means the host item corresponds to current Canonical content; it is not an ownership marker. Deleting such an item creates drift and a later authorized host Plan may recreate it; automatic delivery blocks absent or edited baseline files before writing.

Cleanup is always a separate explicit transaction. The user selects exact resources, reviews their target, kind, name and path, and confirms deletion. Before mutation AgentHub re-scans the target and rejects stale or non-deletable IDs, then backs up all unique affected paths. Standalone Skills, writable Rules and recognized Cursor local plugins may be deleted directly. MCP cleanup removes only the selected server entry and preserves every unrelated key in the host configuration. Claude Code plugin storage and Codex marketplace/cache/plugin state are protected constraints in v0.1 and never expose a raw delete action.

The desktop may offer a one-click pre-selection of every `host_only` deletable resource for the current target (“not imported into AgentHub”). It is only a selection shortcut: the same review list, explicit acknowledgement and cleanup transaction apply, and Canonical matches or constraints are never included.

Cleanup never changes Canonical. Failure restores the pre-cleanup backup automatically. A successful cleanup reports the backup location and leaves any resulting drift visible; it does not silently trigger synchronization.

## Remote version synchronization
A user explicitly connects a dedicated AgentHub repository and branch. Each version save automatically synchronizes when connected; a manual sync retries failed uploads and receives changes on other devices without polling. Fetch and merge preserve both histories; conflicts abort the merge and preserve the local commit. Remote trees must contain only Canonical domains, agenthub.toml and an optional .gitignore, with no symlinks or submodules. Validate merged Canonical before committing. Push never forces; a concurrent remote update triggers one bounded fetch/merge retry. Pending changes overlapping incoming capabilities block publication; disjoint changes are retained. Download-only reception still requires a clean tree. Connection tests authenticate read access before storing configuration; write access is verified at first push.
Host Resources also offers a quick-clean action for every adapter-confirmed deletable resource on the current target, including Canonical matches. It uses the same exact review, acknowledgement, rescan, backup and rollback flow. Protected vendor stores remain visible and excluded.

## Repository authentication and state

First-run restoration reads only. A supplied branch must exist; an omitted branch prefers `agenthub` when present, otherwise the remote HEAD branch. Empty repositories and non-AgentHub/unsupported layouts fail without changing the local library or remote. The restored local HEAD equals the remote branch commit, and subsequent version saves use the same remote/branch and normal fetch/merge/push rules. Connection after restoration is read-verified, not write-verified. Each device authorizes separately and starts with no enabled host or automatic-sync profiles.

When the local version is an ancestor of the fetched version, validate the incoming library and history in a private local checkout before fast-forwarding. This receives versions without creating a synthetic local merge commit or requiring a new device to configure an author first. Divergent versions still use the reviewed conflict-preserving merge path. Sync remains bidirectional and needs write permission; author identity is required when saving local changes.

GitHub and self-hosted Git/Gitea support HTTPS username/access-token sign-in. Login verifies reading without uploading; an actual push establishes upload permission. UI states distinguish configured but unverified, last read verification, last successful sync, failed authentication and incomplete sync. A stored URL alone is not successful authentication. Only an explicit sign-in-and-retry action retries the failed upload after login, and local commits remain saved. Forgetting credentials preserves repository configuration, library content and versions.

## Skill change reminders

The initialized desktop checks existing library Skill IDs in Shared Agents, Cursor, Codex and Claude Code automatically while visible, with a two-minute minimum interval. Only ordinary first-level Skill directories are candidates. Compare the complete portable content, including supporting files, using the existing runtime exclusions. Metadata fingerprints cache per-directory digests; unchanged content is rehashed after thirty minutes. Filesystem errors produce a partial-check warning, never a false clean result. The check never imports, writes hosts, commits or accesses the network.

A differing same-ID resource shows a red reminder in library navigation and on the corresponding Skill. Content already present anywhere in the library is excluded, so explicitly importing a changed copy clears its reminder after recheck. Opening a reminder does not mark it resolved. The review dialog shows source and path, leaves selection empty, and uses the existing atomic reverse-import command with stale-result validation. Import retains the original library item and adds the changed content under a unique ID; it is not an in-place update or merge. Existing optional sync-after-import policy remains applicable.

## Selected capability synchronization (current contract)

Preserve is the default: add/replace selected capabilities and retain unselected host content. Replace reconciles only managed categories and previews deletion of extras. Fixed lists do not expand; explicitly opted all-category lists include future additions. Rules retain individual IDs, including in all-mode. Creating a Rule writes Canonical first; only an enabled, reviewed immediate profile selecting it (or all Rules) delivers it. Legacy profiles keep fixed scope and committed-only delivery. Packages contain Desktop and CLI; Desktop maintains the bundled manager Skill in the library; host delivery follows opted scopes.

Discovery uses a shared comparison digest of portable capability content, separate from transaction file digests. MCP compares normalized target-representable configuration, Rules exclude generated wrappers, Plugins compare their complete payload, and Skills retain runtime exclusions. Generated rule containers are identified explicitly. Equal content is blocked from reverse import in the backend as well as the UI; same-name differences remain reviewable. Parsing/identity uncertainty blocks destructive rewriting rather than treating the configuration as empty.

Remote commands validate updates in Canonical without synchronous host projection. Saving a version projects selected committed capabilities only when requested. An explicitly enabled Desktop immediate watcher may subsequently observe received library changes. `git receive` does not upload.

## Version recovery actions

Versions shows unsaved changes grouped by capability (Skills, MCP, Rules, Plugins), with file details separately available. Refresh is read-only. Discard changes restores the last local saved content; Use remote content previews a validated dedicated-repository snapshot and replaces library content only. The remote action does not reset local history, force-push, upload or synchronously deliver to hosts: the replacement remains a working-tree change for explicit saving. Both actions require an immutable preview, exact confirmation, private recovery backup including the index, stale-input checks, verification and recovery on failure/interruption. Device state, credentials, Git history and host paths are excluded from replacement. A missing local saved version blocks discard; remote authentication/layout errors do not change local content.

Recovery previews use Git clean-filter comparison (including Windows CRLF and repository attributes) to select changed canonical paths. Raw byte digests remain unchanged for backup integrity and concurrent-edit detection. Ignored/untracked canonical files that replacement will remove are still listed.

## Saving, publishing and projecting

Saving is local by default. Optional push and host projection are independent, explicit flags in the reviewed version Plan. Host projection for a selective save is restricted to the capabilities in that save and reads the committed snapshot, never pending working files. This projection uses preserve mode even if a saved profile uses replace, so unrelated resources cannot be removed by a selective save. Explicit full target projection remains a separate reviewed action. Automatic host writes stop when affected existing files drift from the last verified projection; first-time overwrites require a reviewed host Plan. Remote-only publication never directly writes tools; the separately opted immediate Desktop watcher can react to received content changes. Dirty pending capabilities overlapping incoming remote changes block publication, preserving local content/index; disjoint pending capabilities remain untouched.

## Dynamic category scope and immediate delivery

Device-local `SyncSelection.all_kinds` explicitly opts categories into **all current and future library capabilities**; absent/empty means fixed ID lists as before. It never includes another category or writable vendor plugin stores. Plans retain this symbolic scope and bind the entire library digest, so adding a resource invalidates an older Plan. In preserve mode an empty all-category scope writes nothing; replace mode still previews category deletions.

`sync_on_change` is a separate, default-false local scope setting. After reviewed enablement, successful imports and Rule saves deliver according to local immediate/scheduled settings the current local library (including unsaved content) to enabled, reviewed profiles opting into this setting. Changes are backed up and verified, and host drift stops automatic overwrites. Import success and tool-delivery errors are reported separately. Installation, ordinary startup/discovery without a bundle update, and deletion-only changes do not write hosts; an actual bundled manager update follows enabled pending-content scopes; remote/recovery operations themselves do not project. Existing profiles keep their earlier trigger until explicitly reconfirmed. `auto-sync run` remains committed-HEAD-only; `--include-pending` explicitly uses current content. Selective version saves still project only saved changes, even with all-category profiles.

The tool-sync picker offers “All capabilities in this category (including future additions)” alongside fixed individual selection. All-mode rows reflect the entire category, independent of search. Turning it off restores the prior fixed choices. The confirmation shows dynamic categories and whether pending content is delivered on import/edit.

While Desktop is running, external edits inside the library are observed with file-event hints, metadata fallback and two-observation debounce. Only explicit `sync_on_change` profiles participate. Startup establishes a baseline without delivery; tool-only changes and deletion-only library changes do not trigger projection. Edited content must validate first. Background failures are logged with redaction; a subsequent reviewed Plan is required to resolve host drift. Remote operations do not synchronously project, but later library content changes may be observed by this explicitly enabled watcher. CLI has no persistent watcher.

## Desktop background operation (0.1.5 local update)

Desktop supplies a bundled manager Skill to Canonical on first/upgrade launch and explicit discovery. Matching bundle content is unchanged; recognized older content is backed up and updated; unknown or edited same-ID content is reported without overwriting. Installation never writes host directories. Bundle maintenance does not commit or push Git, and projects only through already enabled reviewed pending-content scopes.

Local background settings distinguish host reminder checks (default 120 seconds) from pending-content delivery (immediate by default, or 1/2/5/10/30 minutes). Pause suppresses automatic delivery. A single coordinator merges edits and serializes delivery; manual refresh explicitly checks hosts and delivers enabled scopes. Startup establishes a library baseline and never fills missing host copies. Host edits remain review-only. Close offers exit or tray residency; unavailable tray support keeps the window accessible. Background status and attributed capability changes are machine-local, with unknown attribution on older history.

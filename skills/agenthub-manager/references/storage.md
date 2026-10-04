# Portable storage and remote synchronization

Git shares only:

| Content | Canonical path |
| --- | --- |
| Skill instructions and supporting resources | `skills/<id>/` |
| Plugin manifest and complete validated payload | `plugins/<id>/agenthub.plugin.json`, `plugins/<id>/payload/` |
| Rule text and activation metadata | `rules/<id>/rule.md`, `rules/<id>/rule.json` |
| MCP declaration | `mcp/<id>/server.json` |
| Library schema configuration | `agenthub.toml` |

SQLite, enabled targets, automatic profiles, transactions, host backups, library deletion archives, projections, logs and keys stay local. Remote URL/branch and author identity use repository-local `.git/config`. Do not copy host installation/cache paths or account/session configuration into Git. Skill/plugin dependencies are content; dependencies already inside an imported payload can travel with it and must be reviewed. Installed executables, absolute MCP command paths and credentials may require setup on each device.

Current MCP env/headers can contain strings. Prefer environment placeholders such as `${SERVICE_TOKEN}`. Do not claim that all imported credentials are automatically encrypted or that every target resolves every placeholder. Remote upload inspects all reachable paths and recognizable JSON credentials, including old versions; deleting a token only from the latest version does not remove it from history. Arbitrary Markdown/binary secrets require manual review.

## Remote workflow

- `agenthub git remote-status`
- `agenthub git connect <https-or-ssh-url> --branch <branch>`
- `agenthub git sync`
- `agenthub git disconnect`

Desktop **Versions** presents connection and retry state. Connect tests read access. Author name/email identify a commit; they do not sign into a remote provider. Use the desktop sign-in dialog for GitHub or self-hosted Git/Gitea; encrypted local tokens are shared with CLI and never versioned. Existing system Git credentials and SSH remain available. Interactive prompts and Windows console windows stay disabled. Read verification does not prove push permission. A configured URL without verification is not an authenticated connection.

A saved version commits locally, fetches and merges remote changes, then pushes. Sync requires a clean library working tree and never force-pushes. A concurrent remote update gets one additional reconciliation attempt. Conflicts abort the merge and preserve the local version for manual resolution; do not choose one device's content silently. Local commit success remains success even if remote upload fails: report the remote error and retry with Sync remote after correction.

Read [conflict resolution](conflicts.md) for an Agent-assisted merge using exact fetched versions, file validation and the normal guarded sync. Read [backup migration](migration.md) before adopting an existing backup repository; unrelated content in old history may block it. The skill does not handle secret entry, automatic timers or semantic conflict decisions. Users enter access tokens directly in the desktop; conflict decisions still require the user’s intended content.

No polling or automatic host writes occur on remote reception. On another device, initialize/save its local library, connect the same dedicated repository/branch, synchronize, then review a host Plan before applying. Resource content and Git history are portable; each device chooses its own hosts and scope.

Bulk library deletion archives uncommitted content at `backups/library-delete-<uuid>/<kind>/<id>`. This is a local file recovery copy, not a host transaction rollback entry. Committed content can also be recovered through Git; use a reviewed restore into new working-tree changes, never `reset --hard` as routine recovery.

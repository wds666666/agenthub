---
name: agenthub-manager
description: Help an agent inspect and safely manage AgentHub's user-global Skills, MCP, Plugins, and Rules through inventory, Plan review, Git history, sync, and rollback. Use when a user asks to manage AgentHub or synchronize its Canonical source to Cursor, Codex, or Claude Code; do not use it for project-local agent configuration.
---

# AgentHub Manager

Use AgentHub as the user's single configuration entry point for global agent capabilities. The Canonical source is `~/.agenthub`; it is separate from the current project and from project-level `.agents`, `.cursor`, `.codex`, or `.claude` folders.

## Operating boundaries

- Never scan or edit project directories, repositories, `AGENTS.md`, or `CLAUDE.md` as part of AgentHub management.
- Do not edit host Skills, MCP, Plugins, or Rules directly. Make changes in AgentHub Canonical files or through its CLI/Desktop flow.
- Treat a Plan as read-only. Review its target, selected scope, create/update/delete steps, warnings, and Canonical/Git snapshot before any Apply.
- Sync is a full replacement of the selected writable capability domains. Unselected scope is excluded from that Plan; Rules have an explicit on/off switch. Models, themes, permissions, accounts, and vendor/organization-managed data are outside the writable scope.
- Ask for confirmation immediately before a one-off `sync --confirm`, enabling or changing an automatic-sync profile, or any rollback. Never invent confirmation from an earlier message. Once the user enables a specific automatic profile, later AgentHub mutations may use that stored authorization without asking again; expanding its target or scope requires fresh confirmation.
- Secrets must remain references or encrypted AgentHub secrets. Do not print secret values in chat, logs, diffs, plans, or generated files.

## Choose the least surprising workflow

1. Check state with `agenthub doctor --json` and `agenthub inventory --json`.
2. If AgentHub is not initialized, ask whether to use the Desktop initialization selector, import all user-global discoveries, or create an empty source. Do not silently import.
3. For content changes, edit the Canonical resource, validate it, then show Git status/diff. Offer a user-authored Git commit; never auto-commit.
4. For ongoing synchronization, use the Desktop Target Sync selector to choose the target and scope, preview a Plan, then let the user enable automatic sync. AgentHub immediately reconciles once and only stores the profile after success. Later mutations made through AgentHub automatically run that profile; zero-change runs create no transaction.
5. For a CLI-only full-scope sync, create a Plan, capture its exact ID, review JSON, then require the user to explicitly approve that same ID before `agenthub sync <target> --plan-id <id> --confirm`.
6. If an external agent edits Canonical files directly rather than through an AgentHub mutation command, run `agenthub auto-sync run` after validation. This explicitly delivers the change to already-authorized profiles; it does not discover or broaden targets or scopes.
7. For undo, use transaction history to identify the exact target and transaction. Explain that rollback restores the host state captured immediately before that transaction, creates a new recovery backup/transaction, and does not change Canonical. After rollback, expect drift until the next Plan or automatic run.

## Useful commands

```text
agenthub doctor --json
agenthub inventory --json
agenthub target enable <cursor|codex|claude>
agenthub plan <cursor|codex|claude> --json
agenthub sync <cursor|codex|claude> --plan-id <exact-plan-id> --confirm
agenthub auto-sync status
agenthub auto-sync run
agenthub history --json
agenthub rollback <exact-transaction-id>
agenthub git status
agenthub git diff
agenthub git commit --message "<user-approved message>"
```

Use `agenthub scan` after initialization only as a drift audit. It does not import or merge host changes. Read [references/selection.md](references/selection.md) when the user asks which capabilities are included in a sync or how rollback affects scope.

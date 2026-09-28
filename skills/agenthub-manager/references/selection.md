# AgentHub scope and recovery reference

## Sync scope

The Desktop Target Sync page starts with an explicit scope selector:

- Skills, Plugins, and MCP servers are selected by Canonical ID.
- Rules are selected as one target-wide toggle because adapters may render Rules as a combined global file or plugin.
- “Select all” means all current Canonical inventory items, not arbitrary host resources.
- A generated Plan persists the selection snapshot. Changing the inventory after Plan creation makes Apply fail with “generate a new plan”.
- An automatic-sync profile persists the target and the same selection shape in SQLite. Enabling or changing it requires user confirmation and an initial successful reconciliation. Later AgentHub mutations reuse that stored scope without silently adding newly created capabilities.

Selected Skills/Plugins/MCP are projected into the target writable domain. Host extras in an included domain are shown as deletions and are covered by the transaction backup. A domain with no selected resources is not touched. Rules disabled means the target Rules domain is not touched.

Automatic sync is triggered by AgentHub mutation commands rather than a background watcher. A direct Canonical filesystem edit by an external agent must be followed by `agenthub auto-sync run`; this runs only profiles the user already enabled. A no-op run creates no Plan, backup, or transaction, while every real host write retains a separately rollback-capable transaction.

## Review checklist

Before applying, verify:

1. The target is the intended user-global tool.
2. The scope counts match the user's request.
3. Deletes are expected, especially host-only resources.
4. Warnings do not require secret materialization or acknowledge a compatibility loss.
5. The Plan ID and Canonical digest are the ones being approved.

## Rollback

`agenthub rollback <transaction-id>` restores the writable target domains from that transaction's pre-Apply backup. It first backs up the target's current state, validates the restored files, and records a new transaction so the rollback itself can be undone. It does not restore Canonical Git content, disable a target, or remove the historical record. The next sync will reconcile the target back to the current Canonical state.

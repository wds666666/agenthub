# AgentHub repository contract

All changes must preserve the contracts in `docs/agenthub-standard.md`, `docs/architecture.md`, `docs/sync-contract.md`, `docs/security.md`, `DESIGN.md`, and `UX-CONTRACT.md`.

- AgentHub manages user-level global Skills, MCP, Plugins, and Rules only. Never scan a working directory or project configuration.
- Canonical files are the content source of truth; SQLite is business state; backups are host transaction recovery.
- Plan is read-only. Apply must back up, atomically project where possible, re-read, verify, and automatically roll back on failure.
- Target adapters replace their writable capability domains while preserving unrelated host settings and read-only/vendor-managed resources.
- Changes to architecture, schemas, adapter boundaries, security, or UX contracts require the relevant document to be updated first.
- Tests that inspect host layouts must use an explicit temporary home directory. Never use the developer's real home.
- Product copy uses i18n keys. Chinese is the default locale and English must remain structurally complete.


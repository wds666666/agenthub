# AgentHub repository contract

All changes must preserve the contracts in `docs/agenthub-standard.md`, `docs/architecture.md`, `docs/sync-contract.md`, `docs/security.md`, `DESIGN.md`, and `UX-CONTRACT.md`.

- AgentHub manages user-level global Skills, MCP, Plugins, and Rules only. Never scan a working directory or project configuration.
- Canonical files are the content source of truth; SQLite is business state; backups are host transaction recovery.
- Plan is read-only. Apply must back up, atomically project where possible, re-read, verify, and automatically roll back on failure.
- Target adapters replace their writable capability domains while preserving unrelated host settings and read-only/vendor-managed resources.
- Changes to architecture, schemas, adapter boundaries, security, or UX contracts require the relevant document to be updated first.
- Tests that inspect host layouts must use an explicit temporary home directory. Never use the developer's real home.
- Product copy uses i18n keys. Chinese is the default locale and English must remain structurally complete.
- Build and publication policy: without the user's explicit approval for the specific release, never create/publish/update a GitHub Release, push a release tag, or dispatch a workflow with release publication enabled. Requests for builds, fixes or download links authorize Actions artifacts only. Return the run and temporary artifact download links, with expiry/login requirements. The package workflow defaults to build-only; release publication requires `publish_release=true` and explicit user approval. Do not treat a prior release approval as permission for later releases.


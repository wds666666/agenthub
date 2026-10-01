# Security

- `~/.agenthub` is `0700`. Database, master key, backup manifests and sensitive generated files are `0600`.
- A 256-bit `master.key` is generated from the OS CSPRNG. MCP secrets are encrypted in SQLite with XChaCha20-Poly1305, a random 192-bit nonce, versioned algorithm metadata and AAD binding the secret name and schema version.
- Loss of the key never causes ciphertext deletion or replacement. Doctor reports the state as unrecoverable.
- Logs, Plan payloads, errors, Diff and JSON output pass through structural redaction for tokens, passwords, authorization headers and secret values.
- Desktop logs are written to the OS application log directory, never into Canonical Git history. Windows resolves this to `%LOCALAPPDATA%/dev.agenthub.desktop/logs`. Logs are size-bounded, contain operation names and sanitized identifiers rather than secret payloads, and may be copied for diagnostics. Release GUI builds do not expose a console.
- Projections prefer OAuth, environment variables, env files or `SecretRef`. A target that requires plaintext produces a separate blocking warning and writes mode `0600` only after confirmation.
- Plugin imports reject absolute paths, traversal, escaping symlinks, device/special files, unknown components and manifests that cannot be represented losslessly.
- Scanner roots are a fixed allowlist derived from an explicit user home. The current directory and its ancestors are never inputs.
- Initialization treats scan results as untrusted input. Empty/non-UTF-8/oversized Rules are non-importable, every selected item is re-scanned before import, and the complete staged Canonical inventory must validate before it replaces the empty destination.

- Remote addresses accept HTTPS or SSH, reject embedded passwords/tokens, query strings and option-like values. Authentication uses installed Git credential helpers or an existing SSH agent; AgentHub stores no remote credentials. Network Git disables interactive helpers, askpass, terminal prompts and SSH prompts; Windows child processes retain CREATE_NO_WINDOW. Network operations have a bounded timeout. Only explicitly initiated connection/sync/save operations access the network.
- Before any remote write, inspect all reachable versioned paths, including history, for runtime directories and recognizable plaintext credentials in MCP/JSON configuration (sensitive environment names, authorization headers, URLs and credential arguments). Block upload with a path-only error; never include secret values. Environment placeholders are allowed. This supplements directory exclusions; it cannot detect secrets embedded arbitrarily in Markdown or binary payloads. Users must review their versioned library before sharing it.

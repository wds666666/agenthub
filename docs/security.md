# Security

- `~/.agenthub` is `0700`. Database, master key, backup manifests and sensitive generated files are `0600`.
- A 256-bit `master.key` is generated from the OS CSPRNG. MCP secrets are encrypted in SQLite with XChaCha20-Poly1305, a random 192-bit nonce, versioned algorithm metadata and AAD binding the secret name and schema version.
- Loss of the key never causes ciphertext deletion or replacement. Doctor reports the state as unrecoverable.
- Logs, Plan payloads, errors, Diff and JSON output pass through structural redaction for tokens, passwords, authorization headers and secret values.
- Projections prefer OAuth, environment variables, env files or `SecretRef`. A target that requires plaintext produces a separate blocking warning and writes mode `0600` only after confirmation.
- Plugin imports reject absolute paths, traversal, escaping symlinks, device/special files, unknown components and manifests that cannot be represented losslessly.
- Scanner roots are a fixed allowlist derived from an explicit user home. The current directory and its ancestors are never inputs.


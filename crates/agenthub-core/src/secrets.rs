use crate::{
    paths::{set_private_file, AgentHubPaths},
    storage::Store,
};
use anyhow::{Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand::{rngs::OsRng, RngCore};
use std::fs;

const ALGORITHM: &str = "xchacha20poly1305-v1";
pub fn ensure_key(paths: &AgentHubPaths) -> Result<[u8; 32]> {
    if paths.master_key.exists() {
        let b = fs::read(&paths.master_key)?;
        return b
            .try_into()
            .map_err(|_| anyhow::anyhow!("master key has invalid length"));
    }
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    fs::write(&paths.master_key, key)?;
    set_private_file(&paths.master_key)?;
    Ok(key)
}
pub fn set(store: &Store, paths: &AgentHubPaths, name: &str, value: &[u8]) -> Result<()> {
    anyhow::ensure!(!name.trim().is_empty(), "secret name is required");
    let key = ensure_key(paths)?;
    let cipher = XChaCha20Poly1305::new((&key).into());
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut nonce);
    let aad = format!("agenthub-secret:v1:{name}");
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: value,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("secret encryption failed"))?;
    store.put_secret(name, ALGORITHM, &nonce, &ciphertext)
}
pub fn get(store: &Store, paths: &AgentHubPaths, name: &str) -> Result<Option<Vec<u8>>> {
    let Some((algorithm, nonce, ciphertext)) = store.secret(name)? else {
        return Ok(None);
    };
    anyhow::ensure!(algorithm == ALGORITHM, "unsupported secret algorithm");
    let key = fs::read(&paths.master_key)
        .context("master key missing; encrypted secrets are unrecoverable")?;
    let cipher = XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|_| anyhow::anyhow!("invalid master key"))?;
    let aad = format!("agenthub-secret:v1:{name}");
    let plain = cipher
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("secret authentication failed"))?;
    Ok(Some(plain))
}
pub fn redact(value: &str) -> String {
    let keys = [
        "token",
        "secret",
        "password",
        "authorization",
        "api_key",
        "api-key",
    ];
    let mut out = value.to_string();
    for key in keys {
        let pattern = format!("\"{key}\"");
        if let Some(start) = out.to_ascii_lowercase().find(&pattern) {
            if let Some(colon) = out[start..].find(':') {
                let from = start + colon + 1;
                if let Some(end) = out[from..].find([',', '}', '\n']) {
                    out.replace_range(from..from + end, " \"[REDACTED]\"");
                }
            }
        }
    }
    out
}

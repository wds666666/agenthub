use crate::{paths::AgentHubPaths, AgentHub};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

/// Move the complete root aside, including credentials, before rebuilding an empty library.
/// Caller must close all database handles and serialize other operations.
pub fn reset(paths: &AgentHubPaths, confirmation: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        confirmation == "AGENTHUB",
        "type AGENTHUB to reset the library"
    );
    anyhow::ensure!(
        !fs::symlink_metadata(&paths.root)?.file_type().is_symlink(),
        "refusing to reset a symlink root"
    );
    let parent = paths.root.parent().context("AgentHub root has no parent")?;
    anyhow::ensure!(
        !paths.user_home.starts_with(&paths.root) && paths.root.file_name().is_some(),
        "unsafe AgentHub root"
    );
    let backup = parent.join(format!(".agenthub-reset-{}", uuid::Uuid::new_v4()));
    fs::rename(&paths.root, &backup)?;
    if let Err(error) = AgentHub::open(paths.clone()) {
        if paths.root.exists() {
            fs::remove_dir_all(&paths.root)?;
        }
        fs::rename(&backup, &paths.root).context("restore reset recovery copy")?;
        return Err(error);
    }
    Ok(backup)
}

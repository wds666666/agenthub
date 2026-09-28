use crate::models::{GitIdentity, GitSnapshot};
use anyhow::{Context, Result};
use std::{path::Path, process::Command};

fn run(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .with_context(|| "run git")?;
    anyhow::ensure!(
        out.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
pub fn ensure_repo(root: &Path) -> Result<()> {
    if !root.join(".git").exists() {
        run(root, &["init"])?;
    }
    Ok(())
}
pub fn snapshot(root: &Path) -> Result<GitSnapshot> {
    ensure_repo(root)?;
    let head = run(root, &["rev-parse", "--verify", "HEAD"]).ok();
    let dirty = !run(root, &["status", "--porcelain"])?.is_empty();
    Ok(GitSnapshot { head, dirty })
}
pub fn status(root: &Path) -> Result<String> {
    ensure_repo(root)?;
    run(root, &["status", "--short", "--branch"])
}
pub fn diff(root: &Path) -> Result<String> {
    ensure_repo(root)?;
    let tracked = run(root, &["diff", "--no-ext-diff"])?;
    let untracked = run(root, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(format!("{tracked}\n{untracked}").trim().to_string())
}
pub fn log(root: &Path) -> Result<String> {
    ensure_repo(root)?;
    if run(root, &["rev-parse", "--verify", "HEAD"]).is_err() {
        return Ok(String::new());
    }
    run(
        root,
        &["log", "--date=iso", "--pretty=format:%h%x09%ad%x09%s"],
    )
}
pub fn identity(root: &Path) -> Result<GitIdentity> {
    ensure_repo(root)?;
    Ok(GitIdentity {
        name: run(root, &["config", "--get", "user.name"])
            .ok()
            .filter(|value| !value.is_empty()),
        email: run(root, &["config", "--get", "user.email"])
            .ok()
            .filter(|value| !value.is_empty()),
    })
}
pub fn commit(
    root: &Path,
    message: &str,
    name: Option<&str>,
    email: Option<&str>,
) -> Result<String> {
    anyhow::ensure!(!message.trim().is_empty(), "commit message is required");
    ensure_repo(root)?;
    if let Some(v) = name {
        anyhow::ensure!(!v.trim().is_empty(), "Git user name is required");
        run(root, &["config", "user.name", v])?;
    }
    if let Some(v) = email {
        anyhow::ensure!(
            v.contains('@') && !v.chars().any(char::is_whitespace),
            "valid Git email is required"
        );
        run(root, &["config", "user.email", v])?;
    }
    let configured = identity(root)?;
    anyhow::ensure!(
        configured.name.is_some() && configured.email.is_some(),
        "Git identity is required"
    );
    run(
        root,
        &["add", "skills", "plugins", "rules", "mcp", "agenthub.toml"],
    )?;
    anyhow::ensure!(
        !run(root, &["diff", "--cached", "--name-only"])?.is_empty(),
        "there are no Canonical changes to commit"
    );
    run(root, &["commit", "-m", message])
}
pub fn restore(root: &Path, commit: &str, capability: Option<&str>) -> Result<()> {
    let snap = snapshot(root)?;
    anyhow::ensure!(
        !snap.dirty,
        "working tree is dirty; commit or discard changes first"
    );
    let path = capability.unwrap_or(".");
    run(
        root,
        &[
            "restore",
            "--source",
            commit,
            "--staged",
            "--worktree",
            "--",
            path,
        ],
    )?;
    Ok(())
}

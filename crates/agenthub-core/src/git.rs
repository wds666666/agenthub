use crate::models::{GitIdentity, GitSnapshot};
use anyhow::{Context, Result};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn command(program: &str) -> Command {
    let mut command = Command::new(program);
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("GCM_INTERACTIVE", "Never")
        .env("GIT_ASKPASS", "")
        .env("SSH_ASKPASS", "")
        .env(
            "GIT_SSH_COMMAND",
            "ssh -oBatchMode=yes -oStrictHostKeyChecking=yes -oConnectTimeout=15",
        );
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

pub fn available() -> bool {
    command("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn run(root: &Path, args: &[&str]) -> Result<String> {
    let mut child = command("git")
        .args([
            "-c",
            "core.hooksPath=",
            "-c",
            "core.quotepath=false",
            "-c",
            "commit.gpgSign=false",
            "-c",
            "credential.interactive=false",
        ])
        .arg("-C")
        .arg(root)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("run git")?;
    let mut stdout = child.stdout.take().context("git stdout")?;
    let mut stderr = child.stderr.take().context("git stderr")?;
    let (output_sender, output) = std::sync::mpsc::channel();
    let (error_sender, errors) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut data = Vec::new();
        let _ = output_sender.send(stdout.read_to_end(&mut data).map(|_| data));
    });
    thread::spawn(move || {
        let mut data = Vec::new();
        let _ = error_sender.send(stderr.read_to_end(&mut data).map(|_| data));
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > Duration::from_secs(45) {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!(
                "Git operation timed out; check network and saved credentials, then retry"
            );
        }
        thread::sleep(Duration::from_millis(30));
    };
    let stdout = output
        .recv_timeout(Duration::from_secs(45).saturating_sub(started.elapsed()))
        .context("Git output timed out; check credential helper and retry")??;
    let stderr = errors
        .recv_timeout(Duration::from_secs(45).saturating_sub(started.elapsed()))
        .context("Git error output timed out; check credential helper and retry")??;
    anyhow::ensure!(
        status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    Ok(String::from_utf8_lossy(&stdout).trim().to_string())
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct RemoteSettings {
    pub url: Option<String>,
    pub branch: String,
}
#[derive(Debug, serde::Serialize)]
pub struct CommitResult {
    pub local_saved: bool,
    pub remote_synced: bool,
    pub remote_error: Option<String>,
}

pub fn remote_settings(root: &Path) -> Result<RemoteSettings> {
    ensure_repo(root)?;
    Ok(RemoteSettings {
        url: run(root, &["config", "--get", "remote.agenthub.url"]).ok(),
        branch: run(root, &["config", "--get", "agenthub.remoteBranch"])
            .unwrap_or_else(|_| "main".into()),
    })
}

pub fn connect_remote(root: &Path, url: &str, branch: &str) -> Result<RemoteSettings> {
    ensure_repo(root)?;
    validate_remote(url)?;
    run(root, &["check-ref-format", &format!("refs/heads/{branch}")])?;
    anyhow::ensure!(!branch.starts_with('-'), "invalid branch");
    run(
        root,
        &["ls-remote", "--heads", url, &format!("refs/heads/{branch}")],
    )?;
    run(root, &["config", "remote.agenthub.url", url])?;
    run(root, &["config", "agenthub.remoteBranch", branch])?;
    remote_settings(root)
}
fn validate_remote(url: &str) -> Result<()> {
    let https = url.strip_prefix("https://");
    let ssh = url.strip_prefix("ssh://");
    let scp = url.starts_with("git@") && url.contains(':');
    anyhow::ensure!(
        https.is_some() || ssh.is_some() || scp,
        "use an HTTPS or SSH repository address"
    );
    anyhow::ensure!(
        !url.chars().any(char::is_whitespace) && !url.contains(['?', '#', '\\']),
        "invalid repository address"
    );
    if let Some(rest) = https {
        anyhow::ensure!(
            !rest.split('/').next().unwrap_or("").contains('@'),
            "do not put credentials in repository addresses"
        );
    }
    if let Some(rest) = ssh {
        let authority = rest.split('/').next().unwrap_or("");
        anyhow::ensure!(
            !authority.split('@').next().unwrap_or("").contains(':'),
            "do not put passwords in SSH addresses"
        );
    }
    Ok(())
}
pub fn disconnect_remote(root: &Path) -> Result<()> {
    if remote_settings(root)?.url.is_some() {
        run(root, &["config", "--remove-section", "remote.agenthub"])?;
    }
    Ok(())
}

fn reconcile(root: &Path, url: &str, branch: &str) -> Result<()> {
    let remote_ref = format!("refs/heads/{branch}");
    // An empty repository has no branch to fetch yet.
    if run(root, &["ls-remote", "--heads", url, &remote_ref])?.is_empty() {
        return Ok(());
    }
    run(root, &["fetch", "--no-tags", url, &remote_ref])?;
    let tree = run(root, &["ls-tree", "-r", "FETCH_HEAD"])?;
    for line in tree.lines() {
        let (meta, path) = line.split_once('\t').context("invalid remote tree")?;
        let allowed = path == "agenthub.toml"
            || path == ".gitignore"
            || ["skills/", "plugins/", "rules/", "mcp/"]
                .iter()
                .any(|prefix| path.starts_with(prefix));
        anyhow::ensure!(
            allowed && (meta.starts_with("100644 blob ") || meta.starts_with("100755 blob ")),
            "remote must be a dedicated AgentHub repository with no symlinks or submodules"
        );
    }
    if run(root, &["merge-base", "--is-ancestor", "FETCH_HEAD", "HEAD"]).is_ok() {
        return Ok(());
    }
    let result = (|| -> Result<()> {
        run(
            root,
            &[
                "merge",
                "--no-ff",
                "--no-commit",
                "--allow-unrelated-histories",
                "FETCH_HEAD",
            ],
        )?;
        let paths = crate::paths::AgentHubPaths::new(root.to_path_buf(), root.to_path_buf());
        crate::canonical::inventory(&paths).context("remote library validation failed")?;
        run(
            root,
            &[
                "commit",
                "--no-gpg-sign",
                "-m",
                "Synchronize AgentHub versions",
            ],
        )?;
        Ok(())
    })();
    if let Err(error) = result {
        let conflicts = run(root, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
        if root.join(".git/MERGE_HEAD").exists() {
            run(root, &["merge", "--abort"])
                .context("could not abort remote merge; resolve before retrying")?;
        }
        anyhow::bail!("remote merge stopped; local version preserved. Resolve conflicting resources before retrying: {conflicts} {error}");
    }
    Ok(())
}
pub fn sync_remote(root: &Path) -> Result<()> {
    let settings = remote_settings(root)?;
    let url = settings.url.context("connect a repository first")?;
    validate_remote(&url)?;
    anyhow::ensure!(
        !snapshot(root)?.dirty,
        "save pending library changes before remote sync"
    );
    anyhow::ensure!(
        snapshot(root)?.head.is_some(),
        "save a local version before remote sync"
    );
    let destination = format!("HEAD:refs/heads/{}", settings.branch);
    reconcile(root, &url, &settings.branch)?;
    if let Err(first) = run(root, &["push", &url, &destination]) {
        // Only retry a remote race; authentication and network failures remain actionable.
        if first.to_string().contains("[rejected]") {
            reconcile(root, &url, &settings.branch)?;
            run(root, &["push", &url, &destination])?;
        } else {
            return Err(first);
        }
    }
    Ok(())
}
pub fn commit_and_sync(
    root: &Path,
    message: &str,
    name: Option<&str>,
    email: Option<&str>,
) -> Result<CommitResult> {
    commit(root, message, name, email)?;
    let mut result = CommitResult {
        local_saved: true,
        remote_synced: false,
        remote_error: None,
    };
    if remote_settings(root)?.url.is_some() {
        match sync_remote(root) {
            Ok(()) => result.remote_synced = true,
            Err(error) => result.remote_error = Some(crate::secrets::redact(&error.to_string())),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{paths::AgentHubPaths, AgentHub};
    use std::fs;
    use tempfile::TempDir;

    fn device(home: &Path) -> AgentHub {
        let hub = AgentHub::open(AgentHubPaths::for_home(home)).unwrap();
        commit(
            &hub.paths.root,
            "Initialize library",
            Some("Test"),
            Some("test@example.com"),
        )
        .unwrap();
        hub
    }
    fn skill(hub: &AgentHub, id: &str, body: &str) {
        let dir = hub.paths.skills.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {id}\ndescription: Test\n---\n{body}"),
        )
        .unwrap();
    }
    fn local_remote(root: &Path, remote: &Path) {
        // File transport is strictly a test fixture; public connection rejects it.
        run(
            root,
            &["config", "remote.agenthub.url", remote.to_str().unwrap()],
        )
        .unwrap();
        run(root, &["config", "agenthub.remoteBranch", "main"]).unwrap();
    }
    // Use internal reconciliation directly for local fixtures; sync_remote checks public URLs.
    fn sync_fixture(root: &Path, remote: &Path) -> Result<()> {
        reconcile(root, remote.to_str().unwrap(), "main")?;
        run(
            root,
            &["push", remote.to_str().unwrap(), "HEAD:refs/heads/main"],
        )?;
        Ok(())
    }
    #[test]
    fn devices_merge_distinct_resources_and_preserve_conflicting_local_versions() {
        let temp = TempDir::new().unwrap();
        let remote = temp.path().join("remote.git");
        fs::create_dir(&remote).unwrap();
        run(&remote, &["init", "--bare"]).unwrap();
        let a = device(&temp.path().join("a"));
        let b = device(&temp.path().join("b"));
        skill(&a, "alpha", "initial");
        commit(&a.paths.root, "Add alpha", None, None).unwrap();
        sync_fixture(&a.paths.root, &remote).unwrap();
        sync_fixture(&b.paths.root, &remote).unwrap();
        skill(&a, "alpha", "device A");
        commit(&a.paths.root, "Edit alpha A", None, None).unwrap();
        sync_fixture(&a.paths.root, &remote).unwrap();
        skill(&b, "beta", "device B");
        commit(&b.paths.root, "Add beta B", None, None).unwrap();
        sync_fixture(&b.paths.root, &remote).unwrap();
        assert!(fs::read_to_string(b.paths.skills.join("alpha/SKILL.md"))
            .unwrap()
            .contains("device A"));
        sync_fixture(&a.paths.root, &remote).unwrap();
        assert!(a.paths.skills.join("beta/SKILL.md").exists());
        skill(&a, "alpha", "conflict A");
        skill(&b, "alpha", "conflict B");
        commit(&a.paths.root, "Conflict A", None, None).unwrap();
        commit(&b.paths.root, "Conflict B", None, None).unwrap();
        sync_fixture(&a.paths.root, &remote).unwrap();
        let before = snapshot(&b.paths.root).unwrap().head;
        assert!(sync_fixture(&b.paths.root, &remote).is_err());
        assert_eq!(before, snapshot(&b.paths.root).unwrap().head);
        assert!(!snapshot(&b.paths.root).unwrap().dirty);
        assert!(fs::read_to_string(b.paths.skills.join("alpha/SKILL.md"))
            .unwrap()
            .contains("conflict B"));
    }
    #[test]
    fn push_failure_is_separate_from_a_saved_local_version() {
        let temp = TempDir::new().unwrap();
        let hub = device(temp.path());
        local_remote(&hub.paths.root, &temp.path().join("missing.git"));
        skill(&hub, "alpha", "pending upload");
        let result = commit_and_sync(&hub.paths.root, "Save offline", None, None).unwrap();
        assert!(result.local_saved);
        assert!(!result.remote_synced);
        assert!(result.remote_error.is_some());
        assert!(!snapshot(&hub.paths.root).unwrap().dirty);
    }
    #[test]
    fn remote_validation_rejects_credentials_options_and_local_paths() {
        for value in [
            "--upload-pack=evil",
            "/tmp/repo",
            "file:///tmp/repo",
            "https://token@github.com/repo",
            "https://github.com/repo?token=x",
            "ssh://user:password@host/repo",
        ] {
            assert!(validate_remote(value).is_err(), "{value}");
        }
        for value in [
            "https://github.com/user/repo.git",
            "git@github.com:user/repo.git",
            "ssh://git@host/repo",
        ] {
            assert!(validate_remote(value).is_ok());
        }
    }
    #[test]
    fn remote_runtime_content_is_rejected_before_checkout() {
        let temp = TempDir::new().unwrap();
        let a = device(&temp.path().join("a"));
        let b = device(&temp.path().join("b"));
        fs::write(a.paths.root.join("state/leak"), "private").unwrap();
        run(&a.paths.root, &["add", "-f", "state/leak"]).unwrap();
        run(&a.paths.root, &["commit", "-m", "Unsafe tree"]).unwrap();
        let before = snapshot(&b.paths.root).unwrap().head;
        let branch = run(&a.paths.root, &["branch", "--show-current"]).unwrap();
        assert!(reconcile(&b.paths.root, a.paths.root.to_str().unwrap(), &branch).is_err());
        assert_eq!(before, snapshot(&b.paths.root).unwrap().head);
        assert!(!b.paths.root.join("state/leak").exists());
    }
}

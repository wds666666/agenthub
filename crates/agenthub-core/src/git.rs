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

pub(crate) fn run(root: &Path, args: &[&str]) -> Result<String> {
    let network_url = if matches!(args.first(), Some(&"ls-remote" | &"fetch" | &"push")) {
        args.iter().find(|arg| {
            arg.starts_with("https://") || arg.starts_with("ssh://") || arg.starts_with("git@")
        })
    } else {
        None
    };
    let credential = network_url
        .filter(|url| url.starts_with("https://"))
        .map(|url| crate::git_auth::for_url(root, url))
        .transpose()?
        .flatten();
    let mut process = command("git");
    process.args([
        "-c",
        "core.hooksPath=",
        "-c",
        "core.quotepath=false",
        "-c",
        "commit.gpgSign=false",
        "-c",
        "credential.interactive=false",
    ]);
    if network_url.is_some() {
        process.args([
            "-c",
            "http.followRedirects=false",
            "-c",
            "credential.useHttpPath=true",
        ]);
        for name in [
            "GIT_TRACE",
            "GIT_TRACE_PACKET",
            "GIT_TRACE_CURL",
            "GIT_CURL_VERBOSE",
            "GIT_TRACE_CURL_NO_DATA",
            "GIT_TRACE2",
            "GIT_TRACE2_EVENT",
            "GIT_TRACE2_PERF",
        ] {
            process.env_remove(name);
        }
    }
    if let Some(saved) = &credential {
        process
            .args([
                "-c",
                "credential.helper=",
                "-c",
                &crate::git_auth::helper_configuration()?,
                "-c",
                &format!("credential.username={}", saved.username),
            ])
            .env("AGENTHUB_GIT_ROOT", root);
    }
    let mut child = process
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
            if let Some(url) = network_url {
                mark_network_failure(root, url, "network_error");
            }
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
    let error = crate::git_auth::sanitize(&String::from_utf8_lossy(&stderr), credential.as_ref());
    if !status.success() {
        if let Some(url) = network_url {
            let lower = error.to_ascii_lowercase();
            let state = if [
                "authentication failed",
                "could not read username",
                "could not read password",
                "401",
                "403",
                "access denied",
                "permission denied",
                "invalid username",
                "repository not found",
            ]
            .iter()
            .any(|s| lower.contains(s))
            {
                "auth_failed"
            } else {
                "network_error"
            };
            mark_network_failure(root, url, state);
        }
        anyhow::bail!("git failed: {error}");
    }
    Ok(
        crate::git_auth::sanitize(&String::from_utf8_lossy(&stdout), credential.as_ref())
            .trim()
            .to_string(),
    )
}
fn mark_network_failure(root: &Path, url: &str, state: &str) {
    let branch =
        run(root, &["config", "--get", "agenthub.remoteBranch"]).unwrap_or_else(|_| "main".into());
    let _ = crate::git_auth::remember_state(root, url, &branch, state);
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
    crate::canonical::validate(&crate::paths::AgentHubPaths::new(
        root.to_path_buf(),
        root.to_path_buf(),
    ))?;
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
    pub state: String,
    pub credential_saved: bool,
}
#[derive(Debug, serde::Serialize)]
pub struct CommitResult {
    pub local_saved: bool,
    pub remote_synced: bool,
    pub remote_error: Option<String>,
}

pub fn remote_settings(root: &Path) -> Result<RemoteSettings> {
    ensure_repo(root)?;
    let url = run(root, &["config", "--get", "remote.agenthub.url"]).ok();
    let branch =
        run(root, &["config", "--get", "agenthub.remoteBranch"]).unwrap_or_else(|_| "main".into());
    let state = match &url {
        Some(url) => crate::git_auth::state(root, url, &branch)?,
        None => "disconnected".into(),
    };
    let credential_saved = url
        .as_ref()
        .filter(|url| url.starts_with("https://"))
        .map(|url| crate::git_auth::for_url(root, url))
        .transpose()?
        .flatten()
        .is_some();
    Ok(RemoteSettings {
        url,
        branch,
        state,
        credential_saved,
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
    crate::git_auth::remember_state(root, url, branch, "read_verified")?;
    remote_settings(root)
}
pub(crate) fn validate_remote(url: &str) -> Result<()> {
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
        crate::git_auth::https_url(url)?;
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
    if crate::paths::AgentHubPaths::new(root.to_path_buf(), root.to_path_buf())
        .database
        .exists()
    {
        crate::git_auth::forget(root)?;
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
    if run(root, &["merge-base", "--is-ancestor", "HEAD", "FETCH_HEAD"]).is_ok() {
        validate_incoming(root)?;
        // No merge commit or author setup is needed for a restored device that
        // merely follows newer remote versions. A concurrent local edit is still
        // protected by Git's ordinary fast-forward/working-tree checks.
        run(root, &["merge", "--ff-only", "FETCH_HEAD"])?;
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
        crate::canonical::validate(&paths).context("remote library validation failed")?;
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

fn validate_incoming(root: &Path) -> Result<()> {
    let source_root = root.canonicalize()?;
    let stage_root = source_root
        .join("runtime")
        .join(format!("receive-{}", uuid::Uuid::new_v4()));
    let runtime = stage_root.parent().context("incoming stage parent")?;
    if runtime.exists() {
        anyhow::ensure!(
            std::fs::symlink_metadata(runtime)?.file_type().is_dir(),
            "incoming validation runtime cannot be a symlink"
        );
    }
    std::fs::create_dir_all(runtime)?;
    let result = (|| -> Result<()> {
        let incoming = run(root, &["rev-parse", "FETCH_HEAD"])?;
        // Local object sharing copies no runtime state and makes no network
        // request. Hooks remain disabled by the common process wrapper.
        run(
            root,
            &[
                "clone",
                "--shared",
                "--no-checkout",
                "--",
                source_root.to_str().context("invalid library path")?,
                stage_root.to_str().context("invalid staging path")?,
            ],
        )?;
        run(&stage_root, &["checkout", "--detach", &incoming])?;
        let paths = crate::paths::AgentHubPaths::new(root.to_path_buf(), stage_root.clone());
        paths.ensure_runtime()?;
        crate::canonical::validate(&paths).context("incoming library validation failed")?;
        check_upload_history(&stage_root).context("incoming history is not portable")?;
        Ok(())
    })();
    if stage_root.exists() {
        std::fs::remove_dir_all(&stage_root)
            .context("clean private incoming validation checkout")?;
    }
    result
}

fn credential_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase().replace('-', "_");
    [
        "token",
        "secret",
        "password",
        "authorization",
        "api_key",
        "apikey",
        "private_key",
        "access_key",
        "credential",
    ]
    .iter()
    .any(|key| name.contains(key))
        || name.ends_with("_key")
}
fn placeholder(value: &str) -> bool {
    let value = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("Basic "))
        .unwrap_or(value);
    value.is_empty()
        || (value.starts_with("${") && value.ends_with('}'))
        || value.starts_with("env:")
        || value.starts_with("secret://")
}
fn has_plaintext_credentials(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => object.iter().any(|(key, value)| {
            if credential_name(key) && value.as_str().is_some_and(|text| !placeholder(text)) {
                return true;
            }
            if key == "url"
                && value.as_str().is_some_and(|url| {
                    let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
                    rest.split('/').next().unwrap_or("").contains('@')
                        || url.split_once('?').is_some_and(|(_, query)| {
                            query
                                .split('&')
                                .any(|part| credential_name(part.split('=').next().unwrap_or("")))
                        })
                })
            {
                return true;
            }
            if key == "args"
                && value.as_array().is_some_and(|args| {
                    args.iter().enumerate().any(|(index, arg)| {
                        let Some(arg) = arg.as_str() else {
                            return false;
                        };
                        if !arg.starts_with('-')
                            || !credential_name(arg.split('=').next().unwrap_or(""))
                        {
                            return false;
                        }
                        arg.split_once('=')
                            .map(|(_, text)| !placeholder(text))
                            .unwrap_or_else(|| {
                                args.get(index + 1)
                                    .and_then(|value| value.as_str())
                                    .is_some_and(|text| !placeholder(text))
                            })
                    })
                })
            {
                return true;
            }
            has_plaintext_credentials(value)
        }),
        serde_json::Value::Array(array) => array.iter().any(has_plaintext_credentials),
        _ => false,
    }
}
/// Check history as well as HEAD: deleting a token in a later version does not remove it from a push.
pub(crate) fn check_upload_history(root: &Path) -> Result<()> {
    let objects = run(root, &["rev-list", "--objects", "HEAD"])?;
    for line in objects.lines() {
        let Some((object, path)) = line.split_once(' ') else {
            continue;
        };
        if path.is_empty() {
            continue;
        }
        let allowed = path == "agenthub.toml"
            || path == ".gitignore"
            || ["skills", "plugins", "rules", "mcp"]
                .iter()
                .any(|domain| path == *domain || path.starts_with(&format!("{domain}/")));
        anyhow::ensure!(allowed, "remote upload blocked: version history contains a non-library path ({path}); use a clean dedicated library history");
        if !path.ends_with(".json") {
            continue;
        }
        let content = run(root, &["cat-file", "blob", object])?;
        let value: serde_json::Value = match serde_json::from_str(&content) {
            Ok(value) => value,
            // Some Skill references preserve an old .json path as Markdown documentation.
            // No JSON objects may be concealed in that compatibility document.
            Err(_)
                if path.starts_with("skills/")
                    && content.starts_with("# ")
                    && !content.contains(['{', '}']) =>
            {
                continue;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("remote upload blocked: cannot inspect configuration at {path}")
                });
            }
        };
        anyhow::ensure!(!has_plaintext_credentials(&value), "remote upload blocked: plaintext credentials in version history at {path}; replace with environment placeholders and remove sensitive historical versions before sharing");
    }
    Ok(())
}

pub fn sync_remote(root: &Path) -> Result<()> {
    let result = sync_remote_inner(root);
    if result.is_err() {
        if let Ok(settings) = remote_settings(root) {
            if let Some(url) = settings.url {
                if settings.state != "auth_failed" && settings.state != "network_error" {
                    let _ = crate::git_auth::remember_state(
                        root,
                        &url,
                        &settings.branch,
                        "sync_failed",
                    );
                }
            }
        }
    }
    result
}

fn sync_remote_inner(root: &Path) -> Result<()> {
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
    check_upload_history(root)?;
    let paths = crate::paths::AgentHubPaths::new(root.to_path_buf(), root.to_path_buf());
    crate::canonical::validate(&paths)?;
    let destination = format!("HEAD:refs/heads/{}", settings.branch);
    reconcile(root, &url, &settings.branch)?;
    crate::canonical::validate(&paths)?;
    check_upload_history(root)?;
    if let Err(first) = run(root, &["push", &url, &destination]) {
        // Only retry a remote race; authentication and network failures remain actionable.
        if first.to_string().contains("[rejected]") {
            reconcile(root, &url, &settings.branch)?;
            check_upload_history(root)?;
            run(root, &["push", &url, &destination])?;
        } else {
            return Err(first);
        }
    }
    crate::git_auth::remember_state(root, &url, &settings.branch, "synced")?;
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
    match remote_settings(root) {
        Ok(settings) if settings.url.is_some() => match sync_remote(root) {
            Ok(()) => result.remote_synced = true,
            Err(error) => result.remote_error = Some(crate::secrets::redact(&error.to_string())),
        },
        Err(error) => result.remote_error = Some(crate::secrets::redact(&error.to_string())),
        _ => {}
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
    #[test]
    fn unreadable_saved_credentials_do_not_hide_a_successful_local_commit() {
        let home = TempDir::new().unwrap();
        let hub = device(home.path());
        let url = "https://git.example/team/library.git";
        run(&hub.paths.root, &["config", "remote.agenthub.url", url]).unwrap();
        let credential = crate::git_auth::Credential {
            url: url.into(),
            username: "test".into(),
            token: "fixture-only".into(),
        };
        crate::secrets::set(
            &hub.store,
            &hub.paths,
            "agenthub.git.https",
            &serde_json::to_vec(&credential).unwrap(),
        )
        .unwrap();
        fs::remove_file(&hub.paths.master_key).unwrap();
        skill(&hub, "new-skill", "Local content");
        let result = commit_and_sync(&hub.paths.root, "Saved locally", None, None).unwrap();
        assert!(result.local_saved);
        assert!(!result.remote_synced);
        assert!(result.remote_error.is_some());
        assert!(!snapshot(&hub.paths.root).unwrap().dirty);
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

        // Exercise the manager skill's native resolution after the normal abort.
        assert!(!b.paths.root.join(".git/MERGE_HEAD").exists());
        let remote_head = run(&b.paths.root, &["rev-parse", "FETCH_HEAD"]).unwrap();
        assert!(run(
            &b.paths.root,
            &["merge", "--no-ff", "--no-commit", &remote_head]
        )
        .is_err());
        assert!(b.paths.root.join(".git/MERGE_HEAD").exists());
        skill(
            &b,
            "alpha",
            "Reviewed combination: conflict A and conflict B",
        );
        crate::canonical::validate(&b.paths).unwrap();
        run(&b.paths.root, &["add", "skills/alpha/SKILL.md"]).unwrap();
        assert!(
            run(&b.paths.root, &["diff", "--name-only", "--diff-filter=U"])
                .unwrap()
                .is_empty()
        );
        run(&b.paths.root, &["diff", "--cached", "--check"]).unwrap();
        run(&b.paths.root, &["commit", "-m", "Resolve alpha versions"]).unwrap();
        let parents = run(&b.paths.root, &["rev-list", "--parents", "-n", "1", "HEAD"]).unwrap();
        assert_eq!(parents.split_whitespace().count(), 3);
        check_upload_history(&b.paths.root).unwrap();
        sync_fixture(&b.paths.root, &remote).unwrap();
        sync_fixture(&a.paths.root, &remote).unwrap();
        assert!(fs::read_to_string(a.paths.skills.join("alpha/SKILL.md"))
            .unwrap()
            .contains("conflict A and conflict B"));
        assert!(!temp.path().join("a/.cursor").exists());
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
    fn invalid_content_is_blocked_before_any_remote_network_access() {
        let temp = TempDir::new().unwrap();
        let hub = device(temp.path());
        skill(&hub, "broken", "");
        fs::write(hub.paths.skills.join("broken/SKILL.md"), "").unwrap();
        run(&hub.paths.root, &["add", "skills/broken/SKILL.md"]).unwrap();
        run(
            &hub.paths.root,
            &["commit", "-m", "Unchecked external edit"],
        )
        .unwrap();
        local_remote(
            &hub.paths.root,
            Path::new("https://example.invalid/library.git"),
        );
        assert!(sync_remote(&hub.paths.root)
            .unwrap_err()
            .to_string()
            .contains("skill broken"));
    }
    #[test]
    fn markdown_reference_compatibility_does_not_bypass_json_credential_checks() {
        for (path, body, allowed) in [
            (
                "skills/alpha/reference.json",
                "# Compatibility entry\nSee [new index](xml/index.json).\n",
                true,
            ),
            ("skills/alpha/reference.json", "not JSON", false),
            (
                "skills/alpha/reference.json",
                "# Reference\n{\"token\":\"private-value\"}",
                false,
            ),
            (
                "skills/alpha/reference.json",
                "{\"token\":\"private-value\"}",
                false,
            ),
            (
                "mcp/reference.json",
                "# Compatibility entry\nSee index.json.\n",
                false,
            ),
        ] {
            let temp = TempDir::new().unwrap();
            let hub = device(temp.path());
            let resource = hub.paths.root.join(path);
            fs::create_dir_all(resource.parent().unwrap()).unwrap();
            fs::write(resource, body).unwrap();
            run(&hub.paths.root, &["add", path]).unwrap();
            run(&hub.paths.root, &["commit", "-m", "Reference fixture"]).unwrap();
            let result = check_upload_history(&hub.paths.root);
            assert_eq!(result.is_ok(), allowed, "unexpected result for {path}");
            if let Err(error) = result {
                assert!(!format!("{error:#}").contains("private-value"));
            }
        }
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
    #[test]
    fn plaintext_credentials_are_blocked_even_after_removal_from_current_version() {
        let temp = TempDir::new().unwrap();
        let hub = device(temp.path());
        let dir = hub.paths.mcp.join("test");
        fs::create_dir_all(&dir).unwrap();
        let config = dir.join("server.json");
        fs::write(&config, r#"{"schemaVersion":1,"id":"test","display_name":"Test","transport":"stdio","command":"server","env":{"API_TOKEN":"private-value"}}"#).unwrap();
        commit(&hub.paths.root, "Import sensitive config", None, None).unwrap();
        assert!(check_upload_history(&hub.paths.root).is_err());
        fs::write(&config, r#"{"schemaVersion":1,"id":"test","display_name":"Test","transport":"stdio","command":"server","env":{"API_TOKEN":"${API_TOKEN}"}}"#).unwrap();
        commit(&hub.paths.root, "Use environment reference", None, None).unwrap();
        let error = check_upload_history(&hub.paths.root)
            .unwrap_err()
            .to_string();
        assert!(error.contains("mcp/test/server.json"), "{error}");
        assert!(!error.contains("private-value"));
        let clean = device(&temp.path().join("clean"));
        fs::create_dir_all(clean.paths.mcp.join("test")).unwrap();
        fs::write(
            clean.paths.mcp.join("test/server.json"),
            r#"{"schemaVersion":1,"id":"test","display_name":"Test","transport":"stdio","command":"server","env":{"API_TOKEN":"${API_TOKEN}","PORT":"8000"}}"#,
        )
        .unwrap();
        commit(&clean.paths.root, "Environment reference only", None, None).unwrap();
        check_upload_history(&clean.paths.root).unwrap();
    }
    #[test]
    fn credential_detection_covers_headers_urls_and_arguments() {
        for value in [
            serde_json::json!({"headers":{"Authorization":"Bearer private-value"}}),
            serde_json::json!({"url":"https://host/mcp?api_key=private-value"}),
            serde_json::json!({"url":"https://user:password@host/mcp"}),
            serde_json::json!({"args":["--token","private-value"]}),
            serde_json::json!({"args":["--api-key=private-value"]}),
        ] {
            assert!(has_plaintext_credentials(&value));
        }
        assert!(!has_plaintext_credentials(
            &serde_json::json!({"headers":{"Authorization":"Bearer ${AUTH_HEADER}"}, "args":["--token","${TOKEN}"]})
        ));
    }
}

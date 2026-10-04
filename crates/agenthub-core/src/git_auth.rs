//! Repository-scoped HTTPS credentials, shared by desktop and CLI.
use crate::{paths::AgentHubPaths, secrets, storage::Store};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};
use url::Url;

const SECRET: &str = "agenthub.git.https";
const PROOF: &str = "git_remote_verification";
const HELPER_MODE: &str = "__git-credential";

pub fn token_settings(repository: &str, platform: &str) -> Result<String> {
    let url = https_url(repository)?;
    if platform == "github" {
        anyhow::ensure!(url.host_str() == Some("github.com"), "GitHub sign-in requires a github.com repository; use self-hosted Git for other servers");
        Ok("https://github.com/settings/personal-access-tokens".into())
    } else {
        anyhow::ensure!(platform == "git", "unsupported Git platform");
        Ok(format!(
            "{}/user/settings/applications",
            url.origin().ascii_serialization()
        ))
    }
}

// Deliberately not Debug: credentials must never enter diagnostics.
#[derive(Serialize, Deserialize)]
pub(crate) struct Credential {
    pub url: String,
    pub username: String,
    pub token: String,
}
#[derive(Serialize, Deserialize)]
struct Proof {
    url: String,
    branch: String,
    state: String,
}

fn paths(root: &Path) -> AgentHubPaths {
    AgentHubPaths::new(root.to_path_buf(), root.to_path_buf())
}
fn store(root: &Path) -> Result<Store> {
    Store::open(&paths(root).database)
}

pub(crate) fn https_url(value: &str) -> Result<Url> {
    let url = Url::parse(value).context("invalid repository address")?;
    anyhow::ensure!(
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "use an HTTPS repository address without credentials"
    );
    anyhow::ensure!(
        !value.chars().any(|c| c.is_control() || c.is_whitespace())
            && !value.contains('\\')
            && !url.path().trim_matches('/').is_empty(),
        "invalid repository address"
    );
    Ok(url)
}
fn scope(value: &str) -> Result<(String, String)> {
    let url = https_url(value)?;
    Ok((
        url[url::Position::BeforeHost..url::Position::AfterPort].to_string(),
        url.path()
            .trim_start_matches('/')
            .trim_end_matches('/')
            .to_string(),
    ))
}
pub(crate) fn credential(root: &Path) -> Result<Option<Credential>> {
    if !paths(root).database.exists() {
        return Ok(None);
    }
    secrets::get(&store(root)?, &paths(root), SECRET)?
        .map(|v| serde_json::from_slice(&v).context("invalid saved Git credential"))
        .transpose()
}
pub(crate) fn for_url(root: &Path, url: &str) -> Result<Option<Credential>> {
    let Some(credential) = credential(root)? else {
        return Ok(None);
    };
    if scope(&credential.url)? == scope(url)? {
        Ok(Some(credential))
    } else {
        Ok(None)
    }
}
pub fn forget(root: &Path) -> Result<()> {
    store(root)?.delete_secret(SECRET)?;
    store(root)?.set_meta(PROOF, "")?;
    Ok(())
}
pub(crate) fn remember_state(root: &Path, url: &str, branch: &str, state: &str) -> Result<()> {
    if paths(root).database.exists() {
        store(root)?.set_meta(
            PROOF,
            &serde_json::to_string(&Proof {
                url: url.into(),
                branch: branch.into(),
                state: state.into(),
            })?,
        )?;
    }
    Ok(())
}
pub(crate) fn state(root: &Path, url: &str, branch: &str) -> Result<String> {
    if paths(root).database.exists() {
        if let Some(value) = store(root)?.meta(PROOF)? {
            if let Ok(proof) = serde_json::from_str::<Proof>(&value) {
                if proof.url == url && proof.branch == branch {
                    return Ok(proof.state);
                }
            }
        }
    }
    Ok("unverified".into())
}
pub fn login(
    root: &Path,
    url: &str,
    branch: &str,
    username: &str,
    token: &str,
) -> Result<crate::git::RemoteSettings> {
    https_url(url)?;
    anyhow::ensure!(
        !username.trim().is_empty()
            && !username.chars().any(char::is_control)
            && !username.contains(':')
            && !token.trim().is_empty()
            && !token.chars().any(char::is_control),
        "username and access token are required"
    );
    let db = store(root)?;
    let previous = secrets::get(&db, &paths(root), SECRET)?;
    let value = Credential {
        url: url.into(),
        username: username.trim().into(),
        token: token.into(),
    };
    secrets::set(&db, &paths(root), SECRET, &serde_json::to_vec(&value)?)?;
    match crate::git::connect_remote(root, url, branch) {
        Ok(result) => Ok(result),
        Err(error) => {
            if let Some(old) = previous {
                secrets::set(&db, &paths(root), SECRET, &old)?;
            } else {
                db.delete_secret(SECRET)?;
            }
            // Never forward raw provider output from a token-bearing operation.
            Err(anyhow::anyhow!(sanitize(&error.to_string(), Some(&value))))
        }
    }
}
pub(crate) fn sanitize(message: &str, credential: Option<&Credential>) -> String {
    let mut message = message.to_string();
    if let Some(c) = credential {
        for secret in [
            &c.token,
            &STANDARD.encode(format!("{}:{}", c.username, c.token)),
            &url::form_urlencoded::byte_serialize(c.token.as_bytes()).collect::<String>(),
        ] {
            if !secret.is_empty() {
                message = message.replace(secret, "[REDACTED]");
            }
        }
    }
    secrets::redact(&message)
}
pub(crate) fn helper_configuration() -> Result<String> {
    let exe = std::env::current_exe()?;
    // Git shell helpers use POSIX quoting on Windows too (Git for Windows sh).
    let escaped = exe
        .to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "'\\''");
    Ok(format!("credential.helper=!'{}' {HELPER_MODE}", escaped))
}
fn answer(root: &Path, operation: &str, input: &str) -> Result<String> {
    if operation != "get" {
        return Ok(String::new());
    }
    let fields: BTreeMap<_, _> = input
        .lines()
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once('='))
        .collect();
    if fields.get("protocol") != Some(&"https") {
        return Ok(String::new());
    }
    let (Some(host), Some(path)) = (fields.get("host"), fields.get("path")) else {
        return Ok(String::new());
    };
    let Some(c) = for_url(root, &format!("https://{host}/{path}"))? else {
        return Ok(String::new());
    };
    if fields.get("username").is_some_and(|u| *u != c.username) {
        return Ok(String::new());
    }
    Ok(format!("username={}\npassword={}\n\n", c.username, c.token))
}
/// Must run before the CLI parser or desktop runtime; never opens a window.
pub fn dispatch_helper() -> bool {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some(HELPER_MODE) {
        return false;
    }
    // Error paths intentionally produce no diagnostics or credential fragments.
    let _ = (|| -> Result<()> {
        let root = std::env::var_os("AGENTHUB_GIT_ROOT").context("missing credential context")?;
        let mut input = String::new();
        std::io::stdin()
            .take(16 * 1024)
            .read_to_string(&mut input)?;
        let output = answer(
            Path::new(&root),
            args.get(2).map(String::as_str).unwrap_or(""),
            &input,
        )?;
        std::io::stdout().write_all(output.as_bytes())?;
        Ok(())
    })();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helper_is_encrypted_repository_scoped_and_forgettable() -> Result<()> {
        let home = tempfile::tempdir()?;
        let hub = crate::AgentHub::open(AgentHubPaths::for_home(home.path()))?;
        let c = Credential {
            url: "https://git.example:20110/power/skills.git".into(),
            username: "me".into(),
            token: "private-test-token".into(),
        };
        secrets::set(&hub.store, &hub.paths, SECRET, &serde_json::to_vec(&c)?)?;
        assert!(answer(
            &hub.paths.root,
            "get",
            "protocol=https\nhost=git.example:20110\npath=power/skills.git\n\n"
        )?
        .contains("password=private-test-token"));
        for fields in [
            "protocol=http\nhost=git.example:20110\npath=power/skills.git\n",
            "protocol=https\nhost=other.example:20110\npath=power/skills.git\n",
            "protocol=https\nhost=git.example\npath=power/skills.git\n",
            "protocol=https\nhost=git.example:20110\npath=power/another.git\n",
            "protocol=https\nhost=git.example:20110\n",
            "protocol=https\nhost=git.example:20110\npath=power/skills.git\nusername=other\n",
        ] {
            assert!(answer(&hub.paths.root, "get", fields)?.is_empty());
        }
        assert!(answer(&hub.paths.root, "store", "")?.is_empty());
        let (_, _, encrypted) = hub.store.secret(SECRET)?.unwrap();
        assert!(!encrypted
            .windows(c.token.len())
            .any(|v| v == c.token.as_bytes()));
        remember_state(&hub.paths.root, &c.url, "agenthub", "read_verified")?;
        assert_eq!(state(&hub.paths.root, &c.url, "agenthub")?, "read_verified");
        assert_eq!(state(&hub.paths.root, &c.url, "main")?, "unverified");
        forget(&hub.paths.root)?;
        assert!(credential(&hub.paths.root)?.is_none());
        assert_eq!(state(&hub.paths.root, &c.url, "agenthub")?, "unverified");
        Ok(())
    }
    #[test]
    fn sanitizer_removes_token_and_basic_auth() {
        let c = Credential {
            url: "https://example.com/a/b.git".into(),
            username: "me".into(),
            token: "test+private/token".into(),
        };
        let encoded = STANDARD.encode(format!("{}:{}", c.username, c.token));
        let value = sanitize(&format!("{} {encoded}", c.token), Some(&c));
        assert!(!value.contains(&c.token));
        assert!(!value.contains(&encoded));
    }
}

use crate::{
    canonical::{inventory, sha256},
    models::{CapabilityKind, McpServer, Target},
    paths::AgentHubPaths,
};
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct DomainProjection {
    pub name: &'static str,
    pub kind: CapabilityKind,
    pub target_path: PathBuf,
    pub files: BTreeMap<PathBuf, Vec<u8>>,
    pub preserve_names: Vec<String>,
    pub sensitive: bool,
}

impl DomainProjection {
    pub fn digest(&self) -> String {
        if self.files.len() == 1 {
            if let Some(bytes) = self.files.get(Path::new("")) {
                return sha256(bytes);
            }
        }
        let mut h = Sha256::new();
        for (p, b) in &self.files {
            h.update(p.to_string_lossy().as_bytes());
            h.update(b);
        }
        hex::encode(h.finalize())
    }
}

pub fn projection(paths: &AgentHubPaths, target: Target) -> Result<Vec<DomainProjection>> {
    let caps = inventory(paths)?;
    let skill_files = tree_files(&paths.skills)?;
    let plugin_files = tree_files(&paths.plugins)?;
    let rule_caps: Vec<_> = caps
        .iter()
        .filter(|c| c.kind == CapabilityKind::Rule)
        .collect();
    let (skill_path, mcp_path, plugin_path, rule_projection) = match target {
        Target::Cursor => (
            paths.user_home.join(".cursor/skills"),
            paths.user_home.join(".cursor/mcp.json"),
            paths.user_home.join(".cursor/plugins/local"),
            cursor_rules(paths, &rule_caps)?,
        ),
        Target::Codex => (
            paths.user_home.join(".codex/skills"),
            paths.user_home.join(".codex/config.toml"),
            paths.user_home.join(".codex/plugins"),
            codex_rules(paths, &rule_caps)?,
        ),
        Target::Claude => (
            paths.user_home.join(".claude/skills"),
            paths.user_home.join(".claude.json"),
            paths.user_home.join(".claude/plugins"),
            claude_rules(paths, &rule_caps)?,
        ),
    };
    let mut target_plugins = BTreeMap::new();
    for (path, bytes) in plugin_files {
        if path
            .file_name()
            .is_some_and(|n| n == "agenthub.plugin.json")
        {
            let parent = path.parent().unwrap_or(Path::new(""));
            target_plugins.insert(parent.join(manifest_name(target)), bytes);
        } else {
            target_plugins.insert(path, bytes);
        }
    }
    if target == Target::Cursor {
        for (path, bytes) in &rule_projection.files {
            target_plugins.insert(PathBuf::from("agenthub-rules").join(path), bytes.clone());
        }
    }
    let mcp = project_mcp(paths, target, &mcp_path)?;
    let skill_preserve = if target == Target::Claude {
        vec![".synced".into(), "synced".into()]
    } else {
        Vec::new()
    };
    let plugin_preserve = if target == Target::Claude {
        vec!["managed".into(), "marketplaces".into()]
    } else {
        Vec::new()
    };
    let mut out = vec![
        DomainProjection {
            name: "skills",
            kind: CapabilityKind::Skill,
            target_path: skill_path,
            files: skill_files,
            preserve_names: skill_preserve,
            sensitive: false,
        },
        DomainProjection {
            name: "mcp",
            kind: CapabilityKind::Mcp,
            target_path: mcp_path,
            files: BTreeMap::from([(PathBuf::new(), mcp)]),
            preserve_names: Vec::new(),
            sensitive: true,
        },
        DomainProjection {
            name: "plugins",
            kind: CapabilityKind::Plugin,
            target_path: plugin_path,
            files: target_plugins,
            preserve_names: plugin_preserve,
            sensitive: false,
        },
    ];
    if target != Target::Cursor {
        out.push(rule_projection);
    }
    Ok(out)
}

fn manifest_name(target: Target) -> PathBuf {
    match target {
        Target::Cursor => PathBuf::from(".cursor-plugin/plugin.json"),
        Target::Codex => PathBuf::from(".codex-plugin/plugin.json"),
        Target::Claude => PathBuf::from(".claude-plugin/plugin.json"),
    }
}
fn tree_files(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut out = BTreeMap::new();
    if !root.exists() {
        return Ok(out);
    }
    for e in WalkDir::new(root).follow_links(false) {
        let e = e?;
        if e.file_type().is_symlink() {
            anyhow::bail!(
                "symlink is not allowed in canonical projection: {}",
                e.path().display()
            );
        }
        if e.file_type().is_file() {
            out.insert(
                e.path().strip_prefix(root)?.to_path_buf(),
                fs::read(e.path())?,
            );
        }
    }
    Ok(out)
}
fn read_rules(
    _paths: &AgentHubPaths,
    caps: &[&crate::models::Capability],
) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for c in caps {
        let p = c.path.join("rule.md");
        if p.is_file() {
            out.push((c.id.clone(), fs::read_to_string(p)?));
        }
    }
    Ok(out)
}
fn codex_rules(
    paths: &AgentHubPaths,
    caps: &[&crate::models::Capability],
) -> Result<DomainProjection> {
    let mut body = String::from("# AgentHub managed global rules\n\n");
    for (id, text) in read_rules(paths, caps)? {
        body.push_str(&format!("## {id}\n\n{}\n\n", text.trim()));
    }
    Ok(DomainProjection {
        name: "rules",
        kind: CapabilityKind::Rule,
        target_path: paths.user_home.join(".codex/AGENTS.md"),
        files: BTreeMap::from([(PathBuf::new(), body.into_bytes())]),
        preserve_names: Vec::new(),
        sensitive: false,
    })
}
fn claude_rules(
    paths: &AgentHubPaths,
    caps: &[&crate::models::Capability],
) -> Result<DomainProjection> {
    let mut files = BTreeMap::new();
    for (id, text) in read_rules(paths, caps)? {
        files.insert(PathBuf::from(format!("{id}.md")), text.into_bytes());
    }
    Ok(DomainProjection {
        name: "rules",
        kind: CapabilityKind::Rule,
        target_path: paths.user_home.join(".claude/rules"),
        files,
        preserve_names: Vec::new(),
        sensitive: false,
    })
}
fn cursor_rules(
    paths: &AgentHubPaths,
    caps: &[&crate::models::Capability],
) -> Result<DomainProjection> {
    let mut files = BTreeMap::new();
    let manifest = serde_json::json!({"name":"agenthub-rules","version":"0.1.0","description":"AgentHub generated global rules","rules":"./rules"});
    files.insert(
        PathBuf::from(".cursor-plugin/plugin.json"),
        serde_json::to_vec_pretty(&manifest)?,
    );
    for (id, text) in read_rules(paths, caps)? {
        files.insert(PathBuf::from(format!("rules/{id}.mdc")), text.into_bytes());
    }
    Ok(DomainProjection {
        name: "rules",
        kind: CapabilityKind::Rule,
        target_path: paths.user_home.join(".cursor/plugins/local/agenthub-rules"),
        files,
        preserve_names: Vec::new(),
        sensitive: false,
    })
}

fn project_mcp(paths: &AgentHubPaths, target: Target, target_path: &Path) -> Result<Vec<u8>> {
    let mut servers = Map::new();
    for entry in fs::read_dir(&paths.mcp)? {
        let entry = entry?;
        let p = entry.path().join("server.json");
        if !p.is_file() {
            continue;
        }
        let s: McpServer = serde_json::from_slice(&fs::read(&p)?)?;
        let mut v = Map::new();
        match s.transport.as_str() {
            "stdio" => {
                v.insert(
                    "command".into(),
                    Value::String(s.command.context("stdio command")?),
                );
                v.insert("args".into(), serde_json::to_value(s.args)?);
            }
            "http" | "sse" => {
                v.insert("url".into(), Value::String(s.url.context("url")?));
                v.insert("type".into(), Value::String(s.transport));
            }
            _ => anyhow::bail!("unsupported transport"),
        };
        if !s.env.is_empty() {
            v.insert("env".into(), serde_json::to_value(s.env)?);
        }
        if !s.headers.is_empty() {
            v.insert("headers".into(), serde_json::to_value(s.headers)?);
        }
        servers.insert(s.id, Value::Object(v));
    }
    match target {
        Target::Cursor | Target::Claude => {
            let mut root = if target_path.is_file() {
                serde_json::from_slice::<Value>(&fs::read(target_path)?)
                    .unwrap_or(Value::Object(Map::new()))
            } else {
                Value::Object(Map::new())
            };
            let obj = root
                .as_object_mut()
                .context("host MCP config must be an object")?;
            obj.insert("mcpServers".into(), Value::Object(servers));
            Ok(serde_json::to_vec_pretty(&root)?)
        }
        Target::Codex => {
            let original = if target_path.is_file() {
                fs::read_to_string(target_path)?
            } else {
                String::new()
            };
            let mut doc = original
                .parse::<toml_edit::DocumentMut>()
                .unwrap_or_default();
            doc.remove("mcp_servers");
            let mut table = toml_edit::Table::new();
            for (name, value) in servers {
                let mut server = toml_edit::Table::new();
                if let Some(command) = value.get("command").and_then(Value::as_str) {
                    server["command"] = toml_edit::value(command);
                    let mut array = toml_edit::Array::new();
                    for arg in value
                        .get("args")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                    {
                        array.push(arg);
                    }
                    server["args"] = toml_edit::value(array);
                }
                if let Some(url) = value.get("url").and_then(Value::as_str) {
                    server["url"] = toml_edit::value(url);
                }
                table.insert(&name, toml_edit::Item::Table(server));
            }
            doc.insert("mcp_servers", toml_edit::Item::Table(table));
            Ok(doc.to_string().into_bytes())
        }
    }
}

pub fn actual_domain_digest(domain: &DomainProjection) -> Result<String> {
    if domain.target_path.is_file() {
        return Ok(sha256(&fs::read(&domain.target_path)?));
    }
    if !domain.target_path.exists() {
        return Ok(sha256(b"absent"));
    }
    let files = tree_files_filtered(&domain.target_path, &domain.preserve_names)?;
    let mut h = Sha256::new();
    for (p, b) in files {
        h.update(p.to_string_lossy().as_bytes());
        h.update(b);
    }
    Ok(hex::encode(h.finalize()))
}
pub fn actual_files(domain: &DomainProjection) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    if domain.target_path.is_file() {
        return Ok(BTreeMap::from([(
            PathBuf::new(),
            fs::read(&domain.target_path)?,
        )]));
    }
    if !domain.target_path.exists() {
        return Ok(BTreeMap::new());
    }
    tree_files_filtered(&domain.target_path, &domain.preserve_names)
}
fn tree_files_filtered(root: &Path, preserve: &[String]) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut out = BTreeMap::new();
    for e in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !preserve.iter().any(|n| e.file_name() == n.as_str()))
    {
        let e = e?;
        if e.file_type().is_file() {
            out.insert(
                e.path().strip_prefix(root)?.to_path_buf(),
                fs::read(e.path())?,
            );
        }
    }
    Ok(out)
}

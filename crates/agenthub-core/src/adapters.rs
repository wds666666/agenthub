use crate::{
    canonical::{inventory, sha256},
    models::{CapabilityKind, McpServer, SyncMode, SyncSelection, Target},
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
            update_path_digest(&mut h, p);
            h.update(b);
        }
        hex::encode(h.finalize())
    }
}

pub fn projection(paths: &AgentHubPaths, target: Target) -> Result<Vec<DomainProjection>> {
    projection_with_selection(paths, target, None)
}

pub fn projection_with_selection(
    paths: &AgentHubPaths,
    target: Target,
    selection: Option<&SyncSelection>,
) -> Result<Vec<DomainProjection>> {
    if selection.is_none() {
        return projection_with_selection(paths, target, Some(&full_selection(paths, target)?));
    }
    if let Some(scope) = selection {
        let prefix = format!(".{}", target.as_str());
        let mut roots = Vec::new();
        if scope.manages_skills() {
            roots.push(paths.user_home.join(&prefix).join("skills"));
        }
        if target != Target::Agents && scope.manages_mcp() {
            roots.push(paths.user_home.join(match target {
                Target::Codex => ".codex/config.toml",
                Target::Cursor => ".cursor/mcp.json",
                _ => ".claude.json",
            }));
        }
        if target == Target::Cursor && scope.manages_plugins() {
            roots.push(paths.user_home.join(".cursor/plugins/local"));
        }
        if target != Target::Agents && scope.manages_rules() {
            roots.push(paths.user_home.join(match target {
                Target::Codex => ".codex/AGENTS.md",
                Target::Cursor => ".cursor/plugins/local/agenthub-rules/.cursor-plugin/plugin.json",
                _ => ".claude/rules",
            }));
            if target == Target::Cursor {
                roots.push(
                    paths
                        .user_home
                        .join(".cursor/plugins/local/agenthub-rules/rules"),
                );
            }
        }
        for root in roots {
            let mut node = root.as_path();
            while node != paths.user_home {
                match fs::symlink_metadata(node) {
                    Ok(meta) => anyhow::ensure!(
                        !meta.file_type().is_symlink(),
                        "host domain or ancestor is a symlink; refusing to write"
                    ),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                };
                node = node.parent().context("host path outside user home")?;
            }
        }
    }
    let caps = inventory(paths)?;
    let preserve = selection.is_none_or(|s| s.mode == SyncMode::Preserve);
    anyhow::ensure!(
        selection.is_none_or(|s| !s.authoritative),
        "legacy authoritative selection requires scope review"
    );
    let skill_filter = selection.map(|value| &value.skills);
    let plugin_filter = selection.map(|value| &value.plugins);
    let mcp_filter = selection.map(|value| &value.mcp);
    let skill_files = tree_files_selected(&paths.skills, skill_filter)?;
    if target == Target::Agents {
        return Ok(if selection.is_none_or(SyncSelection::manages_skills) {
            vec![DomainProjection {
                name: "skills",
                kind: CapabilityKind::Skill,
                target_path: paths.user_home.join(".agents/skills"),
                files: skill_files,
                preserve_names: preserved_entries(
                    &paths.user_home.join(".agents/skills"),
                    skill_filter,
                    preserve,
                )?,
                sensitive: false,
            }]
        } else {
            Vec::new()
        });
    }
    let plugin_files = plugin_files_selected(&paths.plugins, plugin_filter)?;
    let rule_caps: Vec<_> = caps
        .iter()
        .filter(|c| c.kind == CapabilityKind::Rule)
        .filter(|c| selection.is_none_or(|value| value.rule_ids.contains(&c.id)))
        .collect();
    let (skill_path, mcp_path, plugin_path, rule_projection) = match target {
        Target::Agents => unreachable!("shared Agents target returns before host projection"),
        Target::Cursor => (
            paths.user_home.join(".cursor/skills"),
            paths.user_home.join(".cursor/mcp.json"),
            paths.user_home.join(".cursor/plugins/local"),
            if selection.is_none_or(SyncSelection::manages_rules) {
                cursor_rules(paths, &rule_caps)?
            } else {
                DomainProjection {
                    name: "rules",
                    kind: CapabilityKind::Rule,
                    target_path: paths
                        .user_home
                        .join(".cursor/plugins/local/agenthub-rules/rules"),
                    files: BTreeMap::new(),
                    preserve_names: Vec::new(),
                    sensitive: false,
                }
            },
        ),
        Target::Codex => (
            paths.user_home.join(".codex/skills"),
            paths.user_home.join(".codex/config.toml"),
            paths.user_home.join(".codex/plugins"),
            if selection.is_none_or(SyncSelection::manages_rules) {
                codex_rules(paths, &rule_caps, preserve)?
            } else {
                DomainProjection {
                    name: "rules",
                    kind: CapabilityKind::Rule,
                    target_path: paths.user_home.join(".codex/AGENTS.md"),
                    files: BTreeMap::new(),
                    preserve_names: Vec::new(),
                    sensitive: false,
                }
            },
        ),
        Target::Claude => (
            paths.user_home.join(".claude/skills"),
            paths.user_home.join(".claude.json"),
            paths.user_home.join(".claude/plugins"),
            claude_rules(paths, &rule_caps)?,
        ),
    };
    let target_plugins = plugin_files;
    let mcp = if selection.is_none_or(SyncSelection::manages_mcp) {
        project_mcp(paths, target, &mcp_path, mcp_filter, preserve)?
    } else {
        Vec::new()
    };
    let mut skill_preserve = if selection.is_none_or(SyncSelection::manages_skills) {
        preserved_entries(&skill_path, skill_filter, preserve)?
    } else {
        Vec::new()
    };
    if target == Target::Claude {
        skill_preserve.extend([".synced".into(), "synced".into(), ".trash".into()]);
    }
    let mut plugin_preserve =
        if target == Target::Cursor && selection.is_none_or(SyncSelection::manages_plugins) {
            preserved_entries(&plugin_path, plugin_filter, preserve)?
        } else {
            Vec::new()
        };
    plugin_preserve.push("agenthub-rules".into());
    let mut rule_projection = rule_projection;
    if target != Target::Codex && preserve && selection.is_none_or(SyncSelection::manages_rules) {
        let names: Vec<_> = rule_projection
            .files
            .keys()
            .filter_map(|p| {
                p.components()
                    .next()
                    .map(|n| n.as_os_str().to_string_lossy().into_owned())
            })
            .collect();
        rule_projection.preserve_names =
            preserved_entries(&rule_projection.target_path, Some(&names), true)?;
    }
    let mut out = Vec::new();
    if selection.is_none_or(SyncSelection::manages_skills) {
        out.push(DomainProjection {
            name: "skills",
            kind: CapabilityKind::Skill,
            target_path: skill_path,
            files: skill_files,
            preserve_names: skill_preserve,
            sensitive: false,
        });
    }
    if selection.is_none_or(SyncSelection::manages_mcp) {
        out.push(DomainProjection {
            name: "mcp",
            kind: CapabilityKind::Mcp,
            target_path: mcp_path,
            files: BTreeMap::from([(PathBuf::new(), mcp)]),
            preserve_names: Vec::new(),
            sensitive: true,
        });
    }
    if target == Target::Cursor && selection.is_none_or(|value| value.manages_plugins()) {
        out.push(DomainProjection {
            name: "plugins",
            kind: CapabilityKind::Plugin,
            target_path: plugin_path,
            files: target_plugins,
            preserve_names: plugin_preserve,
            sensitive: false,
        });
    }
    if selection.is_none_or(SyncSelection::manages_rules) && (!preserve || !rule_caps.is_empty()) {
        if target == Target::Cursor {
            let manifest = serde_json::json!({"name":"agenthub-rules","version":"0.1.0","description":"AgentHub generated global rules","rules":"./rules"});
            out.push(DomainProjection {
                name: "rules-container",
                kind: CapabilityKind::Plugin,
                target_path: paths
                    .user_home
                    .join(".cursor/plugins/local/agenthub-rules/.cursor-plugin/plugin.json"),
                files: BTreeMap::from([(PathBuf::new(), serde_json::to_vec_pretty(&manifest)?)]),
                preserve_names: Vec::new(),
                sensitive: false,
            });
        }
        out.push(rule_projection);
    }
    Ok(out)
}

pub fn full_selection(paths: &AgentHubPaths, target: Target) -> Result<SyncSelection> {
    let caps = inventory(paths)?;
    let ids = |kind| {
        caps.iter()
            .filter(|c| c.kind == kind)
            .map(|c| c.id.clone())
            .collect()
    };
    Ok(SyncSelection {
        skills_managed: true,
        skills: ids(CapabilityKind::Skill),
        mcp_managed: target != Target::Agents,
        mcp: if target == Target::Agents {
            Vec::new()
        } else {
            ids(CapabilityKind::Mcp)
        },
        plugins_managed: target != Target::Agents,
        plugins: if target != Target::Agents {
            ids(CapabilityKind::Plugin)
        } else {
            Vec::new()
        },
        rules_managed: target != Target::Agents,
        rule_ids: if target == Target::Agents {
            Vec::new()
        } else {
            ids(CapabilityKind::Rule)
        },
        ..Default::default()
    })
}

fn preserved_entries(
    root: &Path,
    selected: Option<&Vec<String>>,
    preserve: bool,
) -> Result<Vec<String>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    anyhow::ensure!(
        fs::symlink_metadata(root)?.file_type().is_dir(),
        "host capability root is not an ordinary directory"
    );
    let mut out = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || (preserve && selected.is_some_and(|ids| !ids.contains(&name))) {
            out.push(name);
        }
    }
    Ok(out)
}

fn plugin_files_selected(
    root: &Path,
    selected: Option<&Vec<String>>,
) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut out = BTreeMap::new();
    if !root.is_dir() {
        return Ok(out);
    }
    for plugin in fs::read_dir(root)? {
        let plugin = plugin?;
        if !plugin.file_type()?.is_dir() {
            continue;
        }
        let id = plugin.file_name().to_string_lossy().into_owned();
        if selected.is_some_and(|ids| !ids.contains(&id)) {
            continue;
        }
        let payload = plugin.path().join("payload");
        if !payload.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&payload).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_symlink() {
                anyhow::bail!(
                    "symlink is not allowed in plugin payload: {}",
                    entry.path().display()
                );
            }
            if entry.file_type().is_file() {
                out.insert(
                    PathBuf::from(&id).join(entry.path().strip_prefix(&payload)?),
                    fs::read(entry.path())?,
                );
            }
        }
    }
    Ok(out)
}
fn tree_files_selected(
    root: &Path,
    selected: Option<&Vec<String>>,
) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut out = BTreeMap::new();
    if !root.exists() {
        return Ok(out);
    }
    for e in WalkDir::new(root).follow_links(false) {
        let e = e?;
        if let Some(selected) = selected {
            let relative = e.path().strip_prefix(root)?;
            if let Some(first) = relative.components().next() {
                let id = first.as_os_str().to_string_lossy();
                if !selected.iter().any(|value| value == id.as_ref()) {
                    if e.file_type().is_dir() {
                        continue;
                    }
                    continue;
                }
            }
        }
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
    paths: &AgentHubPaths,
    caps: &[&crate::models::Capability],
    target: Target,
) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for c in caps {
        let p = c.path.join("rule.md");
        if p.is_file() {
            let rule = crate::canonical::read_rule(paths, &c.id)?;
            anyhow::ensure!(
                rule.targets.contains(&target),
                "selected rule is not compatible with target"
            );
            out.push((
                c.id.clone(),
                crate::rule_projection::project(&rule, target)?,
            ));
        }
    }
    Ok(out)
}
fn codex_rules(
    paths: &AgentHubPaths,
    caps: &[&crate::models::Capability],
    preserve: bool,
) -> Result<DomainProjection> {
    let path = paths.user_home.join(".codex/AGENTS.md");
    let original = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };
    let body = crate::rule_projection::render(
        &original,
        &read_rules(paths, caps, Target::Codex)?,
        preserve,
        paths,
    )?;
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
    for (id, text) in read_rules(paths, caps, Target::Claude)? {
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
    let manifest_path = paths
        .user_home
        .join(".cursor/plugins/local/agenthub-rules/.cursor-plugin/plugin.json");
    if manifest_path.exists() {
        let existing: Value = serde_json::from_slice(&fs::read(&manifest_path)?)
            .context("invalid generated rules container")?;
        anyhow::ensure!(
            existing.get("description").and_then(Value::as_str)
                == Some("AgentHub generated global rules"),
            "unknown agenthub-rules plugin; refusing to overwrite"
        );
    }
    let mut files = BTreeMap::new();
    for (id, text) in read_rules(paths, caps, Target::Cursor)? {
        files.insert(PathBuf::from(format!("{id}.mdc")), text.into_bytes());
    }
    Ok(DomainProjection {
        name: "rules",
        kind: CapabilityKind::Rule,
        target_path: paths
            .user_home
            .join(".cursor/plugins/local/agenthub-rules/rules"),
        files,
        preserve_names: Vec::new(),
        sensitive: false,
    })
}

fn project_mcp(
    paths: &AgentHubPaths,
    target: Target,
    target_path: &Path,
    selected: Option<&Vec<String>>,
    preserve: bool,
) -> Result<Vec<u8>> {
    let mut servers = Map::new();
    for entry in fs::read_dir(&paths.mcp)? {
        let entry = entry?;
        let p = entry.path().join("server.json");
        if !p.is_file() {
            continue;
        }
        let s: McpServer = serde_json::from_slice(&fs::read(&p)?)?;
        if let Some(selected) = selected {
            if !selected.iter().any(|id| id == &s.id) {
                continue;
            }
        }
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
        Target::Agents => anyhow::bail!("shared Agents target does not support MCP"),
        Target::Cursor | Target::Claude => {
            let mut root = if target_path.is_file() {
                serde_json::from_slice::<Value>(&fs::read(target_path)?)
                    .context("invalid host MCP JSON; refusing to overwrite")?
            } else {
                Value::Object(Map::new())
            };
            let obj = root
                .as_object_mut()
                .context("host MCP config must be an object")?;
            if preserve {
                let mut existing = match obj.get("mcpServers") {
                    Some(v) => v.as_object().context("invalid mcpServers object")?.clone(),
                    None => Map::new(),
                };
                existing.extend(servers);
                servers = existing;
            }
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
                .context("invalid host MCP TOML; refusing to overwrite")?;
            let mut table = if preserve {
                match doc.get("mcp_servers") {
                    Some(v) => v.as_table().context("invalid mcp_servers table")?.clone(),
                    None => toml_edit::Table::new(),
                }
            } else {
                toml_edit::Table::new()
            };
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
                if value.get("command").is_some() {
                    if let Some(env) = value.get("env").and_then(Value::as_object) {
                        let mut env_table = toml_edit::InlineTable::new();
                        for (key, value) in env {
                            if let Some(value) = value.as_str() {
                                env_table.insert(key, toml_edit::Value::from(value));
                            }
                        }
                        if !env_table.is_empty() {
                            server["env"] = toml_edit::value(env_table);
                        }
                    }
                }
                if value.get("url").is_some() {
                    if let Some(headers) = value.get("headers").and_then(Value::as_object) {
                        let mut header_table = toml_edit::InlineTable::new();
                        for (key, value) in headers {
                            if let Some(value) = value.as_str() {
                                header_table.insert(key, toml_edit::Value::from(value));
                            }
                        }
                        if !header_table.is_empty() {
                            server["http_headers"] = toml_edit::value(header_table);
                        }
                    }
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
        update_path_digest(&mut h, &p);
        h.update(b);
    }
    Ok(hex::encode(h.finalize()))
}

fn update_path_digest(digest: &mut Sha256, path: &Path) {
    for (index, component) in path.components().enumerate() {
        if index > 0 {
            digest.update(b"/");
        }
        digest.update(component.as_os_str().to_string_lossy().as_bytes());
    }
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
        .filter_entry(|e| e.depth() != 1 || !preserve.iter().any(|n| e.file_name() == n.as_str()))
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

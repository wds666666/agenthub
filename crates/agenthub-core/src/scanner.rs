use crate::{
    canonical::{sha256, tree_digest, valid_id},
    models::{CapabilityKind, ScanItem, Target},
    paths::AgentHubPaths,
};
use anyhow::Result;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn scan_global(paths: &AgentHubPaths, targets: &[Target]) -> Result<Vec<ScanItem>> {
    let wanted: BTreeSet<_> = targets.iter().copied().collect();
    let mut items = Vec::new();
    let mut skill_roots = Vec::new();
    if wanted.contains(&Target::Agents) {
        skill_roots.push(("agents", paths.user_home.join(".agents/skills")));
    }
    if wanted.contains(&Target::Cursor) {
        skill_roots.push(("cursor", paths.user_home.join(".cursor/skills")));
        skill_roots.push(("agents", paths.user_home.join(".agents/skills")));
    }
    if wanted.contains(&Target::Codex) {
        skill_roots.push(("codex", paths.user_home.join(".codex/skills")));
        skill_roots.push(("agents", paths.user_home.join(".agents/skills")));
    }
    if wanted.contains(&Target::Claude) {
        skill_roots.push(("claude", paths.user_home.join(".claude/skills")));
    }
    let mut seen = BTreeSet::new();
    for (source, root) in skill_roots {
        if !seen.insert(root.clone()) {
            continue;
        }
        collect_skills(&root, source, &mut items)?;
    }
    if wanted.contains(&Target::Cursor) {
        collect_mcp_json(
            &paths.user_home.join(".cursor/mcp.json"),
            "cursor",
            &mut items,
        )?;
        collect_plugins(
            &paths.user_home.join(".cursor/plugins/local"),
            "cursor",
            &mut items,
        )?;
    }
    if wanted.contains(&Target::Codex) {
        collect_mcp_toml(
            &paths.user_home.join(".codex/config.toml"),
            "codex",
            &mut items,
        )?;
        collect_plugins(&paths.user_home.join(".codex/plugins"), "codex", &mut items)?;
        let p = paths.user_home.join(".codex/AGENTS.md");
        if p.is_file() {
            items.push(rule_item("codex", p)?);
        }
    }
    if wanted.contains(&Target::Claude) {
        collect_mcp_json(&paths.user_home.join(".claude.json"), "claude", &mut items)?;
        collect_plugins(
            &paths.user_home.join(".claude/plugins"),
            "claude",
            &mut items,
        )?;
        collect_markdown(&paths.user_home.join(".claude/rules"), "claude", &mut items)?;
    }
    items.sort_by(|a, b| {
        (&a.source, a.kind, a.path.as_os_str()).cmp(&(&b.source, b.kind, b.path.as_os_str()))
    });
    Ok(items)
}

fn collect_skills(root: &Path, source: &str, out: &mut Vec<ScanItem>) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for e in WalkDir::new(root)
        .min_depth(1)
        .max_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0 || !entry.file_name().to_string_lossy().starts_with('.')
        })
    {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        if e.file_type().is_dir() && valid_id(&name) && e.path().join("SKILL.md").is_file() {
            out.push(item(
                CapabilityKind::Skill,
                source,
                e.path().to_path_buf(),
                None,
            )?);
        }
    }
    Ok(())
}
fn collect_plugins(root: &Path, source: &str, out: &mut Vec<ScanItem>) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for e in fs::read_dir(root)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        if !valid_id(&name)
            || name.starts_with('.')
            || matches!(
                name.as_str(),
                "cache" | "staging" | "marketplaces" | "managed"
            )
            || !e.file_type()?.is_dir()
        {
            continue;
        }
        let path = e.path();
        let recognized = path.join("plugin.json").is_file()
            || path.join(".cursor-plugin/plugin.json").is_file()
            || path.join(".claude-plugin/plugin.json").is_file();
        if recognized {
            out.push(item(CapabilityKind::Plugin, source, path, None)?);
        }
    }
    Ok(())
}
fn collect_markdown(root: &Path, source: &str, out: &mut Vec<ScanItem>) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for e in WalkDir::new(root).max_depth(4).follow_links(false) {
        let e = e?;
        if e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "md") {
            out.push(rule_item(source, e.path().to_path_buf())?);
        }
    }
    Ok(())
}
fn collect_mcp_json(path: &Path, source: &str, out: &mut Vec<ScanItem>) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    if let Some(map) = value.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, server) in map {
            out.push(mcp_item(
                source,
                path.to_path_buf(),
                name,
                sha256(&serde_json::to_vec(server)?),
            ));
        }
    }
    Ok(())
}
fn collect_mcp_toml(path: &Path, source: &str, out: &mut Vec<ScanItem>) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let value: toml::Value = fs::read_to_string(path)?.parse()?;
    if let Some(map) = value.get("mcp_servers").and_then(|v| v.as_table()) {
        for (name, server) in map {
            out.push(mcp_item(
                source,
                path.to_path_buf(),
                name,
                sha256(&serde_json::to_vec(server)?),
            ));
        }
    }
    Ok(())
}

fn mcp_item(source: &str, path: PathBuf, name: &str, digest: String) -> ScanItem {
    let source_key = format!("server:{name}");
    let id = sha256(
        format!(
            "{}:{}:{}:{}:{}",
            source,
            CapabilityKind::Mcp.as_str(),
            path.display(),
            source_key,
            digest
        )
        .as_bytes(),
    );
    ScanItem {
        id,
        kind: CapabilityKind::Mcp,
        source: source.into(),
        digest,
        path,
        selected: false,
        importable: true,
        source_key: Some(source_key),
        warning: None,
    }
}
fn item(
    kind: CapabilityKind,
    source: &str,
    path: PathBuf,
    source_key: Option<String>,
) -> Result<ScanItem> {
    let digest = tree_digest(&path)?;
    let id = sha256(
        format!(
            "{}:{}:{}:{}:{}",
            source,
            kind.as_str(),
            path.display(),
            source_key.as_deref().unwrap_or(""),
            digest
        )
        .as_bytes(),
    );
    Ok(ScanItem {
        id,
        kind,
        source: source.into(),
        digest,
        path,
        selected: false,
        importable: true,
        source_key,
        warning: None,
    })
}

fn rule_item(source: &str, path: PathBuf) -> Result<ScanItem> {
    let mut result = item(CapabilityKind::Rule, source, path.clone(), None)?;
    let metadata = fs::metadata(&path)?;
    if metadata.len() > 1_000_000 {
        result.importable = false;
        result.warning = Some("rule_too_large".into());
        return Ok(result);
    }
    let bytes = fs::read(&path)?;
    match std::str::from_utf8(&bytes) {
        Ok(body) if body.trim().is_empty() => {
            result.importable = false;
            result.warning = Some("rule_empty".into());
        }
        Ok(_) => {}
        Err(_) => {
            result.importable = false;
            result.warning = Some("rule_invalid_encoding".into());
        }
    }
    Ok(result)
}

/// The entire allowlist. Kept public so tests can prove that cwd/project paths cannot enter a scan.
pub fn allowed_roots(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".agents/skills"),
        home.join(".cursor/skills"),
        home.join(".cursor/mcp.json"),
        home.join(".cursor/plugins/local"),
        home.join(".codex/skills"),
        home.join(".codex/config.toml"),
        home.join(".codex/plugins"),
        home.join(".codex/AGENTS.md"),
        home.join(".claude/skills"),
        home.join(".claude/rules"),
        home.join(".claude/plugins"),
        home.join(".claude.json"),
    ]
}

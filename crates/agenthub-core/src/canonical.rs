use crate::models::ScanItem;
use crate::{
    models::{
        Capability, CapabilityDetail, CapabilityFile, CapabilityKind, McpServer, RuleDocument,
        Target,
    },
    paths::AgentHubPaths,
};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn file_digest(path: &Path) -> Result<String> {
    Ok(sha256(
        &fs::read(path).with_context(|| format!("read {}", path.display()))?,
    ))
}

pub fn tree_digest(path: &Path) -> Result<String> {
    if !path.exists() {
        return Ok(sha256(b"absent"));
    }
    if path.is_file() {
        return file_digest(path);
    }
    let mut entries: Vec<_> = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .collect();
    entries.sort_by_key(|e| e.path().to_path_buf());
    let mut hasher = Sha256::new();
    for entry in entries {
        let rel = entry.path().strip_prefix(path)?;
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update(fs::read(entry.path())?);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn inventory(paths: &AgentHubPaths) -> Result<Vec<Capability>> {
    let mut out = Vec::new();
    collect_dirs(&paths.skills, CapabilityKind::Skill, &mut out)?;
    collect_dirs(&paths.plugins, CapabilityKind::Plugin, &mut out)?;
    collect_dirs(&paths.rules, CapabilityKind::Rule, &mut out)?;
    collect_dirs(&paths.mcp, CapabilityKind::Mcp, &mut out)?;
    out.sort_by(|a, b| (a.kind, a.id.as_str()).cmp(&(b.kind, b.id.as_str())));
    Ok(out)
}

fn collect_dirs(root: &Path, kind: CapabilityKind, out: &mut Vec<Capability>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        if !valid_id(&id) {
            continue;
        }
        let path = entry.path();
        let display_name = match kind {
            CapabilityKind::Mcp => {
                let server: McpServer =
                    serde_json::from_slice(&fs::read(path.join("server.json"))?)?;
                validate_mcp(&server)?;
                server.display_name
            }
            CapabilityKind::Rule => read_rule_dir(&path)?.display_name,
            _ => id.clone(),
        };
        out.push(Capability {
            id: id.clone(),
            kind,
            display_name,
            digest: tree_digest(&path)?,
            path,
            compatible_targets: if kind == CapabilityKind::Skill {
                Target::ALL.to_vec()
            } else {
                Target::HOSTS.to_vec()
            },
        });
    }
    Ok(())
}

pub fn read_rule(paths: &AgentHubPaths, id: &str) -> Result<RuleDocument> {
    anyhow::ensure!(valid_id(id), "invalid rule id");
    let dir = paths.rules.join(id);
    anyhow::ensure!(dir.is_dir(), "rule not found");
    read_rule_dir(&dir)
}

pub fn read_capability_detail(
    paths: &AgentHubPaths,
    kind: CapabilityKind,
    id: &str,
) -> Result<CapabilityDetail> {
    anyhow::ensure!(valid_id(id), "invalid capability id");
    let capability = inventory(paths)?
        .into_iter()
        .find(|item| item.kind == kind && item.id == id)
        .context("capability not found")?;
    let primary = match kind {
        CapabilityKind::Skill => "SKILL.md",
        CapabilityKind::Mcp => "server.json",
        CapabilityKind::Plugin => "agenthub.plugin.json",
        CapabilityKind::Rule => "rule.md",
    };
    let primary_path = capability.path.join(primary);
    anyhow::ensure!(primary_path.is_file(), "capability preview is unavailable");
    let bytes = fs::read(&primary_path)?;
    const PREVIEW_LIMIT: usize = 512 * 1024;
    let preview_truncated = bytes.len() > PREVIEW_LIMIT;
    let preview = String::from_utf8_lossy(&bytes[..bytes.len().min(PREVIEW_LIMIT)]).into_owned();
    let preview = if matches!(kind, CapabilityKind::Mcp | CapabilityKind::Plugin) {
        redact_json_preview(&preview)
    } else {
        preview
    };
    let mut files = Vec::new();
    for entry in WalkDir::new(&capability.path).follow_links(false) {
        let entry = entry?;
        anyhow::ensure!(
            !entry.file_type().is_symlink(),
            "canonical symlink is not previewable"
        );
        if entry.file_type().is_file() {
            files.push(CapabilityFile {
                path: entry
                    .path()
                    .strip_prefix(&capability.path)?
                    .to_string_lossy()
                    .into_owned(),
                size: entry.metadata()?.len(),
            });
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(CapabilityDetail {
        capability,
        preview_path: primary.into(),
        preview,
        preview_truncated,
        files,
    })
}

pub fn delete_capability(paths: &AgentHubPaths, kind: CapabilityKind, id: &str) -> Result<()> {
    anyhow::ensure!(valid_id(id), "invalid capability id");
    let root = match kind {
        CapabilityKind::Skill => &paths.skills,
        CapabilityKind::Mcp => &paths.mcp,
        CapabilityKind::Plugin => &paths.plugins,
        CapabilityKind::Rule => &paths.rules,
    };
    let target = root.join(id);
    anyhow::ensure!(target.is_dir(), "capability not found");
    paths.assert_inside_root(&target)?;
    fs::remove_dir_all(target)?;
    Ok(())
}

fn redact_json_preview(preview: &str) -> String {
    fn visit(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                for (key, value) in object {
                    let normalized = key.to_ascii_lowercase().replace('_', "-");
                    if ["token", "secret", "password", "authorization", "api-key"]
                        .iter()
                        .any(|sensitive| normalized.contains(sensitive))
                    {
                        *value = serde_json::Value::String("[REDACTED]".into());
                    } else {
                        visit(value);
                    }
                }
            }
            serde_json::Value::Array(values) => values.iter_mut().for_each(visit),
            _ => {}
        }
    }
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(preview) else {
        return crate::secrets::redact(preview);
    };
    visit(&mut value);
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "[preview unavailable]".into())
}

fn read_rule_dir(dir: &Path) -> Result<RuleDocument> {
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("rule.json"))
            .with_context(|| format!("read rule manifest in {}", dir.display()))?,
    )?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1) as u32;
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .or_else(|| dir.file_name().and_then(|name| name.to_str()))
        .context("rule id is missing")?
        .to_string();
    let display_name = value
        .get("displayName")
        .or_else(|| value.get("display_name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&id)
        .to_string();
    let activation = value
        .get("activation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("always")
        .to_string();
    let paths = serde_json::from_value(
        value
            .get("paths")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    )?;
    let targets = serde_json::from_value(
        value
            .get("targets")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(["cursor", "codex", "claude"])),
    )?;
    let rule = RuleDocument {
        schema_version,
        id,
        display_name,
        activation,
        paths,
        targets,
        body: fs::read_to_string(dir.join("rule.md"))?,
    };
    validate_rule(&rule)?;
    Ok(rule)
}

pub fn validate_rule(rule: &RuleDocument) -> Result<()> {
    anyhow::ensure!(rule.schema_version == 1, "unsupported Rule schema");
    anyhow::ensure!(valid_id(&rule.id), "rule id must be a lowercase slug");
    anyhow::ensure!(
        !rule.display_name.trim().is_empty(),
        "rule display name is required"
    );
    anyhow::ensure!(
        rule.display_name.chars().count() <= 120,
        "rule display name is too long"
    );
    anyhow::ensure!(
        matches!(rule.activation.as_str(), "always" | "manual" | "paths"),
        "unsupported rule activation"
    );
    anyhow::ensure!(!rule.body.trim().is_empty(), "rule body is required");
    anyhow::ensure!(rule.body.len() <= 1_000_000, "rule body is too large");
    anyhow::ensure!(!rule.targets.is_empty(), "at least one target is required");
    anyhow::ensure!(
        rule.targets
            .iter()
            .all(|target| Target::HOSTS.contains(target)),
        "Rules are not supported by the shared Agents target"
    );
    for path in &rule.paths {
        anyhow::ensure!(
            !path.trim().is_empty() && path.len() <= 500,
            "invalid rule path"
        );
        anyhow::ensure!(
            !Path::new(path).is_absolute(),
            "absolute rule path rejected"
        );
        anyhow::ensure!(
            !Path::new(path)
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir)),
            "rule path traversal rejected"
        );
    }
    if rule.activation == "paths" {
        anyhow::ensure!(
            !rule.paths.is_empty(),
            "path activation requires at least one path"
        );
    }
    Ok(())
}

pub fn save_rule(paths: &AgentHubPaths, rule: &RuleDocument, create: bool) -> Result<Capability> {
    validate_rule(rule)?;
    let destination = paths.rules.join(&rule.id);
    if create {
        anyhow::ensure!(!destination.exists(), "a rule with this id already exists");
    } else {
        anyhow::ensure!(destination.is_dir(), "rule not found");
    }

    let stage = paths
        .rules
        .join(format!(".rule-{}-staging", uuid::Uuid::new_v4()));
    fs::create_dir_all(&stage)?;
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": rule.id,
        "displayName": rule.display_name.trim(),
        "activation": rule.activation,
        "paths": rule.paths,
        "targets": rule.targets,
    });
    let write_result = (|| -> Result<()> {
        fs::write(stage.join("rule.md"), &rule.body)?;
        fs::write(
            stage.join("rule.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        crate::paths::set_private_file(&stage.join("rule.md"))?;
        crate::paths::set_private_file(&stage.join("rule.json"))?;
        Ok(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }

    let old = paths
        .rules
        .join(format!(".rule-{}-old", uuid::Uuid::new_v4()));
    if destination.exists() {
        fs::rename(&destination, &old)?;
    }
    if let Err(error) = fs::rename(&stage, &destination) {
        if old.exists() {
            let _ = fs::rename(&old, &destination);
        }
        let _ = fs::remove_dir_all(&stage);
        return Err(error.into());
    }
    if old.exists() {
        fs::remove_dir_all(old)?;
    }
    inventory(paths)?
        .into_iter()
        .find(|item| item.kind == CapabilityKind::Rule && item.id == rule.id)
        .context("saved rule missing from inventory")
}
pub fn canonical_digest(paths: &AgentHubPaths) -> Result<String> {
    let mut hasher = Sha256::new();
    for c in inventory(paths)? {
        hasher.update(c.kind.as_str());
        hasher.update(c.id);
        hasher.update(c.digest);
    }
    hasher.update(fs::read(paths.root.join("agenthub.toml"))?);
    Ok(hex::encode(hasher.finalize()))
}
pub fn valid_id(id: &str) -> bool {
    let reserved = matches!(
        id,
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    );
    !reserved
        && !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}
pub fn validate_mcp(s: &McpServer) -> Result<()> {
    anyhow::ensure!(s.schema_version == 1, "unsupported MCP schema");
    anyhow::ensure!(valid_id(&s.id), "invalid MCP id");
    match s.transport.as_str() {
        "stdio" => anyhow::ensure!(
            s.command.as_ref().is_some_and(|v| !v.is_empty()),
            "stdio requires command"
        ),
        "http" | "sse" => anyhow::ensure!(
            s.url
                .as_ref()
                .is_some_and(|v| v.starts_with("http://") || v.starts_with("https://")),
            "network transport requires URL"
        ),
        _ => anyhow::bail!("unsupported MCP transport"),
    };
    Ok(())
}

pub fn import_scan_items(paths: &AgentHubPaths, items: &[ScanItem]) -> Result<Vec<String>> {
    let mut imported = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for item in items.iter().filter(|i| i.selected) {
        anyhow::ensure!(
            item.importable,
            "{} {} is not importable: {}",
            item.kind.as_str(),
            item.path.display(),
            scan_warning_message(item.warning.as_deref())
        );
        if !seen.insert((item.kind, item.digest.clone())) {
            continue;
        }
        let base = item
            .source_key
            .as_deref()
            .and_then(|key| key.strip_prefix("server:"))
            .or_else(|| item.path.file_stem().and_then(|s| s.to_str()))
            .unwrap_or(item.kind.as_str());
        let id = unique_id(paths, item.kind, &slug(base));
        (|| -> Result<()> {
            match item.kind {
                CapabilityKind::Skill => copy_checked(&item.path, &paths.skills.join(&id))?,
                CapabilityKind::Plugin => {
                    let dest = paths.plugins.join(&id);
                    let (format, source_manifest, components) = inspect_plugin(&item.path)?;
                    fs::create_dir_all(dest.join("payload"))?;
                    copy_checked(&item.path, &dest.join("payload"))?;
                    let manifest = serde_json::json!({"schemaVersion":1,"id":id,"displayName":base,"sourceFormat":format,"sourceManifest":source_manifest,"components":components});
                    fs::write(
                        dest.join("agenthub.plugin.json"),
                        serde_json::to_vec_pretty(&manifest)?,
                    )?;
                }
                CapabilityKind::Rule => {
                    let dest = paths.rules.join(&id);
                    fs::create_dir_all(&dest)?;
                    let body = if item.path.is_file() {
                        fs::read(&item.path)?
                    } else {
                        Vec::new()
                    };
                    fs::write(dest.join("rule.md"), body)?;
                    fs::write(
                        dest.join("rule.json"),
                        serde_json::to_vec_pretty(
                            &serde_json::json!({"schemaVersion":1,"id":id,"activation":"always","paths":[],"targets":["cursor","codex","claude"]}),
                        )?,
                    )?;
                }
                CapabilityKind::Mcp => import_mcp(paths, item, &id)?,
            }
            Ok(())
        })()
        .with_context(|| {
            format!(
                "import {} from {}",
                item.kind.as_str(),
                item.path.display()
            )
        })?;
        imported.push(id);
    }
    Ok(imported)
}

pub fn import_initial_atomic(paths: &AgentHubPaths, items: &[ScanItem]) -> Result<Vec<String>> {
    anyhow::ensure!(
        canonical_dirs_empty(paths)?,
        "incomplete Canonical import exists; discard it before retrying"
    );
    let stage_root = paths
        .root
        .join("runtime")
        .join(format!("import-{}", uuid::Uuid::new_v4()));
    let stage = AgentHubPaths::new(paths.user_home.clone(), stage_root.clone());
    for dir in [&stage.skills, &stage.plugins, &stage.rules, &stage.mcp] {
        fs::create_dir_all(dir)?;
    }
    let result = (|| -> Result<Vec<String>> {
        let imported = import_scan_items(&stage, items)?;
        inventory(&stage).context("staged Canonical validation failed")?;
        replace_canonical_dirs(paths, &stage, &stage_root)?;
        Ok(imported)
    })();
    let _ = fs::remove_dir_all(&stage_root);
    result
}

pub fn import_scan_items_atomic(paths: &AgentHubPaths, items: &[ScanItem]) -> Result<Vec<String>> {
    let stage_root = paths
        .root
        .join("runtime")
        .join(format!("reverse-import-{}", uuid::Uuid::new_v4()));
    let stage = AgentHubPaths::new(paths.user_home.clone(), stage_root.clone());
    fs::create_dir_all(&stage_root)?;
    for (source, destination) in [
        (&paths.skills, &stage.skills),
        (&paths.plugins, &stage.plugins),
        (&paths.rules, &stage.rules),
        (&paths.mcp, &stage.mcp),
    ] {
        if source.exists() {
            copy_checked(source, destination)?;
        } else {
            fs::create_dir_all(destination)?;
        }
    }
    let result = (|| -> Result<Vec<String>> {
        let imported = import_scan_items(&stage, items)?;
        inventory(&stage).context("reverse-import Canonical validation failed")?;
        replace_canonical_dirs(paths, &stage, &stage_root)?;
        Ok(imported)
    })();
    let _ = fs::remove_dir_all(&stage_root);
    result
}

fn replace_canonical_dirs(
    paths: &AgentHubPaths,
    stage: &AgentHubPaths,
    stage_root: &Path,
) -> Result<()> {
    let originals = stage_root.join("originals");
    fs::create_dir_all(&originals)?;
    let pairs = [
        ("skills", &stage.skills, &paths.skills),
        ("plugins", &stage.plugins, &paths.plugins),
        ("rules", &stage.rules, &paths.rules),
        ("mcp", &stage.mcp, &paths.mcp),
    ];
    for (name, _, target) in &pairs {
        if target.exists() {
            fs::rename(target, originals.join(name))?;
        }
    }
    let mut moved = Vec::new();
    for (name, source, target) in &pairs {
        if let Err(error) = fs::rename(source, target) {
            for restored_target in moved.iter().rev() {
                let _ = fs::remove_dir_all(restored_target);
            }
            for (original_name, _, original_target) in &pairs {
                let backup = originals.join(original_name);
                if backup.exists() {
                    let _ = fs::rename(backup, original_target);
                }
            }
            return Err(error).with_context(|| format!("activate staged {name}"));
        }
        moved.push((*target).clone());
    }
    Ok(())
}

pub fn discard_incomplete_import(paths: &AgentHubPaths) -> Result<()> {
    for dir in [
        &paths.skills,
        &paths.plugins,
        &paths.rules,
        &paths.mcp,
        &paths.root.join("runtime"),
    ] {
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
    }
    paths.ensure_runtime()
}

pub fn canonical_dirs_empty(paths: &AgentHubPaths) -> Result<bool> {
    for dir in [&paths.skills, &paths.plugins, &paths.rules, &paths.mcp] {
        if dir.is_dir() && fs::read_dir(dir)?.next().is_some() {
            return Ok(false);
        }
    }
    Ok(true)
}
fn slug(value: &str) -> String {
    let s: String = value
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    let s = if s.is_empty() {
        "imported".to_string()
    } else {
        s
    };
    if valid_id(&s) {
        s
    } else {
        format!("{s}-item")
    }
}

fn scan_warning_message(code: Option<&str>) -> &'static str {
    match code {
        Some("rule_empty") => "rule file is empty",
        Some("rule_too_large") => "rule file is larger than 1 MB",
        Some("rule_invalid_encoding") => "rule file must use UTF-8 encoding",
        _ => "validation failed",
    }
}
fn unique_id(paths: &AgentHubPaths, kind: CapabilityKind, base: &str) -> String {
    let root = match kind {
        CapabilityKind::Skill => &paths.skills,
        CapabilityKind::Plugin => &paths.plugins,
        CapabilityKind::Rule => &paths.rules,
        CapabilityKind::Mcp => &paths.mcp,
    };
    if !root.join(base).exists() {
        return base.into();
    }
    for i in 2..1000 {
        let v = format!("{base}-{i}");
        if !root.join(&v).exists() {
            return v;
        }
    }
    format!("{base}-overflow")
}
fn copy_checked(src: &Path, dst: &Path) -> Result<()> {
    if src.is_file() {
        fs::create_dir_all(dst)?;
        fs::copy(src, dst.join(src.file_name().context("filename")?))?;
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for e in WalkDir::new(src).follow_links(false) {
        let e = e?;
        let rel = e.path().strip_prefix(src)?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if e.file_type().is_symlink() {
            let link = fs::read_link(e.path())?;
            let resolved = e
                .path()
                .parent()
                .unwrap_or(src)
                .join(&link)
                .canonicalize()?;
            anyhow::ensure!(
                resolved.starts_with(src.canonicalize()?),
                "plugin symlink escapes source"
            );
            anyhow::bail!("symlinks are rejected during v0.1 import")
        } else if e.file_type().is_dir() {
            fs::create_dir_all(target)?
        } else if e.file_type().is_file() {
            if let Some(p) = target.parent() {
                fs::create_dir_all(p)?;
            }
            fs::copy(e.path(), target)?;
        } else {
            anyhow::bail!("special file rejected")
        }
    }
    Ok(())
}

fn inspect_plugin(source: &Path) -> Result<(&'static str, String, Vec<String>)> {
    anyhow::ensure!(source.is_dir(), "plugin source must be a directory");
    let candidates = [
        ("portable", PathBuf::from("plugin.json")),
        ("cursor", PathBuf::from(".cursor-plugin/plugin.json")),
        ("claude", PathBuf::from(".claude-plugin/plugin.json")),
    ];
    let (format, relative) = candidates
        .into_iter()
        .find(|(_, path)| source.join(path).is_file())
        .context("recognized plugin manifest not found")?;
    let value: serde_json::Value = serde_json::from_slice(&fs::read(source.join(&relative))?)?;
    let object = value
        .as_object()
        .context("plugin manifest must be an object")?;
    let metadata = [
        "$schema",
        "name",
        "version",
        "description",
        "author",
        "homepage",
        "repository",
        "license",
        "keywords",
    ];
    let component_keys = [
        "skills",
        "mcp",
        "mcpServers",
        "rules",
        "agents",
        "commands",
        "hooks",
        "lsp",
        "assets",
    ];
    for key in object.keys() {
        anyhow::ensure!(
            metadata.contains(&key.as_str()) || component_keys.contains(&key.as_str()),
            "unknown plugin component or field: {key}"
        );
    }
    let mut components = Vec::new();
    for key in component_keys {
        if let Some(component) = object.get(key) {
            validate_component_paths(component)?;
            components.push(key.to_string());
        }
    }
    Ok((format, relative.to_string_lossy().into_owned(), components))
}

fn validate_component_paths(value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::String(text) => {
            if text.contains('/') || text.contains('\\') || text.starts_with('.') {
                let path = Path::new(text);
                anyhow::ensure!(!path.is_absolute(), "absolute plugin path rejected: {text}");
                anyhow::ensure!(
                    !path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "plugin path traversal rejected: {text}"
                );
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                validate_component_paths(value)?;
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                validate_component_paths(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn import_mcp(paths: &AgentHubPaths, item: &ScanItem, id: &str) -> Result<()> {
    let name = item
        .source_key
        .as_deref()
        .and_then(|v| v.strip_prefix("server:"))
        .unwrap_or(id);
    let value = if item.path.extension().is_some_and(|v| v == "toml") {
        let doc: toml::Value = fs::read_to_string(&item.path)?.parse()?;
        serde_json::to_value(
            doc.get("mcp_servers")
                .and_then(|v| v.get(name))
                .context("MCP server missing")?,
        )?
    } else {
        let doc: serde_json::Value = serde_json::from_slice(&fs::read(&item.path)?)?;
        doc.get("mcpServers")
            .and_then(|v| v.get(name))
            .cloned()
            .context("MCP server missing")?
    };
    let transport = if value.get("command").is_some() {
        "stdio"
    } else {
        value.get("type").and_then(|v| v.as_str()).unwrap_or("http")
    };
    let headers = value
        .get("headers")
        .or_else(|| value.get("http_headers"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let server = serde_json::json!({"schemaVersion":1,"id":id,"display_name":name,"transport":transport,"command":value.get("command"),"args":value.get("args").cloned().unwrap_or_else(||serde_json::json!([])),"url":value.get("url"),"env":value.get("env").cloned().unwrap_or_else(||serde_json::json!({})),"headers":headers});
    let dest = paths.mcp.join(id);
    fs::create_dir_all(&dest)?;
    fs::write(
        dest.join("server.json"),
        serde_json::to_vec_pretty(&server)?,
    )?;
    Ok(())
}

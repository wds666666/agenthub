use crate::{
    canonical::{self, sha256, tree_digest},
    models::{CapabilityKind, HostCleanupResult, HostResource, HostResourceRelation, Target},
    paths::{set_private_file, AgentHubPaths},
    scanner,
};
use anyhow::{Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use walkdir::WalkDir;

pub fn inventory(paths: &AgentHubPaths, target: Target) -> Result<Vec<HostResource>> {
    let caps = canonical::inventory(paths)?;
    let mut out = Vec::new();
    for item in scanner::scan_global(paths, &[target])? {
        if item.source != target.as_str() {
            continue;
        }
        let display_name = item
            .source_key
            .as_deref()
            .and_then(|v| v.strip_prefix("server:"))
            .map(str::to_owned)
            .or_else(|| {
                item.path
                    .file_stem()
                    .map(|v| v.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| item.id[..8].to_owned());
        let canonical = caps.iter().find(|cap| {
            cap.kind == item.kind && (cap.digest == item.digest || cap.id == display_name)
        });
        let plugin_constraint =
            item.kind == CapabilityKind::Plugin && matches!(target, Target::Codex | Target::Claude);
        let relation = if plugin_constraint {
            HostResourceRelation::Constraint
        } else if canonical.is_some() {
            HostResourceRelation::CanonicalMatch
        } else {
            HostResourceRelation::HostOnly
        };
        out.push(HostResource {
            id: item.id,
            target,
            kind: item.kind,
            display_name,
            path: item.path,
            digest: item.digest,
            relation,
            canonical_id: canonical.map(|cap| cap.id.clone()),
            deletable: item.importable && !plugin_constraint,
            constraint: plugin_constraint
                .then(|| format!("{}_plugins_cli_managed", target.as_str())),
        });
    }
    if matches!(target, Target::Codex | Target::Claude) {
        let cache = paths
            .user_home
            .join(format!(".{}/plugins/cache", target.as_str()));
        collect_cached_plugins(&cache, target, &mut out)?;
    }
    out.sort_by(|left, right| {
        (left.kind, &left.display_name, &left.path).cmp(&(
            right.kind,
            &right.display_name,
            &right.path,
        ))
    });
    Ok(out)
}

fn collect_cached_plugins(root: &Path, target: Target, out: &mut Vec<HostResource>) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in WalkDir::new(root)
        .min_depth(2)
        .max_depth(5)
        .follow_links(false)
    {
        let entry = entry?;
        if !entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();
        let recognized = path.join("plugin.json").is_file()
            || path.join(".codex-plugin/plugin.json").is_file()
            || path.join(".claude-plugin/plugin.json").is_file();
        if !recognized {
            continue;
        }
        let relative = path.strip_prefix(root)?;
        let parts: Vec<_> = relative.components().collect();
        let display_name = parts
            .get(1)
            .or_else(|| parts.first())
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_else(|| "plugin".into());
        let digest = tree_digest(path)?;
        out.push(HostResource {
            id: sha256(
                format!("{}:plugin:{}:{}", target.as_str(), path.display(), digest).as_bytes(),
            ),
            target,
            kind: CapabilityKind::Plugin,
            display_name,
            path: path.to_path_buf(),
            digest,
            relation: HostResourceRelation::Constraint,
            canonical_id: None,
            deletable: false,
            constraint: Some(format!("{}_plugins_cli_managed", target.as_str())),
        });
    }
    Ok(())
}

pub fn cleanup(
    paths: &AgentHubPaths,
    target: Target,
    selected_ids: &[String],
) -> Result<HostCleanupResult> {
    anyhow::ensure!(
        !selected_ids.is_empty(),
        "select at least one host resource"
    );
    let selected: BTreeSet<_> = selected_ids.iter().collect();
    let current = inventory(paths, target)?;
    let mut resources: Vec<_> = current
        .into_iter()
        .filter(|item| selected.contains(&item.id))
        .collect();
    anyhow::ensure!(
        resources.len() == selected.len(),
        "host inventory changed; scan again"
    );
    resources.sort_by_key(|item| std::cmp::Reverse(item.path.components().count()));
    anyhow::ensure!(
        resources.iter().all(|item| item.deletable),
        "selection contains a protected host constraint"
    );

    let id = format!("host-cleanup-{}", Uuid::new_v4());
    let backup = paths.backups.join(&id);
    fs::create_dir_all(&backup)?;
    let mut unique = BTreeMap::<PathBuf, PathBuf>::new();
    for item in &resources {
        if !unique.contains_key(&item.path) {
            let destination = backup.join(format!("path-{}", unique.len()));
            unique.insert(item.path.clone(), destination);
        }
    }
    for (source, destination) in &unique {
        copy_path(source, destination)?;
    }
    let manifest = serde_json::to_vec_pretty(&resources)?;
    fs::write(backup.join("manifest.json"), manifest)?;
    set_private_file(&backup.join("manifest.json"))?;

    let result = (|| -> Result<()> {
        for item in &resources {
            match item.kind {
                CapabilityKind::Mcp => remove_mcp(&item.path, target, &item.display_name)?,
                _ if item.path.is_dir() => fs::remove_dir_all(&item.path)
                    .with_context(|| format!("delete {}", item.path.display()))?,
                _ if item.path.exists() => fs::remove_file(&item.path)
                    .with_context(|| format!("delete {}", item.path.display()))?,
                _ => anyhow::bail!("host resource disappeared: {}", item.path.display()),
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        for (original, saved) in &unique {
            let _ = restore_path(saved, original);
        }
        return Err(error.context("host cleanup failed and backup restoration was attempted"));
    }
    Ok(HostCleanupResult {
        id,
        deleted: resources.into_iter().map(|item| item.id).collect(),
        backup_path: backup,
    })
}

fn remove_mcp(path: &Path, target: Target, name: &str) -> Result<()> {
    match target {
        Target::Codex => {
            let mut value: toml::Value = fs::read_to_string(path)?.parse()?;
            value
                .get_mut("mcp_servers")
                .and_then(toml::Value::as_table_mut)
                .context("missing mcp_servers")?
                .remove(name);
            atomic_write(path, value.to_string().as_bytes())
        }
        Target::Cursor | Target::Claude => {
            let mut value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
            value
                .get_mut("mcpServers")
                .and_then(serde_json::Value::as_object_mut)
                .context("missing mcpServers")?
                .remove(name);
            atomic_write(path, &serde_json::to_vec_pretty(&value)?)
        }
        Target::Agents => anyhow::bail!("Shared Agents has no MCP domain"),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension("agenthub-tmp");
    fs::write(&temp, bytes)?;
    set_private_file(&temp)?;
    fs::rename(temp, path)?;
    Ok(())
}

fn copy_path(source: &Path, destination: &Path) -> Result<()> {
    if source.is_file() {
        fs::create_dir_all(destination.parent().context("backup parent")?)?;
        fs::copy(source, destination)?;
        return Ok(());
    }
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        anyhow::ensure!(
            !entry.file_type().is_symlink(),
            "refusing to back up symlink: {}",
            entry.path().display()
        );
        let dest = destination.join(entry.path().strip_prefix(source)?);
        if entry.file_type().is_dir() {
            fs::create_dir_all(dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn restore_path(saved: &Path, original: &Path) -> Result<()> {
    if original.is_dir() {
        fs::remove_dir_all(original)?;
    } else if original.exists() {
        fs::remove_file(original)?;
    }
    copy_path(saved, original)
}

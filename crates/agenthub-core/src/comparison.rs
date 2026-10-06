//! Portable content identity, independent of storage wrappers and transaction hashes.
use crate::{
    canonical::{self, sha256},
    models::{CapabilityKind, McpServer, ScanItem, Target},
    paths::AgentHubPaths,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub fn portable_tree(path: &Path) -> Result<String> {
    let mut entries = Vec::new();
    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        anyhow::ensure!(
            !entry.file_type().is_symlink(),
            "nonportable capability link"
        );
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(path)?
                .components()
                .map(|p| p.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            entries.push((relative, canonical::file_digest(entry.path())?));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(sha256(&serde_json::to_vec(&entries)?))
}

pub fn mcp_value(value: &Value, target: Option<Target>) -> Result<Value> {
    let object = value.as_object().context("MCP server must be an object")?;
    let transport = if object.contains_key("command") && !value["command"].is_null() {
        "stdio"
    } else if target == Some(Target::Codex) {
        "http"
    } else {
        value
            .get("transport")
            .or_else(|| value.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("http")
    };
    let command = if transport == "stdio" {
        value
            .get("command")
            .cloned()
            .context("MCP command missing")?
    } else {
        Value::Null
    };
    let url = if transport != "stdio" {
        value.get("url").cloned().context("MCP URL missing")?
    } else {
        Value::Null
    };
    let args = if transport == "stdio" {
        value.get("args").cloned().unwrap_or(json!([]))
    } else {
        json!([])
    };
    let env = if transport == "stdio" {
        value.get("env").cloned().unwrap_or(json!({}))
    } else {
        json!({})
    };
    let headers = if transport != "stdio" {
        value
            .get("http_headers")
            .or_else(|| value.get("headers"))
            .cloned()
            .unwrap_or(json!({}))
    } else {
        json!({})
    };
    anyhow::ensure!(
        args.is_array() && env.is_object() && headers.is_object(),
        "invalid MCP options"
    );
    // Never collapse unsupported host options into a misleading equality.
    let mut extra = serde_json::Map::new();
    for (key, val) in object {
        if (key == "enabled" && val == &Value::Bool(true))
            || (key == "disabled" && val == &Value::Bool(false))
        {
            continue;
        }
        if ![
            "schemaVersion",
            "id",
            "display_name",
            "displayName",
            "transport",
            "type",
            "command",
            "url",
            "args",
            "env",
            "headers",
            "http_headers",
        ]
        .contains(&key.as_str())
        {
            extra.insert(key.clone(), val.clone());
        }
    }
    Ok(
        json!({"transport":transport,"command":command,"args":args,"url":url,"env":env,"headers":headers,"extra":extra}),
    )
}

pub fn read_mcp(item: &ScanItem) -> Result<Value> {
    let name = item
        .source_key
        .as_deref()
        .and_then(|s| s.strip_prefix("server:"))
        .context("MCP identity missing")?;
    let root: Value = if item.path.extension().is_some_and(|x| x == "toml") {
        serde_json::to_value(fs::read_to_string(&item.path)?.parse::<toml::Value>()?)?
    } else {
        serde_json::from_slice(&fs::read(&item.path)?)?
    };
    root.get(if item.source == "codex" {
        "mcp_servers"
    } else {
        "mcpServers"
    })
    .and_then(|v| v.get(name))
    .cloned()
    .context("MCP server missing")
}

pub fn rule_body(body: &str) -> String {
    body.replace("\r\n", "\n").trim().to_owned()
}

pub fn canonical_digest(kind: CapabilityKind, path: &Path) -> Result<String> {
    match kind {
        CapabilityKind::Skill => Ok(crate::skill_content::inspect(path)?.digest),
        CapabilityKind::Plugin => portable_tree(&path.join("payload")),
        CapabilityKind::Rule => {
            let metadata: Value = serde_json::from_slice(&fs::read(path.join("rule.json"))?)?;
            let activation = metadata
                .get("activation")
                .and_then(Value::as_str)
                .unwrap_or("always")
                .to_owned();
            let mut paths: Vec<String> =
                serde_json::from_value(metadata.get("paths").cloned().unwrap_or(json!([])))?;
            paths.sort();
            paths.dedup();
            let value = crate::rule_projection::RuleContent {
                body: rule_body(&fs::read_to_string(path.join("rule.md"))?),
                activation,
                paths,
            };
            Ok(sha256(&serde_json::to_vec(&value)?))
        }
        CapabilityKind::Mcp => {
            let value: Value = serde_json::from_slice(&fs::read(path.join("server.json"))?)?;
            Ok(sha256(&serde_json::to_vec(&mcp_value(&value, None)?)?))
        }
    }
}

pub fn scan_digest(item: &ScanItem) -> Result<String> {
    match item.kind {
        CapabilityKind::Skill => Ok(crate::skill_content::inspect(&item.path)?.digest),
        CapabilityKind::Plugin => portable_tree(&item.path),
        CapabilityKind::Rule => Ok(sha256(&serde_json::to_vec(
            &crate::rule_projection::content(
                &crate::rule_projection::source_body(item)?,
                &item.source,
            )?,
        )?)),
        CapabilityKind::Mcp => Ok(sha256(&serde_json::to_vec(&mcp_value(
            &read_mcp(item)?,
            item.source.parse().ok(),
        )?)?)),
    }
}

fn resolve_reference_maps(value: &Value) -> Result<Value> {
    let mut resolved = value.clone();
    for field in ["env", "headers"] {
        if let Some(map) = resolved.get_mut(field).and_then(Value::as_object_mut) {
            for entry in map.values_mut() {
                if let Some(text) = entry.as_str() {
                    if let Some(name) = text.strip_prefix("${").and_then(|v| v.strip_suffix('}')) {
                        let value = std::env::var(name).map_err(|_| {
                            anyhow::anyhow!("credential environment reference unavailable")
                        })?;
                        *entry = Value::String(value);
                    } else if text.starts_with("secretRef:") || text.starts_with("secret://") {
                        anyhow::bail!("credential reference unavailable");
                    }
                }
            }
        }
    }
    Ok(resolved)
}

pub fn annotate(paths: &AgentHubPaths, items: &mut [ScanItem]) -> Result<()> {
    let caps = canonical::inventory(paths)?;
    for item in items {
        if !item.importable {
            continue;
        }
        item.comparison_digest = match scan_digest(item) {
            Ok(digest) => digest,
            Err(error) if item.kind == CapabilityKind::Skill => {
                return Err(error)
                    .context("import skill from selected source: portable validation failed");
            }
            Err(_) => {
                item.importable = false;
                item.warning = Some("comparison_unavailable".into());
                continue;
            }
        };
        if item.kind == CapabilityKind::Mcp {
            let target: Option<Target> = item.source.parse().ok();
            let host = mcp_value(&read_mcp(item)?, target)?;
            if host["extra"]
                .as_object()
                .is_some_and(|extra| !extra.is_empty())
            {
                item.importable = false;
                item.warning = Some("comparison_unavailable".into());
                continue;
            }
            for cap in caps.iter().filter(|c| c.kind == CapabilityKind::Mcp) {
                let server: McpServer =
                    serde_json::from_slice(&fs::read(cap.path.join("server.json"))?)?;
                let value = mcp_value(&serde_json::to_value(server)?, target)?;
                if value == host {
                    item.comparison_digest = cap.comparison_digest.clone();
                    break;
                }
                match (
                    resolve_reference_maps(&value),
                    resolve_reference_maps(&host),
                ) {
                    (Ok(library), Ok(tool)) if library == tool => {
                        item.comparison_digest = cap.comparison_digest.clone();
                        break;
                    }
                    (Err(_), _) | (_, Err(_))
                        if item.source_key.as_deref()
                            == Some(format!("server:{}", cap.id).as_str()) =>
                    {
                        item.importable = false;
                        item.warning = Some("comparison_unavailable".into());
                    }
                    _ => {}
                }
            }
        }
        if !caps
            .iter()
            .any(|cap| cap.kind == item.kind && cap.comparison_digest == item.comparison_digest)
        {
            let name = item
                .source_key
                .as_deref()
                .and_then(|s| {
                    s.strip_prefix("server:")
                        .or_else(|| s.strip_prefix("rule:"))
                })
                .map(str::to_owned)
                .or_else(|| {
                    item.path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                });
            if caps
                .iter()
                .any(|cap| cap.kind == item.kind && Some(&cap.id) == name.as_ref())
                && item.warning.is_none()
            {
                item.warning = Some("capability_modified".into());
            }
        }
    }
    Ok(())
}

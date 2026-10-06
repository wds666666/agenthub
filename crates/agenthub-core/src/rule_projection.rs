//! Reversible rule blocks in combined tool documents; user text is never inferred as owned.
use crate::{canonical, models::ScanItem, paths::AgentHubPaths};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

const START: &str = "<!-- agenthub:rule ";
const END: &str = "<!-- /agenthub:rule ";

pub fn blocks(body: &str) -> Result<Vec<(String, String, std::ops::Range<usize>)>> {
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(found) = body[offset..].find(START) {
        let start = offset + found;
        anyhow::ensure!(
            !body[offset..start].contains(END),
            "unmatched AgentHub rule closing marker"
        );
        let header_end = body[start..]
            .find(" -->")
            .context("malformed AgentHub rule marker")?
            + start;
        let id = &body[start + START.len()..header_end];
        anyhow::ensure!(
            canonical::valid_id(id) && !out.iter().any(|(name, _, _)| name == id),
            "invalid or duplicate AgentHub rule block"
        );
        let content_start = header_end + 4;
        let closing = format!("{END}{id} -->");
        let close = body[content_start..]
            .find(&closing)
            .context("missing AgentHub rule closing marker")?
            + content_start;
        anyhow::ensure!(
            !body[content_start..close].contains(START),
            "nested AgentHub rule block"
        );
        let end = close + closing.len();
        out.push((
            id.to_owned(),
            body[content_start..close].trim().to_owned(),
            start..end,
        ));
        offset = end;
    }
    anyhow::ensure!(
        !body[offset..].contains(END),
        "unmatched AgentHub rule closing marker"
    );
    Ok(out)
}

pub fn render(
    original: &str,
    desired: &[(String, String)],
    preserve: bool,
    paths: &AgentHubPaths,
) -> Result<String> {
    if desired.is_empty() && preserve {
        return Ok(original.to_owned());
    }
    let mut body = original.to_owned();
    if body.starts_with("# AgentHub managed global rules\n") {
        let mut known = String::from("# AgentHub managed global rules\n\n");
        for cap in canonical::inventory(paths)?
            .into_iter()
            .filter(|c| c.kind == crate::models::CapabilityKind::Rule)
        {
            known.push_str(&format!(
                "## {}\n\n{}\n\n",
                cap.id,
                std::fs::read_to_string(cap.path.join("rule.md"))?.trim()
            ));
        }
        anyhow::ensure!(
            body == known,
            "legacy rule document differs; review it before synchronization"
        );
        let all = canonical::inventory(paths)?
            .into_iter()
            .filter(|c| c.kind == crate::models::CapabilityKind::Rule)
            .map(|c| Ok((c.id, std::fs::read_to_string(c.path.join("rule.md"))?)))
            .collect::<Result<Vec<_>>>()?;
        body = String::new();
        for (id, text) in all {
            body.push_str(&format!(
                "{START}{id} -->\n{}\n{END}{id} -->\n\n",
                text.trim()
            ));
        }
    }
    let parsed = blocks(&body)?;
    let mut selected: BTreeMap<_, _> = desired.iter().cloned().collect();
    for (_, text) in desired {
        anyhow::ensure!(
            !text.contains(START) && !text.contains(END),
            "rule body contains reserved markers"
        );
    }
    if preserve {
        for (id, _, range) in parsed.into_iter().rev() {
            if let Some(text) = selected.remove(&id) {
                body.replace_range(
                    range,
                    &format!("{START}{id} -->\n{}\n{END}{id} -->", text.trim()),
                );
            }
        }
    } else {
        body = String::new();
    }
    for (id, text) in selected {
        if !body.is_empty() && !body.ends_with('\n') {
            body.push('\n');
        }
        body.push_str(&format!(
            "{START}{id} -->\n{}\n{END}{id} -->\n",
            text.trim()
        ));
    }
    Ok(body)
}

pub fn source_body(item: &ScanItem) -> Result<String> {
    let body = std::fs::read_to_string(&item.path)?;
    if let Some(id) = item
        .source_key
        .as_deref()
        .and_then(|s| s.strip_prefix("rule:"))
    {
        return blocks(&body)?
            .into_iter()
            .find(|(name, _, _)| name == id)
            .map(|(_, body, _)| body)
            .context("rule block missing");
    }
    Ok(body)
}

const OPTIONS: &str = "<!-- agenthub:rule-options ";
#[derive(serde::Serialize)]
pub struct RuleContent {
    pub body: String,
    pub activation: String,
    pub paths: Vec<String>,
}

pub fn content(body: &str, source: &str) -> Result<RuleContent> {
    let normalized = body.replace("\r\n", "\n");
    let mut text = normalized.as_str();
    let mut front = None;
    if text.starts_with("---\n") {
        if let Some(end) = text[4..].find("\n---\n") {
            let header = &text[4..end + 4];
            let remainder = &text[end + 9..];
            if remainder.starts_with(OPTIONS) {
                front = Some(header);
                text = remainder;
            }
        }
    }
    let mut activation = "always".to_owned();
    let mut paths = Vec::new();
    if let Some(rest) = text.strip_prefix(OPTIONS) {
        let end = rest
            .find(" -->\n")
            .context("invalid rule metadata marker")?;
        let options: serde_json::Value = serde_json::from_str(&rest[..end])?;
        activation = options["activation"]
            .as_str()
            .context("rule activation missing")?
            .to_owned();
        paths = serde_json::from_value(options["paths"].clone())?;
        text = &rest[end + 5..];
        if let Some(header) = front {
            if source == "cursor" {
                let always = header
                    .lines()
                    .find_map(|line| line.strip_prefix("alwaysApply: "))
                    .context("missing rule activation")?;
                let globs = header
                    .lines()
                    .find_map(|line| line.strip_prefix("globs: "))
                    .context("missing rule paths")?;
                paths = serde_json::from_str(globs)?;
                activation = match always {
                    "true" => "always",
                    "false" if paths.is_empty() => "manual",
                    "false" => "paths",
                    _ => anyhow::bail!("unsupported rule activation"),
                }
                .into();
            } else if source == "claude" {
                paths = serde_json::from_str(
                    header
                        .strip_prefix("paths: ")
                        .context("unsupported rule paths")?,
                )?;
                activation = if paths.is_empty() { "always" } else { "paths" }.into();
            }
        }
    }
    anyhow::ensure!(
        ["always", "manual", "paths"].contains(&activation.as_str()),
        "unsupported rule activation"
    );
    paths.sort();
    paths.dedup();
    Ok(RuleContent {
        body: crate::comparison::rule_body(text),
        activation,
        paths,
    })
}

pub fn project(
    rule: &crate::models::RuleDocument,
    target: crate::models::Target,
) -> Result<String> {
    use crate::models::Target;
    anyhow::ensure!(
        target != Target::Codex || (rule.activation == "always" && rule.paths.is_empty()),
        "Codex global rules cannot represent conditional activation"
    );
    anyhow::ensure!(
        target != Target::Claude || rule.activation != "manual",
        "Claude global rules cannot represent manual activation"
    );
    let mut text = String::new();
    if target == Target::Cursor {
        text.push_str(&format!(
            "---\nalwaysApply: {}\nglobs: {}\n---\n",
            rule.activation == "always",
            serde_json::to_string(&rule.paths)?
        ));
    } else if target == Target::Claude && rule.activation == "paths" {
        text.push_str(&format!(
            "---\npaths: {}\n---\n",
            serde_json::to_string(&rule.paths)?
        ));
    }
    let options = serde_json::json!({"activation":rule.activation,"paths":rule.paths});
    text.push_str(&format!(
        "{OPTIONS}{} -->\n{}",
        serde_json::to_string(&options)?,
        rule.body
    ));
    Ok(text)
}

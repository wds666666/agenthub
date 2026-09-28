use crate::{
    adapters::{actual_files, projection_with_selection},
    canonical::{canonical_digest, sha256},
    git,
    models::{
        CapabilityKind, Plan, PlanAction, PlanCapabilitySummary, PlanStep, SyncSelection, Target,
    },
    paths::AgentHubPaths,
};
use anyhow::Result;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

fn logical_owner(
    target: Target,
    domain: &crate::adapters::DomainProjection,
    rel: &std::path::Path,
) -> (CapabilityKind, String) {
    let parts: Vec<_> = rel
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    if target == Target::Cursor
        && domain.kind == CapabilityKind::Plugin
        && parts.first().is_some_and(|part| part == "agenthub-rules")
    {
        let id = parts
            .get(2)
            .and_then(|name| name.strip_suffix(".mdc"))
            .unwrap_or("__projection__")
            .to_string();
        return (CapabilityKind::Rule, id);
    }
    let mut id = match (parts.first(), parts.get(1)) {
        (Some(root), Some(child)) if root.starts_with('.') => format!("{root}/{child}"),
        (Some(root), _) => root.clone(),
        _ => "__projection__".into(),
    };
    if domain.kind == CapabilityKind::Rule {
        id = id
            .strip_suffix(".md")
            .or_else(|| id.strip_suffix(".mdc"))
            .unwrap_or(&id)
            .to_string();
    }
    (domain.kind, id)
}

fn summarize(
    presence: BTreeMap<(CapabilityKind, String), (bool, bool)>,
    files: BTreeMap<CapabilityKind, usize>,
) -> Vec<PlanCapabilitySummary> {
    let mut result: BTreeMap<CapabilityKind, PlanCapabilitySummary> = BTreeMap::new();
    for ((kind, _), (current, desired)) in presence {
        let item = result.entry(kind).or_insert_with(|| PlanCapabilitySummary {
            kind,
            ..Default::default()
        });
        item.affected += 1;
        if !current && desired {
            item.create += 1;
        } else if current && !desired {
            item.delete += 1;
        } else {
            item.update += 1;
        }
    }
    for (kind, count) in files {
        result
            .entry(kind)
            .or_insert_with(|| PlanCapabilitySummary {
                kind,
                ..Default::default()
            })
            .files = count;
    }
    result.into_values().collect()
}

pub fn create(paths: &AgentHubPaths, target: Target) -> Result<Plan> {
    create_with_selection(paths, target, None)
}

pub fn create_with_selection(
    paths: &AgentHubPaths,
    target: Target,
    selection: Option<&SyncSelection>,
) -> Result<Plan> {
    let canonical = canonical_digest(paths)?;
    let git = git::snapshot(&paths.root)?;
    let domains = projection_with_selection(paths, target, selection)?;
    let mut steps = Vec::new();
    let mut warnings = Vec::new();
    let mut expected = Sha256::new();
    let mut presence = BTreeMap::new();
    let mut changed_files = BTreeMap::new();
    for domain in domains {
        let actual = actual_files(&domain)?;
        for rel in actual.keys() {
            presence
                .entry(logical_owner(target, &domain, rel))
                .or_insert((false, false))
                .0 = true;
        }
        for rel in domain.files.keys() {
            presence
                .entry(logical_owner(target, &domain, rel))
                .or_insert((false, false))
                .1 = true;
        }
        expected.update(domain.name);
        expected.update(domain.digest());
        let mut names = std::collections::BTreeSet::new();
        names.extend(actual.keys().cloned());
        names.extend(domain.files.keys().cloned());
        for rel in names {
            let current = actual.get(&rel);
            let desired = domain.files.get(&rel);
            if current == desired {
                continue;
            }
            let action = match (current, desired) {
                (None, Some(_)) => PlanAction::Create,
                (Some(_), None) => PlanAction::Delete,
                (Some(_), Some(_)) => PlanAction::Replace,
                (None, None) => continue,
            };
            let path = if rel.as_os_str().is_empty() {
                domain.target_path.clone()
            } else {
                domain.target_path.join(&rel)
            };
            let (capability_kind, capability_id) = logical_owner(target, &domain, &rel);
            *changed_files.entry(capability_kind).or_default() += 1;
            steps.push(PlanStep {
                action,
                capability_kind: Some(capability_kind),
                capability_id: Some(capability_id),
                path,
                current_digest: current.map(|v| sha256(v)),
                desired_digest: desired.map(|v| sha256(v)),
                detail: format!("{} 可写域", domain.name),
            });
        }
        if domain.sensitive
            && domain
                .files
                .values()
                .any(|v| String::from_utf8_lossy(v).contains("secretRef"))
        {
            warnings.push(format!(
                "{} requires secret materialization review",
                domain.name
            ));
        }
    }
    let changed_owners: std::collections::BTreeSet<_> = steps
        .iter()
        .filter_map(|step| Some((step.capability_kind?, step.capability_id.clone()?)))
        .collect();
    presence.retain(|owner, _| changed_owners.contains(owner));
    let summary = summarize(presence, changed_files);
    Ok(Plan {
        id: Uuid::new_v4().to_string(),
        target,
        canonical_digest: canonical,
        expected_digest: hex::encode(expected.finalize()),
        git,
        steps,
        selection: selection.cloned(),
        summary,
        warnings,
        created_at: Utc::now().to_rfc3339(),
    })
}

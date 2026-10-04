//! Portable Skill content, shared by discovery preflight and import.
use crate::canonical::sha256;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SkillError {
    #[error("skill contains a nonportable symlink: {0}")]
    Symlink(PathBuf),
    #[error("skill contains a special file: {0}")]
    SpecialFile(PathBuf),
    #[error("SKILL.md is empty")]
    Empty,
    #[error("SKILL.md must use UTF-8 encoding")]
    Encoding,
}

pub(crate) struct SkillContent {
    pub digest: String,
    pub excluded: Vec<PathBuf>,
    entries: Vec<DirEntry>,
}

fn is_runtime(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    if name == ".git" {
        return true;
    }
    if entry.file_type().is_dir() || entry.file_type().is_symlink() {
        if matches!(
            name.as_ref(),
            ".venv"
                | "venv"
                | "node_modules"
                | "__pycache__"
                | ".pytest_cache"
                | ".mypy_cache"
                | ".ruff_cache"
        ) {
            return true;
        }
        // Identify custom virtualenv directories without following link entries.
        if entry.file_type().is_dir() && entry.path().join("pyvenv.cfg").is_file() {
            return true;
        }
    }
    entry.file_type().is_file()
        && entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "pyc" || extension == "pyo")
}

pub(crate) fn inspect(source: &Path) -> Result<SkillContent> {
    anyhow::ensure!(
        fs::symlink_metadata(source)?.file_type().is_dir(),
        "skill source must be an ordinary directory"
    );
    let mut excluded = Vec::new();
    let mut entries = WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if is_runtime(entry) {
                excluded.push(
                    entry
                        .path()
                        .strip_prefix(source)
                        .unwrap_or(entry.path())
                        .to_path_buf(),
                );
                false
            } else {
                true
            }
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("read skill directory")?;
    entries.sort_by(|left, right| left.path().cmp(right.path()));
    for entry in &entries {
        let relative = entry.path().strip_prefix(source)?;
        if entry.file_type().is_symlink() {
            return Err(SkillError::Symlink(relative.into()).into());
        }
        if !entry.file_type().is_dir() && !entry.file_type().is_file() {
            return Err(SkillError::SpecialFile(relative.into()).into());
        }
    }
    let body = fs::read(source.join("SKILL.md")).context("read SKILL.md")?;
    let body = std::str::from_utf8(&body).map_err(|_| SkillError::Encoding)?;
    if body.trim().is_empty() {
        return Err(SkillError::Empty.into());
    }
    let mut hasher = Sha256::new();
    for entry in &entries {
        if entry.file_type().is_file() {
            let relative = entry.path().strip_prefix(source)?;
            hasher.update(relative.to_string_lossy().as_bytes());
            hasher.update(
                fs::read(entry.path())
                    .with_context(|| format!("read skill file {}", relative.display()))?,
            );
        }
    }
    Ok(SkillContent {
        digest: hex::encode(hasher.finalize()),
        excluded,
        entries,
    })
}

pub(crate) fn copy(source: &Path, destination: &Path) -> Result<()> {
    let content = inspect(source)?;
    fs::create_dir_all(destination)?;
    for entry in content.entries {
        let relative = entry.path().strip_prefix(source)?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            anyhow::ensure!(
                fs::symlink_metadata(entry.path())?.file_type().is_file(),
                "skill file changed during import: {}",
                relative.display()
            );
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)
                .with_context(|| format!("copy skill file {}", relative.display()))?;
        }
    }
    Ok(())
}

pub(crate) fn invalid_digest(source: &Path) -> String {
    sha256(format!("invalid-skill:{}", source.display()).as_bytes())
}

/// Cheap portable metadata signature; never reads file bodies or follows links.
pub(crate) fn fingerprint(source: &Path) -> Result<String> {
    anyhow::ensure!(
        fs::symlink_metadata(source)?.file_type().is_dir(),
        "skill source must be an ordinary directory"
    );
    let mut entries = WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !is_runtime(entry))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by(|a, b| a.path().cmp(b.path()));
    let mut hasher = Sha256::new();
    for entry in entries {
        let meta = fs::symlink_metadata(entry.path())?;
        anyhow::ensure!(
            !meta.file_type().is_symlink(),
            "skill contains a nonportable symlink"
        );
        anyhow::ensure!(
            meta.is_file() || meta.is_dir(),
            "skill contains a special file"
        );
        hasher.update(
            entry
                .path()
                .strip_prefix(source)?
                .to_string_lossy()
                .as_bytes(),
        );
        hasher.update(
            format!(
                "{:?}:{}:{:?}:{:?}",
                meta.file_type(),
                meta.len(),
                meta.modified()?,
                meta.created().ok()
            )
            .as_bytes(),
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            hasher.update(
                format!("{}:{}:{}", meta.ino(), meta.ctime(), meta.ctime_nsec()).as_bytes(),
            );
        }
    }
    Ok(hex::encode(hasher.finalize()))
}

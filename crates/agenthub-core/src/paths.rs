use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct AgentHubPaths {
    pub user_home: PathBuf,
    pub root: PathBuf,
    pub skills: PathBuf,
    pub plugins: PathBuf,
    pub rules: PathBuf,
    pub mcp: PathBuf,
    pub state: PathBuf,
    pub database: PathBuf,
    pub secrets: PathBuf,
    pub master_key: PathBuf,
    pub backups: PathBuf,
    pub projections: PathBuf,
}

impl AgentHubPaths {
    pub fn discover() -> Result<Self> {
        let user_home = dirs::home_dir().context("HOME is unavailable")?;
        let root = std::env::var_os("AGENTHUB_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| user_home.join(".agenthub"));
        Ok(Self::new(user_home, root))
    }
    pub fn for_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self::new(home.clone(), home.join(".agenthub"))
    }
    pub fn new(user_home: PathBuf, root: PathBuf) -> Self {
        Self {
            skills: root.join("skills"),
            plugins: root.join("plugins"),
            rules: root.join("rules"),
            mcp: root.join("mcp"),
            state: root.join("state"),
            database: root.join("state/agenthub.db"),
            secrets: root.join("secrets"),
            master_key: root.join("secrets/master.key"),
            backups: root.join("backups"),
            projections: root.join("projections"),
            user_home,
            root,
        }
    }
    pub fn ensure_runtime(&self) -> Result<()> {
        set_dir(&self.root, 0o700)?;
        for path in [
            &self.skills,
            &self.plugins,
            &self.rules,
            &self.mcp,
            &self.state,
            &self.secrets,
            &self.backups,
            &self.projections,
        ] {
            set_dir(path, 0o700)?;
        }
        if !self.root.join(".gitignore").exists() {
            fs::write(
                self.root.join(".gitignore"),
                ".gitignore\nstate/\nsecrets/\nbackups/\nprojections/\nruntime/\n*.log\n",
            )?;
        } else {
            let ignore_path = self.root.join(".gitignore");
            let current = fs::read_to_string(&ignore_path)?;
            if !current.lines().any(|line| line.trim() == ".gitignore") {
                fs::write(&ignore_path, format!(".gitignore\n{current}"))?;
            }
        }
        if !self.root.join("agenthub.toml").exists() {
            fs::write(self.root.join("agenthub.toml"), "schema_version = 1\n")?;
        }
        Ok(())
    }
    pub fn assert_inside_root(&self, candidate: &Path) -> Result<()> {
        let parent = candidate.parent().context("path has no parent")?;
        let root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        let resolved_parent = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        anyhow::ensure!(
            resolved_parent.starts_with(root),
            "path escapes AgentHub root: {}",
            candidate.display()
        );
        Ok(())
    }
}

fn set_dir(path: &Path, mode: u32) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
}

pub fn set_private_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub mod adapters;
mod backup;
pub mod bootstrap;
pub mod canonical;
pub mod comparison;
pub mod git;
pub mod git_auth;
pub mod host;
pub mod models;
pub mod paths;
pub mod planner;
pub mod reset;
pub mod rule_projection;
pub mod scanner;
pub mod secrets;
pub mod skill_changes;
mod skill_content;
pub mod storage;
pub mod transaction;
pub mod versions;

use anyhow::Result;
use paths::AgentHubPaths;
use storage::Store;

pub struct AgentHub {
    pub paths: AgentHubPaths,
    pub store: Store,
}

impl AgentHub {
    pub fn open(paths: AgentHubPaths) -> Result<Self> {
        bootstrap::recover_pending(&paths)?;
        paths.ensure_runtime()?;
        versions::recover_pending(&paths)?;
        let store = Store::open(&paths.database)?;
        paths::set_private_file(&paths.database)?;
        if !store.initialized()? {
            let has_content = !canonical::canonical_dirs_empty(&paths)?;
            let has_versions =
                paths.root.join(".git").exists() && git::snapshot(&paths.root)?.head.is_some();
            if has_content || has_versions {
                canonical::validate(&paths)?;
                store.set_initialized(true)?;
            }
        }
        store.migrate_sync_profiles(&paths)?;
        Ok(Self { paths, store })
    }

    pub fn open_default() -> Result<Self> {
        Self::open(AgentHubPaths::discover()?)
    }
}

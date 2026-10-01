pub mod adapters;
pub mod canonical;
pub mod git;
pub mod host;
pub mod models;
pub mod paths;
pub mod planner;
pub mod reset;
pub mod scanner;
pub mod secrets;
pub mod storage;
pub mod transaction;

use anyhow::Result;
use paths::AgentHubPaths;
use storage::Store;

pub struct AgentHub {
    pub paths: AgentHubPaths,
    pub store: Store,
}

impl AgentHub {
    pub fn open(paths: AgentHubPaths) -> Result<Self> {
        paths.ensure_runtime()?;
        let store = Store::open(&paths.database)?;
        paths::set_private_file(&paths.database)?;
        Ok(Self { paths, store })
    }

    pub fn open_default() -> Result<Self> {
        Self::open(AgentHubPaths::discover()?)
    }
}

//! Local recovery copies preserve link metadata without reading link targets.
use anyhow::Result;
use std::path::Path;

pub(crate) fn copy_link(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::os::unix::fs::symlink(std::fs::read_link(source)?, destination)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = destination;
        anyhow::bail!(
            "local backup cannot preserve symlinks on this platform: {}",
            source.display()
        )
    }
}

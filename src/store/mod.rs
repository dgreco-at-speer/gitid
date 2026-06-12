//! On-disk stores: the hand-editable `profiles.toml` and machine-owned
//! `mappings.toml`, plus the atomic-write primitive every writer uses.

pub mod mappings;
pub mod profiles;

use std::path::Path;

use anyhow::{Context, Result};

/// Write `contents` to `path` atomically: write to a temp file in the same
/// directory, then rename over the destination. Creates parent directories as
/// needed.
pub fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let dir = path
        .parent()
        .with_context(|| format!("path has no parent directory: {}", path.display()))?;
    std::fs::create_dir_all(dir)
        .with_context(|| format!("could not create directory {}", dir.display()))?;

    let mut tmp = tempfile::NamedTempFile::new_in(dir)
        .with_context(|| format!("could not create temp file in {}", dir.display()))?;
    use std::io::Write as _;
    tmp.write_all(contents.as_bytes())
        .context("could not write temp file")?;
    tmp.flush().context("could not flush temp file")?;
    tmp.persist(path)
        .with_context(|| format!("could not persist {}", path.display()))?;
    Ok(())
}

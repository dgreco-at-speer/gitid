//! `gitid remove`.

use anyhow::{Result, bail};

use crate::cli::RemoveArgs;
use crate::cmd::Ctx;
use crate::output;
use crate::store::mappings::MappingsFile;
use crate::store::profiles;
use crate::sync::sync_all;

pub fn run(ctx: &Ctx, args: &RemoveArgs) -> Result<()> {
    let profiles_path = ctx.paths.profiles_toml();
    let existing = profiles::load(&profiles_path)?;
    if !existing.profiles.contains_key(&args.name) {
        bail!(
            "{}",
            crate::output::no_such_profile(
                &args.name,
                existing.profiles.keys().map(String::as_str)
            )
        );
    }

    // Refuse if directories are mapped to it, unless forced.
    let mut mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;
    let mapped: Vec<String> = mappings
        .mappings
        .iter()
        .filter(|m| m.profile == args.name)
        .map(|m| m.dir.clone())
        .collect();
    if !mapped.is_empty() {
        if !args.force {
            output::error(&format!(
                "{} director(ies) are mapped to {:?}:",
                mapped.len(),
                args.name
            ));
            for d in &mapped {
                eprintln!("    {d}");
            }
            bail!(
                "refusing to remove a profile in use; pass --force to remove it and its mappings"
            );
        }
        mappings.mappings.retain(|m| m.profile != args.name);
        mappings.save(&ctx.paths.mappings_toml())?;
    }

    let mut doc = profiles::load_doc(&profiles_path)?;
    profiles::remove_profile(&mut doc, &args.name);
    profiles::save_doc(&profiles_path, &doc)?;

    let report = sync_all(&ctx.paths)?;
    output::success(&format!("removed profile {:?}", args.name));

    // The gh config dir holds auth tokens; never delete it implicitly.
    let gh_dir = ctx.paths.gh_dir(&args.name);
    if gh_dir.exists() {
        output::info(&format!(
            "left gh auth dir in place: {} (delete manually if no longer needed)",
            gh_dir.display()
        ));
    }
    crate::cmd::sync::report_changes(&report);
    Ok(())
}

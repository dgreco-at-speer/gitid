//! `gitid use` and `gitid forget` — manage directory→profile mappings.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};

use crate::cli::{ForgetArgs, UseArgs};
use crate::cmd::{Ctx, resolve_dir};
use crate::output;
use crate::paths::{PathStyle, normalize_dir};
use crate::store::mappings::{Mapping, MappingsFile};
use crate::store::profiles;
use crate::sync::sync_all;

pub fn run(ctx: &Ctx, args: &UseArgs) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    if !profiles.profiles.contains_key(&args.profile) {
        bail!(
            "{}",
            crate::output::no_such_profile(
                &args.profile,
                profiles.profiles.keys().map(String::as_str)
            )
        );
    }

    let abs = resolve_dir(args.dir.as_deref(), &ctx.paths.home)?;
    let canonical = std::fs::canonicalize(&abs)
        .with_context(|| format!("directory does not exist: {}", abs.display()))?;

    let style = PathStyle::host();
    let canonical_norm = normalize_dir(&canonical.to_string_lossy(), style);
    let literal_norm = normalize_dir(&abs.to_string_lossy(), style);
    let dir_literal = (literal_norm != canonical_norm).then_some(literal_norm);

    let case_insensitive = if args.icase {
        true
    } else if args.no_icase {
        false
    } else {
        style.default_icase()
    };

    let mut mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;
    mappings.upsert(Mapping {
        dir: canonical_norm.clone(),
        dir_literal,
        profile: args.profile.clone(),
        case_insensitive,
        env: BTreeMap::new(),
    });
    mappings.save(&ctx.paths.mappings_toml())?;

    let report = output::with_spinner("syncing…", || sync_all(&ctx.paths))?;
    output::success(&format!(
        "{} now uses profile {:?}",
        canonical_norm.trim_end_matches('/'),
        args.profile
    ));
    crate::cmd::sync::report_changes(&report);
    Ok(())
}

pub fn forget(ctx: &Ctx, args: &ForgetArgs) -> Result<()> {
    let abs = resolve_dir(args.dir.as_deref(), &ctx.paths.home)?;
    // Prefer the canonical form (matching how `use` stored it), falling back to
    // the literal path when the directory no longer exists.
    let dir_norm = match std::fs::canonicalize(&abs) {
        Ok(c) => normalize_dir(&c.to_string_lossy(), PathStyle::host()),
        Err(_) => normalize_dir(&abs.to_string_lossy(), PathStyle::host()),
    };

    let mut mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;
    let removed = mappings.remove_dir(&dir_norm)
        // Also try the literal path in case it was stored unresolved.
        || mappings.remove_dir(&normalize_dir(&abs.to_string_lossy(), PathStyle::host()));
    if !removed {
        bail!("no mapping for {}", dir_norm.trim_end_matches('/'));
    }
    mappings.save(&ctx.paths.mappings_toml())?;

    let report = output::with_spinner("syncing…", || sync_all(&ctx.paths))?;
    output::success(&format!("forgot {}", dir_norm.trim_end_matches('/')));
    crate::cmd::sync::report_changes(&report);
    Ok(())
}

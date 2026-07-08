//! `gitid current` — show the profile active for a directory, with a
//! cross-check against what git actually resolves there.

use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{CurrentArgs, CurrentFormat};
use crate::cmd::{Ctx, resolve_dir};
use crate::gitconfig::{config_get_with_origin, is_in_repo};
use crate::output;
use crate::paths::PathStyle;
use crate::store::mappings::{MappingsFile, match_dir};
use crate::store::profiles;

pub fn run(ctx: &Ctx, args: &CurrentArgs) -> Result<ExitCode> {
    let abs = resolve_dir(args.dir.as_deref(), &ctx.paths.home)?;
    let mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;
    let matched = match_dir(&mappings.mappings, &abs, PathStyle::host());

    if args.quiet {
        return Ok(if matched.is_some() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }

    let Some(mapping) = matched else {
        match args.format {
            CurrentFormat::Json => println!("{{\"profile\":null}}"),
            CurrentFormat::Name => {}
            CurrentFormat::Pretty => output::info("no profile active for this directory"),
        }
        return Ok(ExitCode::FAILURE);
    };

    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    let profile = profiles.profiles.get(&mapping.profile);

    match args.format {
        CurrentFormat::Name => println!("{}", mapping.profile),
        CurrentFormat::Json => {
            let email = profile.map(|p| p.email.as_str()).unwrap_or("");
            println!(
                "{{\"profile\":{:?},\"email\":{:?},\"dir\":{:?}}}",
                mapping.profile, email, mapping.dir
            );
        }
        CurrentFormat::Pretty => {
            use crate::output::dim;
            println!("{} {}", dim("profile:"), mapping.profile);
            if let Some(p) = profile {
                println!("  {}  {}", dim("name:"), p.name);
                println!("  {} {}", dim("email:"), p.email);
            }
            // Cross-check against git's actual resolution in a repo.
            if is_in_repo(&abs) {
                if let (Some(p), Ok(Some((actual, origin)))) =
                    (profile, config_get_with_origin(&abs, "user.email"))
                {
                    if actual != p.email {
                        output::warn(&format!(
                            "git resolves user.email = {actual} here (from {origin}), \
                             not the profile's {}; a local override may be set",
                            p.email
                        ));
                    }
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

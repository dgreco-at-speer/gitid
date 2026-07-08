//! `gitid list`.

use anyhow::Result;

use crate::cli::{ListArgs, OutputFormat};
use crate::cmd::Ctx;
use crate::output;
use crate::paths::PathStyle;
use crate::store::mappings::{MappingsFile, match_dir};
use crate::store::profiles;

pub fn run(ctx: &Ctx, args: &ListArgs) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;

    // Which profile is active for the current directory?
    let active = std::env::current_dir().ok().and_then(|cwd| {
        let mappings = MappingsFile::load(&ctx.paths.mappings_toml()).ok()?;
        match_dir(&mappings.mappings, &cwd, PathStyle::host()).map(|m| m.profile.clone())
    });

    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&profiles)?);
        }
        OutputFormat::Names => {
            for name in profiles.profiles.keys() {
                println!("{name}");
            }
        }
        OutputFormat::Table => {
            if profiles.profiles.is_empty() {
                output::info(
                    "no profiles yet — add one with `gitid add <name> --git-name … --email …`",
                );
                return Ok(());
            }
            let mut active_row = None;
            let rows: Vec<Vec<String>> = profiles
                .profiles
                .iter()
                .enumerate()
                .map(|(i, (name, p))| {
                    let marker = if active.as_deref() == Some(name) {
                        active_row = Some(i);
                        "●"
                    } else {
                        " "
                    };
                    let gh = if p.gh_enabled() { "yes" } else { "no" };
                    vec![
                        marker.to_string(),
                        name.clone(),
                        p.name.clone(),
                        p.email.clone(),
                        gh.to_string(),
                    ]
                })
                .collect();
            print!(
                "{}",
                output::table_with_active(
                    &["", "NAME", "GIT NAME", "EMAIL", "GH"],
                    &rows,
                    active_row,
                )
            );
        }
    }
    Ok(())
}

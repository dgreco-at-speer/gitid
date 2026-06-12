//! `gitid dirs`.

use anyhow::Result;

use crate::cli::{DirsArgs, OutputFormat};
use crate::cmd::Ctx;
use crate::output;
use crate::store::mappings::MappingsFile;

pub fn run(ctx: &Ctx, args: &DirsArgs) -> Result<()> {
    let mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;

    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&mappings)?);
        }
        OutputFormat::Names => {
            for m in &mappings.mappings {
                println!("{}", m.dir.trim_end_matches('/'));
            }
        }
        OutputFormat::Table => {
            if mappings.mappings.is_empty() {
                output::info(
                    "no directory mappings yet — assign one with `gitid use <profile> [dir]`",
                );
                return Ok(());
            }
            let rows: Vec<Vec<String>> = mappings
                .mappings
                .iter()
                .map(|m| {
                    vec![
                        m.dir.trim_end_matches('/').to_string(),
                        m.profile.clone(),
                        if m.case_insensitive { "i" } else { "" }.to_string(),
                    ]
                })
                .collect();
            print!(
                "{}",
                output::table(&["DIRECTORY", "PROFILE", "ICASE"], &rows)
            );
        }
    }
    Ok(())
}

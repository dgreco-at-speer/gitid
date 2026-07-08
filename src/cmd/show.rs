//! `gitid show`.

use anyhow::{Result, bail};

use crate::cli::{DetailFormat, ShowArgs};
use crate::cmd::Ctx;
use crate::store::mappings::MappingsFile;
use crate::store::profiles::{self, SigningFormat};

pub fn run(ctx: &Ctx, args: &ShowArgs) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    let Some(profile) = profiles.profiles.get(&args.name) else {
        bail!(
            "{}",
            crate::output::no_such_profile(
                &args.name,
                profiles.profiles.keys().map(String::as_str)
            )
        );
    };

    match args.format {
        DetailFormat::Json => {
            println!("{}", serde_json::to_string_pretty(profile)?);
        }
        DetailFormat::Pretty => {
            use crate::output::dim;
            println!("{} {}", dim("profile:"), args.name);
            println!("  {}   {}", dim("name:"), profile.name);
            println!("  {}  {}", dim("email:"), profile.email);
            if let Some(ssh) = &profile.ssh {
                println!("  {}    {}", dim("ssh:"), ssh.key);
            }
            if let Some(signing) = &profile.signing {
                let fmt = match signing.format {
                    SigningFormat::Ssh => "ssh",
                    SigningFormat::Openpgp => "openpgp",
                };
                println!(
                    "  {} {fmt} key={} commits={}",
                    dim("signing:"),
                    signing.key,
                    signing.commits
                );
            }
            println!(
                "  {}     {}",
                dim("gh:"),
                if profile.gh_enabled() {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            println!(
                "  {} {}",
                dim("fragment:"),
                ctx.paths.fragment(&args.name).display()
            );

            let mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;
            let dirs: Vec<&str> = mappings
                .mappings
                .iter()
                .filter(|m| m.profile == args.name)
                .map(|m| m.dir.as_str())
                .collect();
            if !dirs.is_empty() {
                println!("  {}", crate::output::dim("directories:"));
                for d in dirs {
                    println!("    {d}");
                }
            }
        }
    }
    Ok(())
}

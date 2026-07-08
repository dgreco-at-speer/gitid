//! `gitid add` — create a new profile. The interactive wizard lands in Phase 6;
//! the non-interactive path builds a profile from flags.

use anyhow::{Result, bail};

use crate::cli::{AddArgs, SigningKind};
use crate::cmd::Ctx;
use crate::output;
use crate::store::profiles::{self, Gh, Profile, Signing, SigningFormat, Ssh};
use crate::sync::sync_all;

pub fn run(ctx: &Ctx, args: &AddArgs) -> Result<()> {
    let name = resolve_name(args)?;
    profiles::validate_name(&name)?;

    let path = ctx.paths.profiles_toml();
    let existing = profiles::load(&path)?;
    if existing.profiles.contains_key(&name) {
        bail!("profile {name:?} already exists; use `gitid edit {name}`");
    }

    let profile = if args.non_interactive {
        build_non_interactive(args)?
    } else {
        crate::wizard::add_wizard(ctx, args)?
    };

    let mut doc = profiles::load_doc(&path)?;
    profiles::upsert_profile(&mut doc, &name, &profile)?;
    profiles::save_doc(&path, &doc)?;

    let report = output::with_spinner("syncing…", || sync_all(&ctx.paths))?;
    output::success(&format!("added profile {name:?}"));
    crate::cmd::sync::report_changes(&report);
    Ok(())
}

fn resolve_name(args: &AddArgs) -> Result<String> {
    if let Some(n) = &args.name {
        return Ok(n.clone());
    }
    if !args.non_interactive && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        return Ok(inquire::Text::new("Profile name (slug):").prompt()?);
    }
    bail!("a profile name is required (e.g. `gitid add work ...`)")
}

/// Build a profile purely from flags, erroring on missing required fields.
pub fn build_non_interactive(args: &AddArgs) -> Result<Profile> {
    let git_name = args
        .git_name
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--git-name is required in non-interactive mode"))?;
    let email = args
        .email
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--email is required in non-interactive mode"))?;

    let ssh = args.ssh_key.clone().map(|key| Ssh { key });

    let signing = match args.signing {
        None | Some(SigningKind::None) => None,
        Some(kind) => {
            let format = match kind {
                SigningKind::Ssh => SigningFormat::Ssh,
                SigningKind::Openpgp => SigningFormat::Openpgp,
                SigningKind::None => unreachable!(),
            };
            let key = args.signing_key.clone().ok_or_else(|| {
                anyhow::anyhow!("--signing-key is required when --signing is ssh or openpgp")
            })?;
            Some(Signing {
                format,
                key,
                commits: args.sign_commits,
                tags: None,
            })
        }
    };

    let gh = if args.no_gh {
        None
    } else {
        Some(Gh { enabled: true })
    };

    Ok(Profile {
        name: git_name,
        email,
        ssh,
        signing,
        gh,
        env: Default::default(),
        extra: Default::default(),
    })
}

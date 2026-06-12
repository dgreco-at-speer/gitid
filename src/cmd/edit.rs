//! `gitid edit` — modify an existing profile's fields or open the TOML.

use anyhow::{Result, bail};

use crate::cli::{EditArgs, SigningKind};
use crate::cmd::Ctx;
use crate::output;
use crate::store::profiles::{self, Signing, SigningFormat, Ssh};
use crate::sync::sync_all;

pub fn run(ctx: &Ctx, args: &EditArgs) -> Result<()> {
    let path = ctx.paths.profiles_toml();
    let mut file = profiles::load(&path)?;
    let candidates: Vec<String> = file.profiles.keys().cloned().collect();
    let Some(profile) = file.profiles.get_mut(&args.name) else {
        bail!(
            "{}",
            crate::output::no_such_profile(&args.name, candidates.iter().map(String::as_str))
        );
    };

    if args.open {
        // Opening an editor is interactive; deferred to the wizard phase.
        bail!(
            "--open is not implemented yet; edit {} directly, then run `gitid sync`",
            path.display()
        );
    }

    if let Some(v) = &args.git_name {
        profile.name = v.clone();
    }
    if let Some(v) = &args.email {
        profile.email = v.clone();
    }
    if let Some(v) = &args.ssh_key {
        profile.ssh = Some(Ssh { key: v.clone() });
    }
    if let Some(kind) = args.signing {
        match kind {
            SigningKind::None => profile.signing = None,
            SigningKind::Ssh | SigningKind::Openpgp => {
                let format = if kind == SigningKind::Ssh {
                    SigningFormat::Ssh
                } else {
                    SigningFormat::Openpgp
                };
                let key = args
                    .signing_key
                    .clone()
                    .or_else(|| profile.signing.as_ref().map(|s| s.key.clone()))
                    .ok_or_else(|| anyhow::anyhow!("--signing-key is required to set signing"))?;
                let commits = profile.signing.as_ref().map(|s| s.commits).unwrap_or(false);
                profile.signing = Some(Signing {
                    format,
                    key,
                    commits,
                    tags: profile.signing.as_ref().and_then(|s| s.tags),
                });
            }
        }
    }

    let updated = profile.clone();
    let mut doc = profiles::load_doc(&path)?;
    profiles::upsert_profile(&mut doc, &args.name, &updated)?;
    profiles::save_doc(&path, &doc)?;

    let report = sync_all(&ctx.paths)?;
    output::success(&format!("updated profile {:?}", args.name));
    crate::cmd::sync::report_changes(&report);
    Ok(())
}

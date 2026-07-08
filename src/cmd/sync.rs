//! `gitid sync` and `gitid init`.

use anyhow::Result;

use crate::cmd::Ctx;
use crate::output;
use crate::sync::{SyncReport, sync_all};

pub fn run(ctx: &Ctx) -> Result<()> {
    let report = output::with_spinner("syncing…", || sync_all(&ctx.paths))?;
    report_changes(&report);
    Ok(())
}

pub fn init(ctx: &Ctx) -> Result<()> {
    std::fs::create_dir_all(&ctx.paths.config_dir)?;
    std::fs::create_dir_all(&ctx.paths.data_dir)?;
    let report = output::with_spinner("initialising…", || sync_all(&ctx.paths))?;
    output::success(&format!(
        "initialised gitid (config: {}, data: {})",
        ctx.paths.config_dir.display(),
        ctx.paths.data_dir.display()
    ));
    report_changes(&report);
    output::info("run `gitid setup` to install the shell hook");
    Ok(())
}

/// Print the report and any warnings. Shared by every mutating command.
pub fn report_changes(report: &SyncReport) {
    for profile in &report.dangling_mappings {
        output::warn(&format!(
            "mapping references unknown profile {profile:?}; run `gitid use` or remove it"
        ));
    }
    if report.global_include_added {
        output::success("added gitid include to your global gitconfig");
    }
    if report.is_noop() {
        return;
    }
    let mut bits = Vec::new();
    if !report.fragments_written.is_empty() {
        bits.push(format!(
            "{} fragment(s) written",
            report.fragments_written.len()
        ));
    }
    if !report.fragments_pruned.is_empty() {
        bits.push(format!("{} pruned", report.fragments_pruned.len()));
    }
    if !report.gh_dirs_created.is_empty() {
        bits.push(format!(
            "{} gh dir(s) created",
            report.gh_dirs_created.len()
        ));
    }
    if report.include_changed {
        bits.push("include regenerated".to_string());
    }
    if !bits.is_empty() {
        output::success(&format!("synced: {}", bits.join(", ")));
    }
}

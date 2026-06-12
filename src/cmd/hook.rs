//! `gitid hook <shell>` — print the hook script to eval/source.

use anyhow::Result;

use crate::cli::HookArgs;
use crate::shell::hook;

pub fn run(args: &HookArgs) -> Result<()> {
    print!("{}", hook::script(args.shell));
    Ok(())
}

//! Emission of the per-shell hook scripts. Templates are embedded at build time
//! and have their `{{GITID}}` placeholder replaced with the command name.

use crate::cli::Shell;

const BASH: &str = include_str!("templates/hook.bash");
const ZSH: &str = include_str!("templates/hook.zsh");
const FISH: &str = include_str!("templates/hook.fish");
const PS1: &str = include_str!("templates/hook.ps1");
const NU: &str = include_str!("templates/hook.nu");

/// The hook script for `shell`, ready to eval/source.
pub fn script(shell: Shell) -> String {
    let template = match shell {
        Shell::Bash => BASH,
        Shell::Zsh => ZSH,
        Shell::Fish => FISH,
        Shell::Powershell => PS1,
        Shell::Nu => NU,
    };
    template.replace("{{GITID}}", "gitid")
}

/// The line a user adds to their shell rc file to enable the hook, and the
/// conventional rc-file path (relative to home where applicable).
pub fn install_line(shell: Shell) -> &'static str {
    match shell {
        Shell::Bash => "eval \"$(gitid hook bash)\"",
        Shell::Zsh => "eval \"$(gitid hook zsh)\"",
        Shell::Fish => "gitid hook fish | source",
        Shell::Powershell => "Invoke-Expression (& gitid hook powershell | Out-String)",
        Shell::Nu => "gitid hook nu | save -f ($nu.data-dir | path join vendor/autoload/gitid.nu)",
    }
}

/// The conventional rc file for `shell`, relative to the user's home directory.
pub fn rc_file(shell: Shell) -> &'static str {
    match shell {
        Shell::Bash => ".bashrc",
        Shell::Zsh => ".zshrc",
        Shell::Fish => ".config/fish/config.fish",
        Shell::Powershell => ".config/powershell/Microsoft.PowerShell_profile.ps1",
        Shell::Nu => ".config/nushell/vendor/autoload/gitid.nu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_is_substituted() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::Powershell,
            Shell::Nu,
        ] {
            let s = script(shell);
            assert!(!s.contains("{{GITID}}"), "{shell:?} still has placeholder");
            assert!(s.contains("gitid env"), "{shell:?} missing env call");
            assert!(
                s.contains("GITID_HOOK_ACTIVE"),
                "{shell:?} missing GITID_HOOK_ACTIVE marker"
            );
        }
    }
}

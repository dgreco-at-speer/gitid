//! Command-line surface (clap derive). The dispatch lives in [`crate::cmd`].

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "gitid",
    version,
    about = "Switch between git identities per directory",
    long_about = "gitid manages named git identities (name/email, SSH key, commit signing, \
                  GitHub CLI auth) and assigns them to directory trees via git conditional \
                  includes — without touching any repo's local config."
)]
pub struct Cli {
    /// Disable coloured output.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Increase verbosity (repeatable).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List profiles.
    #[command(alias = "ls")]
    List(ListArgs),
    /// Add a new profile.
    #[command(alias = "new")]
    Add(AddArgs),
    /// Show a profile's details.
    Show(ShowArgs),
    /// Edit a profile.
    Edit(EditArgs),
    /// Remove a profile.
    #[command(alias = "rm")]
    Remove(RemoveArgs),
    /// Assign a profile to a directory tree.
    Use(UseArgs),
    /// Remove the mapping for a directory.
    Forget(ForgetArgs),
    /// List directory→profile mappings.
    Dirs(DirsArgs),
    /// Show the profile active for a directory.
    #[command(alias = "whoami")]
    Current(CurrentArgs),
    /// Diagnose configuration problems.
    Doctor(DoctorArgs),
    /// Regenerate all derived files from the stores.
    Sync,
    /// Print shell activation for the current directory (used by hooks).
    Env(EnvArgs),
    /// Print the shell hook script to eval/source.
    Hook(HookArgs),
    /// Install the shell hook into your shell's rc file.
    Setup(SetupArgs),
    /// Print shell completions.
    Completions(CompletionsArgs),
    /// Create config dirs and bootstrap the global include.
    Init,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Names,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DetailFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CurrentFormat {
    Pretty,
    Json,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SigningKind {
    Ssh,
    Openpgp,
    None,
}

/// Supported shells for hooks, env output, completions, and setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    #[value(alias = "pwsh")]
    Powershell,
    #[value(alias = "nushell")]
    Nu,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Profile name (lowercase slug). Prompted if omitted in interactive mode.
    pub name: Option<String>,
    /// git user.name.
    #[arg(long = "git-name")]
    pub git_name: Option<String>,
    /// git user.email.
    #[arg(long = "email")]
    pub email: Option<String>,
    /// Path to the SSH private key.
    #[arg(long = "ssh-key")]
    pub ssh_key: Option<String>,
    /// Commit-signing method.
    #[arg(long, value_enum)]
    pub signing: Option<SigningKind>,
    /// Signing key: public-key path (ssh) or key id (openpgp).
    #[arg(long = "signing-key")]
    pub signing_key: Option<String>,
    /// Sign commits by default.
    #[arg(long = "sign-commits")]
    pub sign_commits: bool,
    /// Provision an isolated GH_CONFIG_DIR for this profile.
    #[arg(long = "gh", overrides_with = "no_gh")]
    pub gh: bool,
    /// Do not provision a gh config dir.
    #[arg(long = "no-gh")]
    pub no_gh: bool,
    /// Fail rather than prompt for missing fields.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    pub name: String,
    #[arg(long, value_enum, default_value_t = DetailFormat::Pretty)]
    pub format: DetailFormat,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    pub name: String,
    #[arg(long = "git-name")]
    pub git_name: Option<String>,
    #[arg(long = "email")]
    pub email: Option<String>,
    #[arg(long = "ssh-key")]
    pub ssh_key: Option<String>,
    #[arg(long, value_enum)]
    pub signing: Option<SigningKind>,
    #[arg(long = "signing-key")]
    pub signing_key: Option<String>,
    /// Open profiles.toml in $EDITOR, then sync.
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    pub name: String,
    /// Remove even if directories are mapped to it (and drop those mappings).
    #[arg(long, short)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct UseArgs {
    pub profile: String,
    /// Directory to assign (default: current directory).
    pub dir: Option<String>,
    /// Force case-insensitive directory matching.
    #[arg(long = "icase", overrides_with = "no_icase")]
    pub icase: bool,
    /// Force case-sensitive directory matching.
    #[arg(long = "no-icase")]
    pub no_icase: bool,
}

#[derive(Debug, Args)]
pub struct ForgetArgs {
    /// Directory to unassign (default: current directory).
    pub dir: Option<String>,
}

#[derive(Debug, Args)]
pub struct DirsArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
}

#[derive(Debug, Args)]
pub struct CurrentArgs {
    /// Directory to inspect (default: current directory).
    pub dir: Option<String>,
    #[arg(long, value_enum, default_value_t = CurrentFormat::Pretty)]
    pub format: CurrentFormat,
    /// Print nothing; exit 0 if a profile is active, 1 otherwise.
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Directory to inspect (default: current directory).
    pub dir: Option<String>,
}

#[derive(Debug, Args)]
pub struct EnvArgs {
    #[arg(short, long, value_enum)]
    pub shell: Shell,
    /// Resolve a specific directory instead of the current one.
    #[arg(long)]
    pub dir: Option<String>,
}

#[derive(Debug, Args)]
pub struct HookArgs {
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(Debug, Args)]
pub struct SetupArgs {
    #[arg(value_enum)]
    pub shell: Option<Shell>,
    /// Only print the line to add; do not modify any file.
    #[arg(long)]
    pub print: bool,
    /// Append to the rc file without prompting.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    pub shell: Shell,
}

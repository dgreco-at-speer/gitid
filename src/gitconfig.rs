//! Generation of gitconfig text: profile fragments, the `includeIf` manifest,
//! and the escaping/pattern rules that make conditional includes resolve
//! correctly. Everything in this file is pure; the git-subprocess and
//! global-bootstrap helpers live alongside in [`crate::gitconfig::git`] (Phase 2).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::paths::{GitidPaths, PathStyle, contract_home, expand_tilde, normalize_dir};
use crate::store::mappings::Mapping;
use crate::store::profiles::{Profile, SIGNING_KEY_AGENT, SigningFormat, Ssh};

/// Quote a gitconfig *value* if it needs it. Values containing `#`, `;`, `"`,
/// `\`, or leading/trailing whitespace are wrapped in double quotes with `\` and
/// `"` escaped.
pub fn quote_git_value(value: &str) -> String {
    let needs_quote = value.is_empty()
        || value.starts_with(char::is_whitespace)
        || value.ends_with(char::is_whitespace)
        || value.chars().any(|c| matches!(c, '#' | ';' | '"' | '\\'));
    if !needs_quote {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Escape wildmatch metacharacters that appear *literally* in a path so git's
/// `gitdir:` glob treats them as ordinary characters.
fn escape_wildmatch(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            '*' => out.push_str("[*]"),
            '?' => out.push_str("[?]"),
            '[' => out.push_str("[[]"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape a string for use inside a gitconfig section-header subsection (the
/// `"..."` part of `[includeIf "..."]`): backslash and double-quote.
fn escape_section_header(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

/// Build the `gitdir:` / `gitdir/i:` condition for a directory.
///
/// The directory is normalised to forward slashes with a single trailing slash
/// (git appends `**`, so the pattern matches every repo in the tree), its home
/// prefix is contracted to `~/` for portability, and literal glob metacharacters
/// are escaped. The returned string is the raw condition, not yet header-quoted.
pub fn gitdir_pattern(dir: &str, icase: bool, home: &str, style: PathStyle) -> String {
    let norm = normalize_dir(dir, style);
    let contracted = contract_home(&norm, home, style);
    // contract_home may have dropped the trailing slash semantics; re-ensure it.
    let with_slash = if contracted.ends_with('/') {
        contracted
    } else {
        format!("{contracted}/")
    };
    let escaped = escape_wildmatch(&with_slash);
    let keyword = if icase { "gitdir/i:" } else { "gitdir:" };
    format!("{keyword}{escaped}")
}

/// Render a profile's gitconfig fragment. SSH and signing key paths are expanded
/// to absolute (a quoted `~` inside `core.sshCommand` would not be shell-expanded).
/// Agent-held keys point at the public key `gitid sync` materialises under the
/// data dir; ssh resolves the private half via the agent.
pub fn render_fragment(name: &str, profile: &Profile, paths: &GitidPaths) -> String {
    let home = &paths.home;
    let mut out = String::new();
    out.push_str("# Managed by gitid. Do not edit; run `gitid sync` to regenerate.\n");

    out.push_str("[user]\n");
    out.push_str(&format!("\tname = {}\n", quote_git_value(&profile.name)));
    out.push_str(&format!("\temail = {}\n", quote_git_value(&profile.email)));
    if let Some(signing) = &profile.signing {
        let key = match signing.format {
            SigningFormat::Ssh if signing.key == SIGNING_KEY_AGENT => derived_pub_str(name, paths),
            SigningFormat::Ssh => abs_path_str(&signing.key, home),
            SigningFormat::Openpgp => signing.key.clone(),
        };
        out.push_str(&format!("\tsigningkey = {}\n", quote_git_value(&key)));
    }

    if let Some(signing) = &profile.signing {
        let fmt = match signing.format {
            SigningFormat::Ssh => "ssh",
            SigningFormat::Openpgp => "openpgp",
        };
        out.push_str("[gpg]\n");
        out.push_str(&format!("\tformat = {fmt}\n"));
        if signing.commits {
            out.push_str("[commit]\n\tgpgsign = true\n");
        }
        if signing.tags == Some(true) {
            out.push_str("[tag]\n\tgpgsign = true\n");
        }
    }

    if let Some(ssh) = &profile.ssh {
        let key = match ssh {
            Ssh::Path(p) => abs_path_str(&p.key, home),
            Ssh::Agent(_) => derived_pub_str(name, paths),
        };
        let cmd = format!("ssh -i {} -o IdentitiesOnly=yes", shell_quote_arg(&key));
        out.push_str("[core]\n");
        out.push_str(&format!("\tsshCommand = {}\n", quote_git_value(&cmd)));
    }

    // Raw passthrough. Keys are `section.key` or `section.subsection.key`.
    for (key, value) in &profile.extra {
        if let Some((section, name)) = split_config_key(key) {
            out.push_str(&format!("[{section}]\n"));
            out.push_str(&format!("\t{name} = {}\n", quote_git_value(value)));
        }
    }

    out
}

/// Expand a leading `~` and return a forward-slashed absolute path string for
/// embedding into a gitconfig value.
fn abs_path_str(input: &str, home: &Path) -> String {
    let expanded = expand_tilde(input, home);
    let s = expanded.to_string_lossy().into_owned();
    // Forward slashes are accepted by git and ssh on every platform.
    s.replace('\\', "/")
}

/// The derived public-key path for an agent-held key, as a gitconfig value.
fn derived_pub_str(name: &str, paths: &GitidPaths) -> String {
    paths.ssh_pub(name).to_string_lossy().replace('\\', "/")
}

/// Quote an argument for the POSIX shell git runs `core.sshCommand` through.
/// Only quotes when needed (whitespace or shell-special characters).
fn shell_quote_arg(arg: &str) -> String {
    let special = |c: char| {
        c.is_whitespace() || matches!(c, '\'' | '"' | '\\' | '$' | '`' | '&' | ';' | '(' | ')')
    };
    if !arg.is_empty() && !arg.chars().any(special) {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    for c in arg.chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Split a flattened config key like `core.autocrlf` or
/// `url.git@github.com:.insteadOf` into a header section and trailing key.
///
/// The section is everything up to the *last* `.`; the key is the remainder.
/// For a subsection (`url.<x>.insteadOf`) git wants `[url "<x>"]`, but to keep
/// the passthrough simple we emit `[url.<x>]` form is invalid — so we quote the
/// subsection when present.
fn split_config_key(key: &str) -> Option<(String, &str)> {
    let (head, last) = key.rsplit_once('.')?;
    match head.split_once('.') {
        // section.subsection -> [section "subsection"]
        Some((section, subsection)) => Some((
            format!("{section} \"{}\"", escape_section_header(subsection)),
            last,
        )),
        // plain section
        None => Some((head.to_string(), last)),
    }
}

/// Render `include.gitconfig`: one `includeIf` block per mapping, pointing at the
/// profile fragment via a path relative to the data dir. Mappings must already be
/// ordered ascending by directory length so longer (more specific) trees win.
pub fn render_include(mappings: &[Mapping], home: &str, style: PathStyle) -> String {
    let mut out = String::new();
    out.push_str("# Managed by gitid. Do not edit; run `gitid sync` to regenerate.\n");
    for m in mappings {
        let frag = format!("profiles/{}.gitconfig", m.profile);
        emit_include_block(&mut out, &m.dir, m.case_insensitive, home, style, &frag);
        // A symlinked tree the user named differently: belt-and-suspenders.
        if let Some(literal) = &m.dir_literal {
            if literal != &m.dir {
                emit_include_block(&mut out, literal, m.case_insensitive, home, style, &frag);
            }
        }
    }
    out
}

fn emit_include_block(
    out: &mut String,
    dir: &str,
    icase: bool,
    home: &str,
    style: PathStyle,
    fragment_rel: &str,
) {
    let pattern = gitdir_pattern(dir, icase, home, style);
    out.push_str(&format!(
        "[includeIf \"{}\"]\n\tpath = {}\n",
        escape_section_header(&pattern),
        fragment_rel
    ));
}

// ---------------------------------------------------------------------------
// git subprocess + global-include bootstrap (impure)
// ---------------------------------------------------------------------------

/// Minimum git version supporting `includeIf` and `gitdir/i`.
pub const MIN_GIT: (u32, u32) = (2, 13);

/// Parse `git --version` into `(major, minor)`.
pub fn git_version() -> Result<(u32, u32)> {
    let out = Command::new("git")
        .arg("--version")
        .output()
        .context("could not run `git --version` (is git installed?)")?;
    let text = String::from_utf8_lossy(&out.stdout);
    parse_git_version(&text).with_context(|| format!("could not parse git version from {text:?}"))
}

fn parse_git_version(text: &str) -> Option<(u32, u32)> {
    // "git version 2.54.0" (possibly with vendor suffixes)
    let nums = text.split_whitespace().find(|t| t.contains('.'))?;
    let mut parts = nums.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// Resolve the global gitconfig path git would *write* to, mirroring git's own
/// precedence: `$GIT_CONFIG_GLOBAL`, else `~/.gitconfig` if it exists, else
/// `$XDG_CONFIG_HOME/git/config` if it exists, else `~/.gitconfig`.
pub fn resolve_global_path(home: &Path) -> PathBuf {
    if let Some(p) = std::env::var_os("GIT_CONFIG_GLOBAL") {
        return PathBuf::from(p);
    }
    let dotfile = home.join(".gitconfig");
    if dotfile.exists() {
        return dotfile;
    }
    let xdg = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("git")
        .join("config");
    if xdg.exists() {
        return xdg;
    }
    dotfile
}

/// Whether gitid can safely write to `path` in place. A missing file counts as
/// writable (it will be created); an existing file is writable only if it can be
/// opened for writing. A gitconfig managed read-only by a config manager
/// (Home-Manager / Nix symlink it into the store) fails here — the signal to
/// divert to a writable `.local` companion rather than clobber it.
fn is_writable_in_place(path: &Path) -> bool {
    match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(_) => true,
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    }
}

/// The writable companion path gitid diverts to when the resolved global
/// gitconfig is read-only: the same file with `.local` appended
/// (`~/.config/git/config` → `~/.config/git/config.local`, `~/.gitconfig` →
/// `~/.gitconfig.local`). For git to load it, the managed global must include it.
pub fn local_companion(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(std::ffi::OsString::from)
        .unwrap_or_default();
    name.push(".local");
    path.with_file_name(name)
}

/// Whether `global` (a specific gitconfig file) already includes `include_path`
/// (compared after `~` expansion). Reads the file by path via `git config
/// --file` so the check is deterministic regardless of process environment, with
/// a string-scan fallback.
fn global_include_present(global: &Path, home: &Path, include_path: &Path) -> Result<bool> {
    if !global.exists() {
        return Ok(false);
    }
    let out = Command::new("git")
        .arg("config")
        .arg("--file")
        .arg(global)
        .args(["--get-all", "include.path"])
        .output()
        .context("could not run `git config`")?;
    if out.status.success() {
        let target = include_path.to_path_buf();
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if expand_tilde(line.trim(), home) == target {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    // Non-zero usually means the key is absent. Fall back to scanning the file.
    match std::fs::read_to_string(global) {
        Ok(text) => Ok(text.lines().any(|l| l.contains("include.gitconfig"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("could not read {}", global.display())),
    }
}

/// Outcome of [`ensure_include`].
#[derive(Debug, Clone)]
pub struct IncludeOutcome {
    /// Whether a new `[include]` block was appended (false = already present).
    pub added: bool,
    /// The file gitid wrote to (or found the include already in).
    pub target: PathBuf,
    /// The read-only resolved global gitconfig, set when gitid diverted to a
    /// writable `.local` companion. `None` on the normal path.
    pub diverted_from: Option<PathBuf>,
}

/// Ensure the user's global gitconfig includes our manifest, exactly once.
///
/// Appends an `[include]` block at EOF (never via `git config --add`, which
/// would insert into the first existing `[include]` section and could let a
/// global `[user]` defined later win over our profile fragments).
///
/// When the resolved global gitconfig is read-only (e.g. managed by
/// Home-Manager / Nix, which symlink it into the store), gitid does not clobber
/// it: it diverts to a writable `.local` companion ([`local_companion`]) and
/// records the divert in the returned [`IncludeOutcome`], so callers can prompt
/// the user to wire the companion into their managed config.
pub fn ensure_include(home: &Path, include_path: &Path) -> Result<IncludeOutcome> {
    let global = resolve_global_path(home);
    let (target, diverted_from) = if is_writable_in_place(&global) {
        (global, None)
    } else {
        (local_companion(&global), Some(global))
    };

    if global_include_present(&target, home, include_path)? {
        return Ok(IncludeOutcome {
            added: false,
            target,
            diverted_from,
        });
    }
    let display_path = {
        let abs = include_path.to_string_lossy();
        contract_home(&abs, &home.to_string_lossy(), PathStyle::host())
    };

    let mut content = match std::fs::read_to_string(&target) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("could not read {}", target.display())),
    };
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    if !content.is_empty() {
        content.push('\n');
    }
    content.push_str("# Added by gitid.\n");
    content.push_str("[include]\n");
    content.push_str(&format!("\tpath = {display_path}\n"));

    crate::store::atomic_write(&target, &content)
        .with_context(|| format!("could not update {}", target.display()))?;
    Ok(IncludeOutcome {
        added: true,
        target,
        diverted_from,
    })
}

/// Whether git, resolving config the way it does in the current environment,
/// actually loads our manifest via some `include.path`. Unlike
/// [`global_include_present`] (which inspects a single file), this reflects what
/// git truly sees — including a `.local` companion pulled in by a managed global
/// config. Used to decide whether a diverted include is actually effective.
pub fn effective_include_present(home: &Path, include_path: &Path) -> Result<bool> {
    let out = Command::new("git")
        .args(["config", "--get-all", "include.path"])
        .output()
        .context("could not run `git config`")?;
    if !out.status.success() {
        return Ok(false);
    }
    let target = include_path.to_path_buf();
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .any(|line| expand_tilde(line.trim(), home) == target))
}

/// Read-only classification of whether git will load the gitid manifest,
/// mirroring [`ensure_include`]'s divert logic. Consumed by `doctor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapStatus {
    /// The resolved global gitconfig includes the manifest (normal case).
    Present,
    /// The global is read-only; gitid diverted to `local`, and git loads it.
    PresentViaLocal { local: PathBuf },
    /// gitid diverted to `local`, but git does not load it — the managed global
    /// does not include the companion.
    LocalNotLoaded { global: PathBuf, local: PathBuf },
    /// The manifest is not included anywhere.
    Missing,
}

/// Classify the global-include bootstrap state for `doctor`.
pub fn bootstrap_status(home: &Path, include_path: &Path) -> Result<BootstrapStatus> {
    let global = resolve_global_path(home);
    if global_include_present(&global, home, include_path)? {
        return Ok(BootstrapStatus::Present);
    }
    let local = local_companion(&global);
    if global_include_present(&local, home, include_path)? {
        return if effective_include_present(home, include_path)? {
            Ok(BootstrapStatus::PresentViaLocal { local })
        } else {
            Ok(BootstrapStatus::LocalNotLoaded { global, local })
        };
    }
    Ok(BootstrapStatus::Missing)
}

/// `git -C <dir> config --get <key>`, returning `None` when unset.
pub fn config_get(dir: &Path, key: &str) -> Result<Option<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "--get", key])
        .output()
        .context("could not run `git config`")?;
    if out.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&out.stdout).trim_end().to_string(),
        ))
    } else {
        Ok(None)
    }
}

/// `git -C <dir> config --show-origin --get <key>`, returning `(value, origin)`.
pub fn config_get_with_origin(dir: &Path, key: &str) -> Result<Option<(String, String)>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "--show-origin", "--get", key])
        .output()
        .context("could not run `git config`")?;
    if !out.status.success() {
        return Ok(None);
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let line = line.trim_end();
    // Format: "<origin>\t<value>"
    if let Some((origin, value)) = line.split_once('\t') {
        Ok(Some((value.to_string(), origin.to_string())))
    } else {
        Ok(Some((line.to_string(), String::new())))
    }
}

/// Whether `dir` resolves into a git repository.
pub fn is_in_repo(dir: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::profiles::{Gh, Signing, Ssh};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn git_version_parsing() {
        assert_eq!(parse_git_version("git version 2.54.0"), Some((2, 54)));
        assert_eq!(
            parse_git_version("git version 2.39.3 (Apple Git-146)"),
            Some((2, 39))
        );
        assert_eq!(
            parse_git_version("git version 2.13.0.windows.1"),
            Some((2, 13))
        );
        assert_eq!(parse_git_version("no version here"), None);
    }

    #[test]
    fn quote_values() {
        assert_eq!(quote_git_value("Jane Doe"), "Jane Doe");
        assert_eq!(quote_git_value("a#b"), "\"a#b\"");
        assert_eq!(quote_git_value("a;b"), "\"a;b\"");
        assert_eq!(quote_git_value(" lead"), "\" lead\"");
        assert_eq!(quote_git_value("back\\slash"), "\"back\\\\slash\"");
        assert_eq!(quote_git_value("quo\"te"), "\"quo\\\"te\"");
        assert_eq!(quote_git_value(""), "\"\"");
    }

    #[test]
    fn pattern_home_relative() {
        assert_eq!(
            gitdir_pattern("/home/jane/code/work", false, "/home/jane", PathStyle::Unix),
            "gitdir:~/code/work/"
        );
    }

    #[test]
    fn pattern_absolute_outside_home() {
        assert_eq!(
            gitdir_pattern("/mnt/work", false, "/home/jane", PathStyle::Unix),
            "gitdir:/mnt/work/"
        );
    }

    #[test]
    fn pattern_icase() {
        assert_eq!(
            gitdir_pattern("/home/jane/work", true, "/home/jane", PathStyle::Unix),
            "gitdir/i:~/work/"
        );
    }

    #[test]
    fn pattern_windows_drive() {
        assert_eq!(
            gitdir_pattern(
                "C:\\Users\\Jane\\code",
                true,
                "C:\\Users\\Jane",
                PathStyle::Windows
            ),
            "gitdir/i:~/code/"
        );
    }

    #[test]
    fn pattern_escapes_metachars() {
        assert_eq!(
            gitdir_pattern("/home/jane/wo*rk", false, "/home/jane", PathStyle::Unix),
            "gitdir:~/wo[*]rk/"
        );
        assert_eq!(
            gitdir_pattern("/home/jane/a[b]", false, "/home/jane", PathStyle::Unix),
            "gitdir:~/a[[]b]/"
        );
    }

    fn profile_all() -> Profile {
        Profile {
            name: "Jane Doe".into(),
            email: "jane@corp.example".into(),
            ssh: Some(Ssh::from_path("~/.ssh/id_work")),
            signing: Some(Signing {
                format: SigningFormat::Ssh,
                key: "~/.ssh/id_work.pub".into(),
                commits: true,
                tags: Some(true),
            }),
            gh: Some(Gh { enabled: true }),
            env: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }

    fn test_paths(home: &str) -> GitidPaths {
        let home = PathBuf::from(home);
        GitidPaths {
            config_dir: home.join(".config/gitid"),
            data_dir: home.join(".local/share/gitid"),
            home,
        }
    }

    #[test]
    fn fragment_full() {
        let frag = render_fragment("work", &profile_all(), &test_paths("/home/jane"));
        assert!(frag.contains("name = Jane Doe"));
        assert!(frag.contains("email = jane@corp.example"));
        assert!(frag.contains("signingkey = /home/jane/.ssh/id_work.pub"));
        assert!(frag.contains("format = ssh"));
        assert!(frag.contains("[commit]\n\tgpgsign = true"));
        assert!(frag.contains("[tag]\n\tgpgsign = true"));
        // Internal whitespace needs no quoting in a gitconfig value.
        assert!(frag.contains("sshCommand = ssh -i /home/jane/.ssh/id_work -o IdentitiesOnly=yes"));
    }

    #[test]
    fn fragment_minimal() {
        let p = Profile {
            name: "Pat".into(),
            email: "pat@example.com".into(),
            ssh: None,
            signing: None,
            gh: None,
            env: BTreeMap::new(),
            extra: BTreeMap::new(),
        };
        let frag = render_fragment("pat", &p, &test_paths("/home/pat"));
        assert!(frag.contains("name = Pat"));
        assert!(!frag.contains("[gpg]"));
        assert!(!frag.contains("[core]"));
    }

    #[test]
    fn fragment_agent_key() {
        let mut p = profile_all();
        p.ssh = Some(Ssh::from_agent("SHA256:abcdef"));
        p.signing = Some(Signing {
            format: SigningFormat::Ssh,
            key: SIGNING_KEY_AGENT.into(),
            commits: true,
            tags: None,
        });
        let frag = render_fragment("work", &p, &test_paths("/home/jane"));
        let pub_path = "/home/jane/.local/share/gitid/ssh/work.pub";
        assert!(
            frag.contains(&format!(
                "sshCommand = ssh -i {pub_path} -o IdentitiesOnly=yes"
            )),
            "{frag}"
        );
        assert!(frag.contains(&format!("signingkey = {pub_path}")), "{frag}");
        assert!(frag.contains("format = ssh"));
        // The selector itself never appears in derived config.
        assert!(!frag.contains("SHA256:abcdef"));
    }

    #[test]
    fn fragment_quotes_paths_with_spaces() {
        let p = profile_all();
        let frag = render_fragment("work", &p, &test_paths("/home/Jane Doe"));
        assert!(
            frag.contains(
                "sshCommand = \"ssh -i \\\"/home/Jane Doe/.ssh/id_work\\\" -o IdentitiesOnly=yes\""
            ),
            "{frag}"
        );
    }

    #[test]
    fn shell_quoting() {
        assert_eq!(shell_quote_arg("/home/jane/.ssh/id"), "/home/jane/.ssh/id");
        assert_eq!(
            shell_quote_arg("/home/Jane Doe/key"),
            "\"/home/Jane Doe/key\""
        );
        assert_eq!(shell_quote_arg("a\"b"), "\"a\\\"b\"");
        assert_eq!(shell_quote_arg("a$b"), "\"a\\$b\"");
        assert_eq!(shell_quote_arg(""), "\"\"");
    }

    #[test]
    fn fragment_openpgp_signing_no_expand() {
        let mut p = profile_all();
        p.signing = Some(Signing {
            format: SigningFormat::Openpgp,
            key: "ABCD1234".into(),
            commits: false,
            tags: None,
        });
        let frag = render_fragment("work", &p, &test_paths("/home/jane"));
        assert!(frag.contains("signingkey = ABCD1234"));
        assert!(frag.contains("format = openpgp"));
        assert!(!frag.contains("[commit]"));
    }

    #[test]
    fn fragment_extra_passthrough() {
        let mut p = profile_all();
        p.extra.insert("core.autocrlf".into(), "input".into());
        p.extra.insert(
            "url.git@github.com-work:.insteadOf".into(),
            "git@github.com:".into(),
        );
        let frag = render_fragment("work", &p, &test_paths("/home/jane"));
        assert!(frag.contains("[core]\n\tautocrlf = input"));
        assert!(frag.contains("[url \"git@github.com-work:\"]\n\tinsteadOf = git@github.com:"));
    }

    #[test]
    fn include_orders_and_relative_paths() {
        let mk = |dir: &str, profile: &str| Mapping {
            dir: normalize_dir(dir, PathStyle::Unix),
            dir_literal: None,
            profile: profile.into(),
            case_insensitive: false,
            env: BTreeMap::new(),
        };
        let mappings = vec![
            mk("/home/jane/code", "personal"),
            mk("/home/jane/code/work", "work"),
        ];
        let inc = render_include(&mappings, "/home/jane", PathStyle::Unix);
        let personal_at = inc.find("personal.gitconfig").unwrap();
        let work_at = inc.find("work.gitconfig").unwrap();
        assert!(personal_at < work_at, "longer dir must come last");
        assert!(
            inc.contains("[includeIf \"gitdir:~/code/\"]\n\tpath = profiles/personal.gitconfig")
        );
        assert!(
            inc.contains("[includeIf \"gitdir:~/code/work/\"]\n\tpath = profiles/work.gitconfig")
        );
    }

    #[test]
    fn include_emits_literal_when_differs() {
        let mappings = vec![Mapping {
            dir: "/mnt/big/work/".into(),
            dir_literal: Some("/home/jane/work/".into()),
            profile: "work".into(),
            case_insensitive: false,
            env: BTreeMap::new(),
        }];
        let inc = render_include(&mappings, "/home/jane", PathStyle::Unix);
        assert!(inc.contains("gitdir:/mnt/big/work/"));
        assert!(inc.contains("gitdir:~/work/"));
        let _ = PathBuf::new();
    }

    #[test]
    fn local_companion_appends_local_suffix() {
        use std::path::Path;
        assert_eq!(
            local_companion(Path::new("/home/jane/.config/git/config")),
            PathBuf::from("/home/jane/.config/git/config.local")
        );
        assert_eq!(
            local_companion(Path::new("/home/jane/.gitconfig")),
            PathBuf::from("/home/jane/.gitconfig.local")
        );
    }
}

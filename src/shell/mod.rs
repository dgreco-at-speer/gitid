//! Per-shell rendering of [`EnvOp`] activation diffs, plus the embedded hook
//! scripts. POSIX shells receive evalable code; nushell receives JSON (it
//! cannot eval strings) that its hook feeds to `load-env`/`hide-env`.

pub mod hook;

use crate::activation::EnvOp;
use crate::cli::Shell;

/// Render a list of env operations into the given shell's syntax. For
/// bash/zsh/fish/powershell this is directly evalable; for nu it is a JSON
/// payload of the form `{"set":{...},"unset":[...]}`. An empty op list renders
/// to an empty string (the no-change fast path).
pub fn render_ops(shell: Shell, ops: &[EnvOp]) -> String {
    if ops.is_empty() {
        return String::new();
    }
    match shell {
        Shell::Bash | Shell::Zsh => render_posix(ops),
        Shell::Fish => render_fish(ops),
        Shell::Powershell => render_pwsh(ops),
        Shell::Nu => render_nu(ops),
    }
}

fn render_posix(ops: &[EnvOp]) -> String {
    let mut out = String::new();
    for op in ops {
        match op {
            EnvOp::Set(k, v) => out.push_str(&format!("export {k}={};\n", quote_posix(v))),
            EnvOp::Unset(k) => out.push_str(&format!("unset {k};\n")),
        }
    }
    out
}

fn render_fish(ops: &[EnvOp]) -> String {
    let mut out = String::new();
    for op in ops {
        match op {
            EnvOp::Set(k, v) => out.push_str(&format!("set -gx {k} {};\n", quote_fish(v))),
            EnvOp::Unset(k) => out.push_str(&format!("set -e {k};\n")),
        }
    }
    out
}

fn render_pwsh(ops: &[EnvOp]) -> String {
    let mut out = String::new();
    for op in ops {
        match op {
            EnvOp::Set(k, v) => out.push_str(&format!("$env:{k} = {}\n", quote_pwsh(v))),
            EnvOp::Unset(k) => out.push_str(&format!(
                "Remove-Item Env:\\{k} -ErrorAction SilentlyContinue\n"
            )),
        }
    }
    out
}

fn render_nu(ops: &[EnvOp]) -> String {
    use serde_json::{Map, Value, json};
    let mut set = Map::new();
    let mut unset = Vec::new();
    for op in ops {
        match op {
            EnvOp::Set(k, v) => {
                set.insert(k.clone(), Value::String(v.clone()));
            }
            EnvOp::Unset(k) => unset.push(Value::String(k.clone())),
        }
    }
    let payload = json!({ "set": Value::Object(set), "unset": Value::Array(unset) });
    payload.to_string()
}

/// POSIX single-quote: wrap in `'…'`, replacing each `'` with `'\''`.
fn quote_posix(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// fish single-quote: wrap in `'…'`, escaping `\` and `'`.
fn quote_fish(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            _ => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// PowerShell single-quote: wrap in `'…'`, doubling each `'`.
fn quote_pwsh(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(k: &str, v: &str) -> EnvOp {
        EnvOp::Set(k.into(), v.into())
    }

    #[test]
    fn empty_renders_empty() {
        assert_eq!(render_ops(Shell::Bash, &[]), "");
        assert_eq!(render_ops(Shell::Nu, &[]), "");
    }

    #[test]
    fn posix_set_and_unset() {
        let ops = vec![set("GH_CONFIG_DIR", "/gh/work"), EnvOp::Unset("X".into())];
        let out = render_ops(Shell::Bash, &ops);
        assert_eq!(out, "export GH_CONFIG_DIR='/gh/work';\nunset X;\n");
    }

    #[test]
    fn posix_quotes_single_quote() {
        let out = render_ops(Shell::Zsh, &[set("K", "a'b")]);
        assert_eq!(out, "export K='a'\\''b';\n");
    }

    #[test]
    fn fish_escapes() {
        let out = render_ops(Shell::Fish, &[set("K", "a'b\\c")]);
        assert_eq!(out, "set -gx K 'a\\'b\\\\c';\n");
    }

    #[test]
    fn pwsh_doubles_quote() {
        let out = render_ops(Shell::Powershell, &[set("K", "a'b")]);
        assert_eq!(out, "$env:K = 'a''b'\n");
    }

    #[test]
    fn pwsh_unset() {
        let out = render_ops(Shell::Powershell, &[EnvOp::Unset("K".into())]);
        assert_eq!(out, "Remove-Item Env:\\K -ErrorAction SilentlyContinue\n");
    }

    #[test]
    fn nu_json_payload() {
        let ops = vec![set("GH_CONFIG_DIR", "/gh/work"), EnvOp::Unset("X".into())];
        let out = render_ops(Shell::Nu, &ops);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["set"]["GH_CONFIG_DIR"], "/gh/work");
        assert_eq!(v["unset"][0], "X");
    }

    #[test]
    fn values_with_spaces_and_unicode() {
        let out = render_ops(Shell::Bash, &[set("K", "a b ☃")]);
        assert_eq!(out, "export K='a b ☃';\n");
    }
}

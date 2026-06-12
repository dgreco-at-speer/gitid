//! Real-shell tests: source the emitted hook, cd into a mapped tree, and assert
//! the environment activates and deactivates. Each shell is detect-and-skip: if
//! the interpreter isn't installed, the test prints a notice and passes.

mod common;

use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;

use common::TestEnv;

fn gitid_bin_dir() -> PathBuf {
    assert_cmd::cargo::cargo_bin("gitid")
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Set up a gh-enabled `work` profile mapped to HOME/code, return that tree.
fn setup(env: &TestEnv) -> PathBuf {
    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--git-name",
            "W",
            "--email",
            "w@x.example",
        ])
        .assert()
        .success();
    let tree = env.home().join("code");
    std::fs::create_dir_all(&tree).unwrap();
    env.gitid()
        .args(["use", "work", tree.to_str().unwrap()])
        .assert()
        .success();
    tree
}

/// Run `program args… snippet` with gitid on PATH and the isolated env.
/// Returns `None` if the interpreter is not installed.
fn run(
    env: &TestEnv,
    program: &str,
    args: &[&str],
    snippet: &str,
    tree: &PathBuf,
) -> Option<String> {
    let path = format!(
        "{}:{}",
        gitid_bin_dir().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let result = Command::new(program)
        .args(args)
        .arg(snippet)
        .env("HOME", env.home())
        .env("TREE", tree)
        .env("GITID_CONFIG_DIR", &env.paths.config_dir)
        .env("GITID_DATA_DIR", &env.paths.data_dir)
        .env("GIT_CONFIG_GLOBAL", env.global_gitconfig())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("PATH", path)
        .output();
    match result {
        Ok(o) => Some(String::from_utf8_lossy(&o.stdout).to_string()),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            eprintln!("skipped: {program} not installed");
            None
        }
        Err(e) => panic!("failed to run {program}: {e}"),
    }
}

fn assert_activates_and_restores(out: &str) {
    let a = out.lines().find(|l| l.starts_with("A=")).unwrap_or("A=");
    let b = out.lines().find(|l| l.starts_with("B=")).unwrap_or("B=");
    assert!(
        a.contains("gh/work"),
        "expected activation in A; got {out:?}"
    );
    assert!(
        !b.contains("gh/work"),
        "expected deactivation in B; got {out:?}"
    );
}

#[test]
fn zsh_hook_activates_on_cd() {
    let env = TestEnv::new();
    let tree = setup(&env);
    let snippet = r#"
eval "$(gitid hook zsh)"
cd "$TREE"; echo "A=$GH_CONFIG_DIR"
cd "$HOME"; echo "B=${GH_CONFIG_DIR:-}"
"#;
    if let Some(out) = run(&env, "zsh", &["-f", "-c"], snippet, &tree) {
        assert_activates_and_restores(&out);
    }
}

#[test]
fn bash_hook_activates_on_cd() {
    let env = TestEnv::new();
    let tree = setup(&env);
    // PROMPT_COMMAND does not run non-interactively, so call the hook directly.
    let snippet = r#"
eval "$(gitid hook bash)"
cd "$TREE"; _gitid_hook; echo "A=$GH_CONFIG_DIR"
cd "$HOME"; _gitid_hook; echo "B=${GH_CONFIG_DIR:-}"
"#;
    if let Some(out) = run(&env, "bash", &["-c"], snippet, &tree) {
        assert_activates_and_restores(&out);
    }
}

#[test]
fn fish_hook_activates_on_cd() {
    let env = TestEnv::new();
    let tree = setup(&env);
    let snippet = r#"
gitid hook fish | source
cd "$TREE"; echo "A=$GH_CONFIG_DIR"
cd "$HOME"; echo "B=$GH_CONFIG_DIR"
"#;
    if let Some(out) = run(&env, "fish", &["--no-config", "-c"], snippet, &tree) {
        assert_activates_and_restores(&out);
    }
}

#[test]
fn nu_env_contract_loads() {
    let env = TestEnv::new();
    let tree = setup(&env);
    // Drive the nu side of the contract: parse JSON and load-env, then read back.
    let snippet = r#"
cd $env.TREE
let out = (^gitid env --shell nu | str trim)
let d = ($out | from json)
load-env $d.set
print $"A=($env.GH_CONFIG_DIR)"
"#;
    if let Some(out) = run(&env, "nu", &["-c"], snippet, &tree) {
        assert!(out.contains("gh/work"), "got: {out:?}");
    }
}

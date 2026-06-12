//! Hook, setup, and completion surface tests.

mod common;

use common::TestEnv;

#[test]
fn hook_outputs_for_every_shell() {
    let env = TestEnv::new();
    for (shell, needle) in [
        ("bash", "gitid env --shell bash"),
        ("zsh", "add-zsh-hook chpwd"),
        ("fish", "--on-variable PWD"),
        ("powershell", "function global:prompt"),
        ("nu", "env_change.PWD"),
    ] {
        env.gitid()
            .args(["hook", shell])
            .assert()
            .success()
            .stdout(predicates::str::contains(needle));
    }
}

#[test]
fn setup_print_shows_install_line() {
    let env = TestEnv::new();
    env.gitid()
        .args(["setup", "zsh", "--print"])
        .assert()
        .success()
        .stdout(predicates::str::contains("eval \"$(gitid hook zsh)\""));
}

#[test]
fn setup_yes_appends_marked_block_and_is_idempotent() {
    let env = TestEnv::new();
    let rc = env.home().join(".zshrc");
    std::fs::write(&rc, "# my zshrc\n").unwrap();

    env.gitid()
        .args(["setup", "zsh", "--yes"])
        .assert()
        .success();
    let after = std::fs::read_to_string(&rc).unwrap();
    assert!(after.contains("# my zshrc"));
    assert!(after.contains("# >>> gitid hook >>>"));
    assert_eq!(after.matches("gitid hook zsh").count(), 1);

    // Running again must not duplicate.
    env.gitid()
        .args(["setup", "zsh", "--yes"])
        .assert()
        .success();
    let after2 = std::fs::read_to_string(&rc).unwrap();
    assert_eq!(after2.matches("# >>> gitid hook >>>").count(), 1);
}

#[test]
fn completions_generate() {
    let env = TestEnv::new();
    for shell in ["bash", "zsh", "fish", "powershell", "nu"] {
        env.gitid()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicates::str::contains("gitid"));
    }
}

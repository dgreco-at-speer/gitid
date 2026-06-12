//! Tests for `gitid env` — the shell-hook hot path.

mod common;

use common::TestEnv;

/// Extract the `GITID_STATE='...'` value from bash activation output.
fn extract_state(bash_output: &str) -> String {
    let line = bash_output
        .lines()
        .find(|l| l.contains("GITID_STATE="))
        .expect("state line present");
    let start = line.find('\'').unwrap() + 1;
    let end = line.rfind('\'').unwrap();
    line[start..end].to_string()
}

fn setup_work(env: &TestEnv) -> std::path::PathBuf {
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

#[test]
fn activation_exports_profile_and_gh_dir() {
    let env = TestEnv::new();
    let tree = setup_work(&env);

    let out = env
        .gitid()
        .args([
            "env",
            "--shell",
            "bash",
            "--dir",
            tree.join("sub").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();

    assert!(out.contains("export GITID_PROFILE='work';"), "{out}");
    assert!(out.contains("GH_CONFIG_DIR"), "{out}");
    assert!(out.contains("gh/work"), "{out}");
    assert!(out.contains("export GITID_STATE="), "{out}");
}

#[test]
fn no_mapping_no_state_prints_nothing() {
    let env = TestEnv::new();
    setup_work(&env);
    env.gitid()
        .args([
            "env",
            "--shell",
            "bash",
            "--dir",
            "/tmp/definitely/not/mapped",
        ])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn leaving_mapped_dir_restores_and_unsets() {
    let env = TestEnv::new();
    let tree = setup_work(&env);

    // Activate, capture state.
    let activate = env
        .gitid()
        .args(["env", "--shell", "bash", "--dir", tree.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let state = extract_state(&String::from_utf8(activate).unwrap());

    // Now leave: same binary, GITID_STATE set, dir outside any mapping.
    let out = env
        .gitid()
        .env("GITID_STATE", &state)
        .args(["env", "--shell", "bash", "--dir", "/tmp/outside"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();

    assert!(out.contains("unset GITID_PROFILE;"), "{out}");
    assert!(out.contains("unset GH_CONFIG_DIR;"), "{out}");
    assert!(out.contains("unset GITID_STATE;"), "{out}");
}

#[test]
fn same_dir_with_matching_state_is_noop() {
    let env = TestEnv::new();
    let tree = setup_work(&env);
    let activate = env
        .gitid()
        .args(["env", "--shell", "bash", "--dir", tree.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let state = extract_state(&String::from_utf8(activate).unwrap());

    env.gitid()
        .env("GITID_STATE", &state)
        .args([
            "env",
            "--shell",
            "bash",
            "--dir",
            tree.join("sub").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn gitid_disable_silences_output() {
    let env = TestEnv::new();
    let tree = setup_work(&env);
    env.gitid()
        .env("GITID_DISABLE", "1")
        .args(["env", "--shell", "bash", "--dir", tree.to_str().unwrap()])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn fish_and_nu_dialects() {
    let env = TestEnv::new();
    let tree = setup_work(&env);

    env.gitid()
        .args(["env", "--shell", "fish", "--dir", tree.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("set -gx GITID_PROFILE 'work';"));

    let nu = env
        .gitid()
        .args(["env", "--shell", "nu", "--dir", tree.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&nu).unwrap();
    assert_eq!(v["set"]["GITID_PROFILE"], "work");
}

#[test]
fn corrupt_state_recovers_gracefully() {
    let env = TestEnv::new();
    let tree = setup_work(&env);
    // Garbage state must be treated as "no active profile": still activates.
    env.gitid()
        .env("GITID_STATE", "!!!garbage!!!")
        .args(["env", "--shell", "bash", "--dir", tree.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("export GITID_PROFILE='work';"));
}

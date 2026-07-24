//! Tests for the `gitid __complete` backend and the dynamic completion
//! scripts emitted by `gitid completions <shell>`.

mod common;

use common::{TestEnv, sample_profile};

#[test]
fn complete_profiles_lists_and_filters() {
    let env = TestEnv::new();
    env.add_profile("work", &sample_profile("Work", "work@corp.example"));
    env.add_profile("personal", &sample_profile("Me", "me@example.com"));

    let out = env
        .gitid()
        .args(["__complete", "profiles", ""])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    assert!(lines.contains(&"work"));
    assert!(lines.contains(&"personal"));

    env.gitid()
        .args(["__complete", "profiles", "wo"])
        .assert()
        .success()
        .stdout("work\n");

    env.gitid()
        .args(["__complete", "profiles", "zz"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn complete_is_silent_without_store() {
    let env = TestEnv::new();
    env.gitid()
        .args(["__complete", "profiles", ""])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn completions_script_has_marker() {
    let env = TestEnv::new();
    for shell in ["bash", "zsh", "fish"] {
        env.gitid()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicates::str::contains("GITID_COMPLETIONS_ACTIVE"))
            .stdout(predicates::str::contains("__complete profiles"));
    }
}

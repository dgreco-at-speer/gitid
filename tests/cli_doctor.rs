//! Tests for `gitid doctor` and the did-you-mean suggestions.

mod common;

use common::TestEnv;

#[test]
fn doctor_passes_on_a_healthy_setup() {
    let env = TestEnv::new();
    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--git-name",
            "Work",
            "--email",
            "work@corp.example",
            "--no-gh",
        ])
        .assert()
        .success();
    let tree = env.home().join("code");
    std::fs::create_dir_all(&tree).unwrap();
    env.gitid()
        .args(["use", "work", tree.to_str().unwrap()])
        .assert()
        .success();
    let repo = tree.join("repo");
    env.git_init(&repo);

    env.gitid()
        .args(["doctor", repo.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("all checks passed"))
        .stdout(predicates::str::contains("up to date"))
        .stdout(predicates::str::contains("SIGNING"))
        .stdout(predicates::str::contains("work"));
}

#[test]
fn doctor_detects_active_hook_and_completions() {
    let env = TestEnv::new();
    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--git-name",
            "Work",
            "--email",
            "work@corp.example",
            "--no-gh",
        ])
        .assert()
        .success();

    env.gitid()
        .arg("doctor")
        .env("GITID_HOOK_ACTIVE", "1")
        .env("GITID_COMPLETIONS_ACTIVE", "1")
        .assert()
        .success()
        .stdout(predicates::str::contains("shell hook active"))
        .stdout(predicates::str::contains("shell completions active"));
}

#[test]
fn doctor_reports_not_installed_when_absent() {
    let env = TestEnv::new();
    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--git-name",
            "Work",
            "--email",
            "work@corp.example",
            "--no-gh",
        ])
        .assert()
        .success();

    env.gitid()
        .arg("doctor")
        .env_remove("GITID_HOOK_ACTIVE")
        .env_remove("GITID_COMPLETIONS_ACTIVE")
        .assert()
        .success()
        .stdout(predicates::str::contains("shell hook not installed"))
        .stdout(predicates::str::contains("shell completions not installed"));
}

#[test]
fn doctor_flags_missing_sync() {
    let env = TestEnv::new();
    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--git-name",
            "Work",
            "--email",
            "work@corp.example",
            "--no-gh",
        ])
        .assert()
        .success();
    // Corrupt the generated include so doctor notices drift.
    std::fs::write(env.paths.include_gitconfig(), "# tampered\n").unwrap();
    env.gitid()
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("out of date"));
}

#[test]
fn unknown_profile_suggests_closest() {
    let env = TestEnv::new();
    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--git-name",
            "W",
            "--email",
            "w@x.example",
            "--no-gh",
        ])
        .assert()
        .success();
    let tree = env.home().join("code");
    std::fs::create_dir_all(&tree).unwrap();
    env.gitid()
        .args(["use", "wrok", tree.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("did you mean \"work\""));
}

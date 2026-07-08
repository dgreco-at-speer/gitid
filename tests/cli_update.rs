//! Tests for the opportunistic update notice and the `update` command surface.
//! These never touch the network: the notice is driven entirely by a pre-seeded
//! `update-check.json`, and the background refresh only fires on an interactive
//! stderr (never in the test harness).

mod common;

use common::TestEnv;
use predicates::prelude::PredicateBooleanExt;

/// Seed the update-check cache so the notice can be exercised without a network.
fn seed_cache(env: &TestEnv, latest: &str) {
    let path = env.paths.update_state_json();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // A recent last_check keeps the cache "fresh" so no refresh is attempted.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    std::fs::write(
        &path,
        format!(r#"{{"version":1,"last_check":{now},"latest_version":"{latest}"}}"#),
    )
    .unwrap();
}

#[test]
fn notice_shown_when_newer_version_cached() {
    let env = TestEnv::new();
    seed_cache(&env, "v9.9.9");
    env.gitid()
        .args(["list", "--format", "names"])
        .assert()
        .success()
        .stderr(predicates::str::contains("gitid 9.9.9 is available"));
}

#[test]
fn no_notice_when_cache_not_newer() {
    let env = TestEnv::new();
    seed_cache(&env, "v0.0.1");
    env.gitid()
        .args(["list", "--format", "names"])
        .assert()
        .success()
        .stderr(predicates::str::contains("is available").not());
}

#[test]
fn notice_suppressed_by_env() {
    let env = TestEnv::new();
    seed_cache(&env, "v9.9.9");
    env.gitid()
        .env("GITID_NO_UPDATE_CHECK", "1")
        .args(["list", "--format", "names"])
        .assert()
        .success()
        .stderr(predicates::str::contains("is available").not());
}

#[test]
fn shell_eval_commands_stay_silent() {
    let env = TestEnv::new();
    seed_cache(&env, "v9.9.9");
    // `hook` and `env` are consumed by the shell; they must emit no notice.
    env.gitid()
        .args(["hook", "bash"])
        .assert()
        .success()
        .stderr(predicates::str::contains("is available").not());
    env.gitid()
        .args(["env", "--shell", "bash"])
        .assert()
        .success()
        .stderr(predicates::str::contains("is available").not());
}

#[test]
fn stdout_is_not_polluted_by_notice() {
    let env = TestEnv::new();
    seed_cache(&env, "v9.9.9");
    // The notice goes to stderr; stdout for `list --format names` (no profiles)
    // must remain empty so pipelines are unaffected.
    env.gitid()
        .args(["list", "--format", "names"])
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}

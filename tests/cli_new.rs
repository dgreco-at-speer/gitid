//! `gitid new` — provisioning-from-scratch tests. Only the non-interactive path
//! is exercised (interactive keygen can't be driven from a test harness).

mod common;

use common::TestEnv;
use gitid::store::profiles;

#[test]
fn new_generates_ssh_key_and_writes_profile() {
    let env = TestEnv::new();
    let key = env.home().join(".ssh").join("id_ed25519_work");

    env.gitid()
        .args([
            "new",
            "work",
            "--non-interactive",
            "--git-name",
            "Jane Doe",
            "--email",
            "jane@corp.example",
            "--ssh-key",
        ])
        .arg(&key)
        .args(["--no-gh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("created profile \"work\""));

    // The keypair was actually generated.
    assert!(key.exists(), "private key was not generated");
    assert!(
        key.with_extension("pub").exists(),
        "public key was not generated"
    );

    // The profile is persisted and references the key.
    let parsed = profiles::load(&env.paths.profiles_toml()).unwrap();
    let profile = parsed.profiles.get("work").expect("profile written");
    assert_eq!(profile.name, "Jane Doe");
    assert_eq!(profile.email, "jane@corp.example");
    assert_eq!(
        profile.ssh.as_ref().map(|s| s.key.as_str()),
        Some(key.to_string_lossy().as_ref())
    );

    // The derived fragment wires up core.sshCommand.
    let fragment = std::fs::read_to_string(env.paths.fragment("work")).unwrap();
    assert!(
        fragment.contains("sshCommand = ssh -i"),
        "fragment missing sshCommand: {fragment}"
    );
}

#[test]
fn new_reuses_existing_key_without_clobbering() {
    let env = TestEnv::new();
    let ssh_dir = env.home().join(".ssh");
    std::fs::create_dir_all(&ssh_dir).unwrap();
    let key = ssh_dir.join("existing");
    std::fs::write(&key, "-----BEGIN OPENSSH PRIVATE KEY-----\nsentinel\n").unwrap();

    env.gitid()
        .args([
            "new",
            "reuse",
            "--non-interactive",
            "--git-name",
            "R",
            "--email",
            "r@e.example",
            "--ssh-key",
        ])
        .arg(&key)
        .args(["--no-gh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("reusing existing SSH key"));

    // The pre-existing key content is untouched.
    let body = std::fs::read_to_string(&key).unwrap();
    assert!(body.contains("sentinel"), "existing key was overwritten");
}

#[test]
fn new_without_ssh_creates_profile_with_no_key() {
    let env = TestEnv::new();
    env.gitid()
        .args([
            "new",
            "oss",
            "--non-interactive",
            "--git-name",
            "Jane",
            "--email",
            "jane@home.example",
            "--no-ssh",
            "--no-gh",
        ])
        .assert()
        .success();

    let parsed = profiles::load(&env.paths.profiles_toml()).unwrap();
    let profile = parsed.profiles.get("oss").expect("profile written");
    assert!(profile.ssh.is_none(), "expected no ssh block");
    assert!(profile.gh.is_none(), "expected no gh block");
}

#[test]
fn new_rejects_duplicate_profile() {
    let env = TestEnv::new();
    env.add_profile("work", &common::sample_profile("Existing", "e@e.example"));

    env.gitid()
        .args([
            "new",
            "work",
            "--non-interactive",
            "--git-name",
            "X",
            "--email",
            "x@e.example",
            "--no-ssh",
            "--no-gh",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));
}

#[test]
fn new_non_interactive_requires_git_name() {
    let env = TestEnv::new();
    env.gitid()
        .args([
            "new",
            "work",
            "--non-interactive",
            "--email",
            "x@e.example",
            "--no-ssh",
            "--no-gh",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--git-name is required"));
}

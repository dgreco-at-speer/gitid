//! End-to-end tests driving the real binary against an isolated HOME, verifying
//! that conditional includes actually make git resolve the right identity.

mod common;

use common::TestEnv;

/// The headline behaviour: assigning a profile to a directory makes git resolve
/// that identity inside repos in the tree — without touching the repo's config.
#[test]
fn use_makes_git_resolve_profile_identity() {
    let env = TestEnv::new();

    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--git-name",
            "Work User",
            "--email",
            "work@corp.example",
            "--no-gh",
        ])
        .assert()
        .success();

    let work_tree = env.home().join("code").join("work");
    std::fs::create_dir_all(&work_tree).unwrap();
    env.gitid()
        .args(["use", "work", work_tree.to_str().unwrap()])
        .assert()
        .success();

    // A repo inside the tree resolves the profile identity.
    let repo = work_tree.join("repo");
    env.git_init(&repo);
    assert_eq!(
        env.git_config_in(&repo, "user.email").as_deref(),
        Some("work@corp.example")
    );
    assert_eq!(
        env.git_config_in(&repo, "user.name").as_deref(),
        Some("Work User")
    );

    // A repo outside the tree does not.
    let other = env.home().join("elsewhere").join("repo");
    env.git_init(&other);
    assert_eq!(env.git_config_in(&other, "user.email"), None);
}

#[test]
fn nested_mappings_longest_prefix_wins() {
    let env = TestEnv::new();
    for (name, email) in [("personal", "me@home.example"), ("work", "me@corp.example")] {
        env.gitid()
            .args([
                "add",
                name,
                "--non-interactive",
                "--git-name",
                "Me",
                "--email",
                email,
                "--no-gh",
            ])
            .assert()
            .success();
    }

    let code = env.home().join("code");
    let work = code.join("work");
    std::fs::create_dir_all(&work).unwrap();
    env.gitid()
        .args(["use", "personal", code.to_str().unwrap()])
        .assert()
        .success();
    env.gitid()
        .args(["use", "work", work.to_str().unwrap()])
        .assert()
        .success();

    let work_repo = work.join("repo");
    env.git_init(&work_repo);
    assert_eq!(
        env.git_config_in(&work_repo, "user.email").as_deref(),
        Some("me@corp.example"),
        "nested longer mapping must win"
    );

    let personal_repo = code.join("oss").join("repo");
    env.git_init(&personal_repo);
    assert_eq!(
        env.git_config_in(&personal_repo, "user.email").as_deref(),
        Some("me@home.example")
    );
}

#[test]
fn ssh_command_is_set_in_resolved_config() {
    let env = TestEnv::new();
    let key = env.home().join(".ssh").join("id_work");
    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--git-name",
            "Work",
            "--email",
            "work@corp.example",
            "--ssh-key",
            key.to_str().unwrap(),
            "--no-gh",
        ])
        .assert()
        .success();
    let tree = env.home().join("w");
    std::fs::create_dir_all(&tree).unwrap();
    env.gitid()
        .args(["use", "work", tree.to_str().unwrap()])
        .assert()
        .success();
    let repo = tree.join("r");
    env.git_init(&repo);
    let cmd = env.git_config_in(&repo, "core.sshCommand").unwrap();
    assert!(cmd.contains("-i"), "got: {cmd}");
    assert!(cmd.contains("IdentitiesOnly=yes"), "got: {cmd}");
    assert!(cmd.contains(key.to_str().unwrap()), "got: {cmd}");
}

#[test]
fn bootstrap_is_idempotent_and_appends_once() {
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
    // First sync already happened via add; run two more.
    env.gitid().arg("sync").assert().success();
    env.gitid().arg("sync").assert().success();

    let global = env.read_global_gitconfig();
    let includes = global.matches("include.gitconfig").count();
    assert_eq!(includes, 1, "global config:\n{global}");
}

#[test]
fn bootstrap_preserves_existing_comments_and_user_block() {
    let env = TestEnv::new();
    env.write_global_gitconfig(
        "# my hand-written config\n[user]\n\temail = global@everywhere.example\n",
    );

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

    let global = env.read_global_gitconfig();
    assert!(global.contains("# my hand-written config"), "{global}");
    assert!(global.contains("global@everywhere.example"), "{global}");
    // Our include must be appended after the user block.
    let user_at = global.find("[user]").unwrap();
    let inc_at = global.find("[include]").unwrap();
    assert!(
        inc_at > user_at,
        "include must come after user block:\n{global}"
    );

    // And precedence: inside the mapped tree, the profile wins over global [user].
    let repo = tree.join("repo");
    env.git_init(&repo);
    assert_eq!(
        env.git_config_in(&repo, "user.email").as_deref(),
        Some("work@corp.example")
    );
}

#[test]
fn current_reports_active_profile() {
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
        .args(["use", "work", tree.to_str().unwrap()])
        .assert()
        .success();

    env.gitid()
        .args([
            "current",
            "--format",
            "name",
            tree.join("sub").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("work"));
}

#[test]
fn remove_refuses_when_mapped_without_force() {
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
        .args(["use", "work", tree.to_str().unwrap()])
        .assert()
        .success();

    env.gitid().args(["remove", "work"]).assert().failure();
    env.gitid()
        .args(["remove", "work", "--force"])
        .assert()
        .success();
    env.gitid()
        .args(["list", "--format", "names"])
        .assert()
        .success()
        .stdout("");
}

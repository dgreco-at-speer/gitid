//! SSH-agent-held profile keys: sync materialisation, derived config, doctor,
//! and (on unix, best-effort) an end-to-end run against a real ssh-agent.

mod common;

use common::TestEnv;
use gitid::store::profiles::{Profile, SIGNING_KEY_AGENT, Signing, SigningFormat, Ssh};
use predicates::prelude::*;

const PUB_LINE: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB5RCT+eqSGqnokTIWWpaBW4uUC1MqNCTBddYzZSN99a work@corp.example";

fn agent_profile() -> Profile {
    Profile {
        name: "Work".into(),
        email: "work@corp.example".into(),
        ssh: Some(Ssh::from_agent("work@corp.example")),
        signing: Some(Signing {
            format: SigningFormat::Ssh,
            key: SIGNING_KEY_AGENT.into(),
            commits: true,
            tags: None,
        }),
        gh: None,
        env: Default::default(),
        extra: Default::default(),
    }
}

#[test]
fn sync_without_agent_and_no_cached_key_fails() {
    let env = TestEnv::new();
    env.add_profile("work", &agent_profile());
    env.gitid()
        .arg("sync")
        .assert()
        .failure()
        .stderr(predicate::str::contains("could not resolve ssh-agent key"))
        .stderr(predicate::str::contains("SSH_AUTH_SOCK"));
}

#[test]
fn sync_without_agent_keeps_cached_key_and_warns() {
    let env = TestEnv::new();
    env.add_profile("work", &agent_profile());
    let pub_path = env.paths.ssh_pub("work");
    std::fs::create_dir_all(pub_path.parent().unwrap()).unwrap();
    std::fs::write(&pub_path, format!("{PUB_LINE}\n")).unwrap();

    env.gitid()
        .arg("sync")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "could not refresh the ssh-agent key",
        ));

    // The cached key survives and the derived config points at it.
    assert_eq!(
        std::fs::read_to_string(&pub_path).unwrap(),
        format!("{PUB_LINE}\n")
    );
    let expected = pub_path.to_string_lossy().replace('\\', "/");
    let fragment = std::fs::read_to_string(env.paths.fragment("work")).unwrap();
    assert!(
        fragment.contains(&format!(
            "sshCommand = ssh -i {expected} -o IdentitiesOnly=yes"
        )),
        "{fragment}"
    );
    assert!(
        fragment.contains(&format!("signingkey = {expected}")),
        "{fragment}"
    );

    // And git resolves it inside a mapped repo.
    let tree = env.home().join("w");
    std::fs::create_dir_all(&tree).unwrap();
    env.gitid()
        .args(["use", "work", tree.to_str().unwrap()])
        .assert()
        .success();
    let repo = tree.join("r");
    env.git_init(&repo);
    let cmd = env.git_config_in(&repo, "core.sshCommand").unwrap();
    assert!(cmd.contains(&expected), "got: {cmd}");
    assert_eq!(
        env.git_config_in(&repo, "user.signingkey").as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        env.git_config_in(&repo, "gpg.format").as_deref(),
        Some("ssh")
    );
}

#[test]
fn sync_prunes_pub_after_switching_to_path_key() {
    let env = TestEnv::new();
    env.add_profile("work", &agent_profile());
    let pub_path = env.paths.ssh_pub("work");
    std::fs::create_dir_all(pub_path.parent().unwrap()).unwrap();
    std::fs::write(&pub_path, format!("{PUB_LINE}\n")).unwrap();
    env.gitid().arg("sync").assert().success();

    let mut profile = agent_profile();
    profile.ssh = Some(Ssh::from_path("~/.ssh/id_work"));
    profile.signing = None;
    env.add_profile("work", &profile);
    env.gitid().arg("sync").assert().success();
    assert!(!pub_path.exists(), "stale pub file must be pruned");
}

#[test]
fn show_reports_agent_key() {
    let env = TestEnv::new();
    env.add_profile("work", &agent_profile());

    env.gitid()
        .args(["show", "work", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"agent\": \"work@corp.example\""));

    env.gitid()
        .args(["show", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent work@corp.example"));
}

#[test]
fn doctor_warns_when_agent_unreachable_but_key_cached() {
    let env = TestEnv::new();
    env.add_profile("work", &agent_profile());
    let pub_path = env.paths.ssh_pub("work");
    std::fs::create_dir_all(pub_path.parent().unwrap()).unwrap();
    std::fs::write(&pub_path, format!("{PUB_LINE}\n")).unwrap();
    env.gitid().arg("sync").assert().success();

    env.gitid()
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("ssh-agent not reachable"));
}

#[test]
fn doctor_fails_when_agent_unreachable_and_key_never_materialised() {
    let env = TestEnv::new();
    env.add_profile("work", &agent_profile());

    env.gitid()
        .arg("doctor")
        .assert()
        .failure()
        .stdout(predicate::str::contains("never materialised"));
}

#[test]
fn add_with_agent_key_fails_without_agent() {
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
            "--ssh-agent-key",
            "work@corp.example",
            "--no-gh",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SSH_AUTH_SOCK"));
}

#[test]
fn ssh_key_and_agent_key_flags_conflict() {
    let env = TestEnv::new();
    env.gitid()
        .args([
            "add",
            "work",
            "--non-interactive",
            "--ssh-key",
            "~/.ssh/id",
            "--ssh-agent-key",
            "x",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// ---------------------------------------------------------------------------
// Live-agent end-to-end (unix only, skips when openssh tools are missing).
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod live {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};

    fn have(bin: &str) -> bool {
        Command::new(bin)
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    /// Kills the spawned ssh-agent when the test ends (pass or fail).
    struct AgentGuard(Child);
    impl Drop for AgentGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn spawn_agent(sock: &Path) -> Option<AgentGuard> {
        let child = Command::new("ssh-agent")
            .args(["-D", "-a"])
            .arg(sock)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let guard = AgentGuard(child);
        for _ in 0..100 {
            if sock.exists() {
                return Some(guard);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        None
    }

    fn keygen(dir: &Path, comment: &str) -> PathBuf {
        let key = dir.join("id_test");
        let status = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", comment, "-f"])
            .arg(&key)
            .status()
            .unwrap();
        assert!(status.success(), "ssh-keygen failed");
        key
    }

    #[test]
    fn live_agent_end_to_end() {
        if !have("ssh-agent") || !have("ssh-keygen") || !have("ssh-add") {
            eprintln!("skipping: openssh tools not available");
            return;
        }
        let env = TestEnv::new();
        let sock = env.tmp.path().join("agent.sock");
        let Some(_agent) = spawn_agent(&sock) else {
            eprintln!("skipping: could not start ssh-agent");
            return;
        };
        let key = keygen(env.tmp.path(), "work@corp.example");
        let added = Command::new("ssh-add")
            .arg(&key)
            .env("SSH_AUTH_SOCK", &sock)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(added.success(), "ssh-add failed");
        let pub_line = std::fs::read_to_string(key.with_extension("pub"))
            .unwrap()
            .trim_end()
            .to_string();

        // Add by comment selector; validation talks to the live agent.
        env.gitid()
            .args([
                "add",
                "work",
                "--non-interactive",
                "--git-name",
                "Work",
                "--email",
                "work@corp.example",
                "--ssh-agent-key",
                "work@corp.example",
                "--signing",
                "ssh",
                "--signing-key",
                SIGNING_KEY_AGENT,
                "--sign-commits",
                "--no-gh",
            ])
            .env("SSH_AUTH_SOCK", &sock)
            .assert()
            .success();

        // The public key was materialised from the agent.
        let materialised = std::fs::read_to_string(env.paths.ssh_pub("work")).unwrap();
        assert_eq!(materialised, format!("{pub_line}\n"));

        // Doctor is green (warnings at most) with the agent running.
        env.gitid()
            .arg("doctor")
            .env("SSH_AUTH_SOCK", &sock)
            .assert()
            .success()
            .stdout(predicate::str::contains("ssh-agent holds"));

        // git resolves the derived key for auth and signing.
        let tree = env.home().join("w");
        std::fs::create_dir_all(&tree).unwrap();
        env.gitid()
            .args(["use", "work", tree.to_str().unwrap()])
            .env("SSH_AUTH_SOCK", &sock)
            .assert()
            .success();
        let repo = tree.join("r");
        env.git_init(&repo);
        let expected = env.paths.ssh_pub("work").to_string_lossy().into_owned();
        let cmd = env.git_config_in(&repo, "core.sshCommand").unwrap();
        assert!(cmd.contains(&expected), "got: {cmd}");
        assert_eq!(
            env.git_config_in(&repo, "user.signingkey").as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(
            env.git_config_in(&repo, "commit.gpgsign").as_deref(),
            Some("true")
        );
    }
}

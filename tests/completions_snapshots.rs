//! Snapshot tests of the emitted completion scripts so any template change is
//! reviewed.

use assert_cmd::Command;

fn completions(shell: &str) -> String {
    let out = Command::cargo_bin("gitid")
        .unwrap()
        .args(["completions", shell])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn snapshot_bash() {
    insta::assert_snapshot!(completions("bash"));
}

#[test]
fn snapshot_zsh() {
    insta::assert_snapshot!(completions("zsh"));
}

#[test]
fn snapshot_fish() {
    insta::assert_snapshot!(completions("fish"));
}

#[test]
fn snapshot_powershell() {
    insta::assert_snapshot!(completions("powershell"));
}

#[test]
fn snapshot_nu() {
    insta::assert_snapshot!(completions("nu"));
}

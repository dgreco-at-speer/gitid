//! Snapshot tests of the emitted hook scripts so any template change is reviewed.

use assert_cmd::Command;

fn hook(shell: &str) -> String {
    let out = Command::cargo_bin("gitid")
        .unwrap()
        .args(["hook", shell])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn snapshot_bash() {
    insta::assert_snapshot!(hook("bash"));
}

#[test]
fn snapshot_zsh() {
    insta::assert_snapshot!(hook("zsh"));
}

#[test]
fn snapshot_fish() {
    insta::assert_snapshot!(hook("fish"));
}

#[test]
fn snapshot_powershell() {
    insta::assert_snapshot!(hook("powershell"));
}

#[test]
fn snapshot_nu() {
    insta::assert_snapshot!(hook("nu"));
}

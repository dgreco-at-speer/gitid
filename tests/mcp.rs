//! `gitid mcp` surface: harness registration (`install`) and the stdio server
//! (`serve`).

mod common;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use common::TestEnv;

#[test]
fn install_print_claude_code_shows_snippet() {
    let env = TestEnv::new();
    env.gitid()
        .args(["mcp", "install", "claude-code", "--print"])
        .assert()
        .success()
        .stdout(predicates::str::contains("mcpServers"))
        .stdout(predicates::str::contains("gitid"))
        .stdout(predicates::str::contains("serve"));
}

#[test]
fn install_print_opencode_uses_local_shape() {
    let env = TestEnv::new();
    env.gitid()
        .args(["mcp", "install", "opencode", "--print"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"mcp\""))
        .stdout(predicates::str::contains("\"type\": \"local\""))
        .stdout(predicates::str::contains("\"command\""));
}

#[test]
fn install_project_cursor_merges_and_is_idempotent() {
    let env = TestEnv::new();
    let work = env.tmp.path().join("work");
    let cfg = work.join(".cursor").join("mcp.json");
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg,
        r#"{"mcpServers":{"other":{"command":"other-server"}}}"#,
    )
    .unwrap();

    env.gitid()
        .current_dir(&work)
        .args(["mcp", "install", "cursor", "--project", "--yes"])
        .assert()
        .success();

    let after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
    // Pre-existing server preserved; gitid added alongside.
    assert_eq!(after["mcpServers"]["other"]["command"], "other-server");
    assert_eq!(
        after["mcpServers"]["gitid"]["args"],
        serde_json::json!(["mcp", "serve"])
    );

    // Second run is a no-op.
    env.gitid()
        .current_dir(&work)
        .args(["mcp", "install", "cursor", "--project", "--yes"])
        .assert()
        .success();
    let after2: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
    assert_eq!(after, after2);
}

/// Drive the stdio server through initialize + tools/list. Responses are read
/// before stdin is closed, so the reply isn't cancelled by EOF.
#[test]
fn serve_lists_tools_over_stdio() {
    let env = TestEnv::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_gitid"))
        .args(["mcp", "serve"])
        .env("HOME", &env.paths.home)
        .env("GITID_CONFIG_DIR", &env.paths.config_dir)
        .env("GITID_DATA_DIR", &env.paths.data_dir)
        .env("GIT_CONFIG_GLOBAL", env.global_gitconfig())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    let mut send = |line: &str| {
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    };
    let mut read_line = || {
        let mut s = String::new();
        stdout.read_line(&mut s).unwrap();
        s
    };

    send(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
    );
    let init = read_line();
    assert!(init.contains("\"protocolVersion\""), "init reply: {init}");
    assert!(
        init.contains("gitid"),
        "serverInfo should name gitid: {init}"
    );

    send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    let tools = read_line();
    for name in ["gitid_list", "gitid_add", "gitid_use", "gitid_doctor"] {
        assert!(tools.contains(name), "tools/list missing {name}: {tools}");
    }

    drop(stdin);
    let _ = child.wait();
}

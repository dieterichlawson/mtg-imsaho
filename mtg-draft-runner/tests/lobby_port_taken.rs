//! A table that cannot take its port says so and nothing else (#745).
//!
//! An all-AI table started its draft before binding, so a port already in
//! use left stderr saying "the draft has started" and a log running to
//! "--- Pack 1 ---" ahead of the FATAL line: the record of a draft that
//! never ran (the shape of #735).
#![cfg(unix)]

mod lobby_support;

use std::net::TcpListener;
use std::process::{Command, Stdio};

#[test]
fn a_taken_port_is_the_only_thing_the_table_reports() {
    let (dir, bin) = lobby_support::scratch("port-taken");
    let held = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = held.local_addr().unwrap().port();
    let log = dir.join("draft.log");
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-server"))
        .args(["--seats", "ai,ai", "--port", &port.to_string(), "--best-of", "1", "--seed", "1"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(lobby_support::repo_root())
        .env("CLAUDE_CODE_BIN", &bin)
        .stdin(Stdio::null())
        .output()
        .expect("the server runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "a table that cannot listen fails:\n{stderr}");
    assert!(stderr.contains("cannot listen"), "and says why:\n{stderr}");
    for claim in ["the draft has started", "no human seats", "key="] {
        assert!(!stderr.contains(claim), "nothing is said of a table that never opened ({claim:?}):\n{stderr}");
    }
    let log = std::fs::read_to_string(&log).unwrap_or_default();
    // The packs dealt are the table's setup and are logged; a pass is not.
    assert!(!log.contains("--- Pack 1 ---"), "the log records no draft:\n{log}");
    assert!(log.contains("FATAL cannot listen"), "and ends saying why:\n{log}");
    drop(held);
}

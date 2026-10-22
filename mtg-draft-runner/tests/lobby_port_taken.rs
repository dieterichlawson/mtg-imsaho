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
    // Nor is the log written: the port is taken before it is opened, and
    // the log may be the running table's own (#893's review).
    let log = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(log.is_empty(), "the log is not written:\n{log}");
    drop(held);
}

/// #893: the same on --resume. The snapshot's replay was announced, and
/// the log opened for it, before the bind: resumed by mistake while the
/// old server still ran, it claimed "the table carries on" and wrote that
/// NOTE and a FATAL into the running table's own log.
#[test]
fn a_resume_on_a_taken_port_claims_nothing_and_leaves_the_log_alone() {
    let (dir, bin) = lobby_support::scratch("port-taken-resume");
    let held = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = held.local_addr().unwrap().port();
    let log = dir.join("draft.log");
    std::fs::write(&log, "the running table's log\n").unwrap();
    let seat = serde_json::json!({"kind": "ai:cc", "name": "seat", "key": null, "joined": true,
        "kicked": false, "gave_up": false, "notes": null});
    let save = dir.join("table.save.json");
    std::fs::write(&save, serde_json::json!({"version": 1, "set": "isd", "seed": 1, "best_of": 1,
        "seats": [seat, seat], "picks": [], "decks": [], "matches": []}).to_string()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-server"))
        .args(["--port", &port.to_string(), "--resume", save.to_str().unwrap()])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(lobby_support::repo_root())
        .env("CLAUDE_CODE_BIN", &bin)
        .stdin(Stdio::null())
        .output()
        .expect("the server runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && stderr.contains("cannot listen"), "{stderr}");
    assert!(!stderr.contains("carries on"), "nothing is said of a table that never opened:\n{stderr}");
    assert_eq!(std::fs::read_to_string(&log).unwrap(), "the running table's log\n",
        "the running table's log is not written to");
    drop(held);
}

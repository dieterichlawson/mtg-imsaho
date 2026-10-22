//! A snapshot that cannot be written is said, not promised (#891).
//!
//! Both write errors were dropped: with an unwritable `--save` the server
//! printed "carry on with --resume <path>" and never made the file, and a
//! write that failed halfway through a table (a full disk, a removed
//! directory) said nothing on the terminal or in the log.
#![cfg(unix)]

mod lobby_support;

use std::process::{Command, Stdio};
use std::time::Duration;

use lobby_support::Server;

#[test]
fn an_unwritable_save_is_refused_at_start_up() {
    let (dir, bin) = lobby_support::scratch("save-unwritable");
    let port = lobby_support::free_port().to_string();
    let save = dir.join("no").join("such").join("t.json");
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-server"))
        .args(["--seats", "ai,ai", "--best-of", "1", "--seed", "5", "--port", &port])
        .args(["--game-ports", "40100-40140"])
        .arg("--log").arg(dir.join("draft.log"))
        .arg("--save").arg(&save)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .stdin(Stdio::null())
        .output().expect("the server runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "refused: {stderr}");
    assert!(stderr.contains("--save: the snapshot cannot be written to"), "{stderr}");
    assert!(!stderr.contains("carry on with --resume"), "nothing promised: {stderr}");
}

#[test]
fn a_write_that_fails_later_is_said_and_so_is_its_recovery() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let (dir, _) = lobby_support::scratch("save-later");
    let saves = dir.join("saves");
    std::fs::create_dir_all(&saves).unwrap();
    let save = saves.join("t.json");
    let server = Server::start("save-later-table", "human,ai", &["--save", save.to_str().unwrap()]);
    assert!(save.exists(), "written at start-up");

    std::fs::remove_dir_all(&saves).unwrap();
    let _c0 = server.client(0); // joining is a change, and a write
    let terminal = server.wait_for_stderr("WARN the snapshot cannot be written to", Duration::from_secs(30));
    assert_eq!(terminal.matches("WARN the snapshot cannot be written").count(), 1, "said once: {terminal}");
    let log = server.wait_for_log("WARN the snapshot cannot be written to", Duration::from_secs(10));
    assert!(log.contains("--resume has nothing newer"), "{log}");

    std::fs::create_dir_all(&saves).unwrap();
    let _c0_again = server.client(0);
    server.wait_for_stderr("NOTE the snapshot is written again", Duration::from_secs(30));
    assert!(save.exists());
}

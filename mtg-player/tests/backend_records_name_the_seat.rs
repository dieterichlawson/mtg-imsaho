//! #659: a tournament match runs both seats' backends on one thread named
//! `seat A v B`, and the backend's own records — `API_RETRY`, `API_ERROR`
//! — carried only that thread name and a message with no seat in it. A
//! retry that succeeded could never be traced to the seat that made it.
//!
//! Its own binary because the log is process-global.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

#[test]
fn a_failed_call_is_logged_under_the_seat_that_made_it() {
    let dir = std::env::temp_dir().join(format!("mtg-api-seat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("claude");
    std::fs::write(
        &bin,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 9.9.9; exit 0; fi\ncat >/dev/null; echo boom >&2; exit 3\n",
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let log = dir.join("run.log");
    mtg_player::game_log::init(log.to_str().unwrap()).unwrap();

    let mut seat = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("Seat3", bin.to_str().unwrap());
    seat.backend_send_for_test("pick");

    let logged = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    let api: Vec<&str> = logged.lines().filter(|l| l.contains("API_")).collect();
    assert!(!api.is_empty(), "the failing call wrote no API record:\n{logged}");
    for line in &api {
        assert!(line.contains("API_ERROR [Seat3]"), "a backend record that does not name its seat: {line}");
    }
}

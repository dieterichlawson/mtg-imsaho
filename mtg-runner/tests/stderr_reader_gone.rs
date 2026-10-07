//! A stderr whose reader has gone is not a reason to panic (issue #685).
//!
//! The runtime ignores SIGPIPE, so `eprintln!` to a pipe whose reader has
//! exited panics. A `claude -p` seat's retry line was one: with stderr piped
//! through `head`, the first failed call killed the run with exit 101.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

#[test]
fn a_failing_seat_with_nobody_reading_stderr_plays_on() {
    let dir = std::env::temp_dir().join(format!("mtg-stderr-gone-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("claude");
    std::fs::write(&bin, "#!/bin/sh\n[ \"$1\" = \"--version\" ] && { echo stub 0.0; exit 0; }\ncat >/dev/null; echo boom >&2; exit 3\n").unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "cc", "--p2", "random", "--seed", "91020", "-q"])
        .args(["--log", dir.join("game.log").to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        // The seat gives up in a second and forfeits (#587), so the game ends.
        .env("MTG_GAME_RETRY_BUDGET_SECS", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the runner starts");
    // The reader goes away before anything is written.
    drop(child.stderr.take());
    let status = child.wait().expect("the runner ends");
    assert_ne!(status.code(), Some(101), "a write to a stderr nobody reads panicked the run");
    assert!(status.success(), "the run ends normally: {status}");
}

/// Nor is a stdout whose reader has gone (issue #717): the banner and the
/// summary were `println!`, so `mtg-runner ... | head` exited 101, and the
/// summary's panic came before the `--log` file's RESULT record.
#[test]
fn a_game_with_nobody_reading_stdout_ends_and_logs_its_result() {
    let dir = std::env::temp_dir().join(format!("mtg-stdout-gone-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("game.log");
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "random", "--p2", "random", "--seed", "3"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the runner starts");
    drop(child.stdout.take());
    let status = child.wait().expect("the runner ends");
    assert_ne!(status.code(), Some(101), "a write to a stdout nobody reads panicked the run");
    assert!(status.success(), "the run ends normally: {status}");
    let text = std::fs::read_to_string(&log).expect("the log was written");
    assert!(text.contains("RESULT"), "the log has its RESULT record:\n{}",
        text.lines().rev().take(5).collect::<Vec<_>>().join("\n"));
    let _ = std::fs::remove_dir_all(&dir);
}

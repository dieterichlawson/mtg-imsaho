//! However a run stops, it stops its `claude -p` subprocesses too.
//!
//! #206 made Ctrl-C and SIGTERM take a run's in-flight calls down with it,
//! by putting each child in its own process group and recording the group
//! in a fixed registry the signal handler walks. Two holes in that, both
//! found the same night:
//!
//! - The *fatal* exit path did not sweep the registry at all. `die` is
//!   `eprintln!` + `process::exit`, which runs no destructors and raises no
//!   signal, so a seat exhausting its retries left every other seat's whole
//!   `claude -p` process tree reparented to init and running on against a
//!   draft that no longer existed (issue #537). That is the ordinary case:
//!   all seats call in parallel and the joins are walked in seat order, so
//!   a fatal always fires while the others are in flight.
//!
//! It is checked here the way an operator would see it: real seats hanging
//! in real subprocesses, a real exit, and nothing left alive.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A `CLAUDE_CODE_BIN` seat that records its own pid and then hangs.
///
/// `STUB_FAIL_SEAT` names one seat that instead exits non-zero at once, so
/// the run reaches its fatal path with the others mid-call. The seat number
/// is read out of the pick prompt, which states it verbatim.
fn hanging_seat(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(
        &bin,
        r##"#!/usr/bin/env python3
import os, re, sys, time
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
message = sys.stdin.read()
m = re.search(r"You are seat (\d+) of", message)
seat = m.group(1) if m else "?"
if os.environ.get("STUB_FAIL_SEAT") == seat:
    sys.stderr.write("stub: seat %s deliberate failure\n" % seat)
    sys.exit(17)
with open(os.path.join(os.environ["STUB_PIDS"], str(os.getpid())), "w") as f:
    f.write(seat)
time.sleep(600)
"##,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn have_python() -> bool {
    std::process::Command::new("python3")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// The pids the stub has recorded so far.
fn recorded_pids(pids: &Path) -> Vec<i32> {
    let Ok(entries) = std::fs::read_dir(pids) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok()?.file_name().to_str()?.parse().ok())
        .collect()
}

/// Whether a process is still there. `kill -0` is the question without the
/// signal; a zombie would answer yes, but these are never our children.
fn alive(pid: i32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Wait for every recorded pid to be gone, up to `limit`. Returns the ones
/// still alive at the deadline.
fn survivors(pids: &[i32], limit: Duration) -> Vec<i32> {
    let deadline = Instant::now() + limit;
    loop {
        let left: Vec<i32> = pids.iter().copied().filter(|p| alive(*p)).collect();
        if left.is_empty() || Instant::now() > deadline {
            return left;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Leave nothing of ours running, whether the assertions passed or not.
fn reap(pids: &[i32]) {
    for pid in pids {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

#[test]
fn a_seats_fatal_takes_the_other_seats_calls_with_it() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }

    let dir = std::env::temp_dir().join(format!("mtg-draft-fatal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pids = dir.join("pids");
    std::fs::create_dir_all(&pids).unwrap();
    let bin = hanging_seat(&dir);

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "4", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", dir.join("run.log").to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .env("STUB_PIDS", &pids)
        .env("STUB_FAIL_SEAT", "0")
        // Long enough that the other seats are unmistakably mid-call when
        // seat 0 gives up, short enough that the test is quick.
        .env("MTG_DRAFT_RETRY_BUDGET_SECS", "5")
        .env("MTG_CLAUDE_CODE_TIMEOUT_SECS", "900")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the runner runs");

    assert!(!status.success(), "a seat that exhausts its retries is fatal: {status}");

    let hung = recorded_pids(&pids);
    assert!(
        !hung.is_empty(),
        "no seat ever reached a hanging call, so this run never had the thing \
         the fix is about — the fixture is broken, not the runner"
    );

    let left = survivors(&hung, Duration::from_secs(10));
    reap(&left);
    assert!(
        left.is_empty(),
        "{} of {} in-flight `claude -p` calls outlived the run's fatal exit \
         (pids {left:?}) — with a real seat each is a billed call still \
         running against a draft that has stopped",
        left.len(),
        hung.len()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

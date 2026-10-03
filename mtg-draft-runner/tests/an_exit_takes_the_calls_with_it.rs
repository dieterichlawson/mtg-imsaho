//! However a run stops, it stops its `claude -p` subprocesses too.
//!
//! #206 made Ctrl-C and SIGTERM (and, since #653, Ctrl-\) take a run's
//! in-flight calls down with it, by putting each child in its own process group and recording the group
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
//! - The registry held four groups and `--players` defaults to **8**, so
//!   half of an ordinary draft's calls ran outside the handler and survived
//!   the interrupt — whichever four lost the CAS, so not even a
//!   predictable set to clean up by hand (issue #538).
//!
//! Both are checked here the way an operator would see them: real seats
//! hanging in real subprocesses, a real exit, and nothing left alive.
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

#[test]
fn an_interrupt_takes_every_seats_call_with_it_at_the_default_player_count() {
    // 8 is `--players`' default, which is the whole point: the registry was
    // sized for four and the shipped configuration runs eight.
    signal_takes_every_call_with_it("TERM", 8);
}

/// #653: Ctrl-\ is a signal a terminal sends too, and the handler did not
/// cover it — the runner dumped core and every call ran on under init.
#[test]
fn a_quit_takes_every_seats_call_with_it() {
    signal_takes_every_call_with_it("QUIT", 4);
}

/// #654: SIGKILL (the OOM killer, a container stop) runs no handler at all,
/// and every call in flight ran on under init. The kernel's parent-death
/// signal takes the process the runner started; the stub is that process,
/// as the real CLI is.
#[cfg(target_os = "linux")]
#[test]
fn a_kill_takes_every_seats_call_with_it() {
    signal_takes_every_call_with_it("KILL", 4);
}

/// Start a run of `seats` hanging seats, wait until all are mid-call, send
/// the runner `signal`, and require that no call outlives it.
fn signal_takes_every_call_with_it(signal: &str, seats: usize) {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }

    let dir = std::env::temp_dir().join(format!("mtg-draft-sig{signal}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pids = dir.join("pids");
    std::fs::create_dir_all(&pids).unwrap();
    let bin = hanging_seat(&dir);

    // Through a shell only to turn core dumps off: SIGQUIT's default
    // action writes one, and a test should not leave it behind.
    let mut child = std::process::Command::new("sh")
        .args(["-c", "ulimit -c 0; exec \"$0\" \"$@\"", env!("CARGO_BIN_EXE_mtg-draft-runner")])
        .args(["--model", "cc", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--players", &seats.to_string()])
        .args(["--log", dir.join("run.log").to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .env("STUB_PIDS", &pids)
        .env("MTG_CLAUDE_CODE_TIMEOUT_SECS", "900")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the runner runs");

    // Every seat picks at once, so all of them should be in flight before
    // any of them returns. Wait for that rather than guessing at a sleep.
    let deadline = Instant::now() + Duration::from_secs(60);
    let hung = loop {
        let seen = recorded_pids(&pids);
        if seen.len() >= seats {
            break seen;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            reap(&seen);
            panic!(
                "only {} of {seats} seats reached a call in 60s — the fixture never \
                 got the run into the state this is about",
                seen.len()
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let killed = std::process::Command::new("kill")
        .args([format!("-{signal}"), child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(killed.success(), "the runner could not be signalled");
    let _ = child.wait();

    let left = survivors(&hung, Duration::from_secs(10));
    reap(&left);
    assert!(
        left.is_empty(),
        "{} of {} in-flight `claude -p` calls survived SIG{signal} (pids {left:?}) — \
         the signal handler does not cover this signal, or its registry does not \
         cover a run of {seats} seats",
        left.len(),
        hung.len()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// #652: with stderr closed, a seat's failure wedged the run forever. The
/// runtime ignores SIGPIPE, so the failing seat's own `eprintln!` panicked,
/// `die`'s first `eprintln!` panicked again before it swept or exited, and
/// every other thread parked in `report_worker_failure` for good — 0% CPU,
/// no exit, and the other seats' calls left running.
#[test]
fn a_fatal_with_stderr_closed_still_exits_and_sweeps() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    use std::io::Read;

    let dir = std::env::temp_dir().join(format!("mtg-draft-epipe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pids = dir.join("pids");
    std::fs::create_dir_all(&pids).unwrap();
    let bin = hanging_seat(&dir);

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7"])
        .args(["--log", dir.join("run.log").to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .env("STUB_PIDS", &pids)
        .env("STUB_FAIL_SEAT", "0")
        .env("MTG_DRAFT_RETRY_BUDGET_SECS", "5")
        .env("MTG_CLAUDE_CODE_TIMEOUT_SECS", "900")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the runner runs");

    // Read until the pick loop has started, then close our end — what
    // `| head` does to an operator's pipe.
    let mut stderr = child.stderr.take().unwrap();
    let mut seen = Vec::new();
    let mut byte = [0u8; 1];
    while !String::from_utf8_lossy(&seen).contains("Pack 1 Pick 1") {
        match stderr.read(&mut byte) {
            Ok(1) => seen.push(byte[0]),
            _ => break,
        }
    }
    drop(stderr);

    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() > deadline {
            break None;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let hung = recorded_pids(&pids);
    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        reap(&hung);
        panic!("the runner was still running 60s after its stderr closed — wedged");
    }
    assert!(!status.unwrap().success(), "a seat's fatal is still a failed run");

    let left = survivors(&hung, Duration::from_secs(10));
    reap(&left);
    assert!(left.is_empty(), "{} in-flight calls outlived the run (pids {left:?})", left.len());
    let _ = std::fs::remove_dir_all(&dir);
}

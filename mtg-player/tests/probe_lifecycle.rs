//! The `claude -p` availability probe is a subprocess like any other.
//!
//! It is the first one a `cc` run spawns, and it used to go through none of
//! the lifecycle the calls get. `available()` was a bare `status()`:
//!
//! - **no deadline**, so a CLI that does not answer `--version` hung the run
//!   forever before the first prompt — with no `--log` open yet, nothing
//!   printed, and nothing recorded anywhere;
//! - **no `setpgid`** and **no `LiveGroup` slot**, so a signal left the probe
//!   and its descendants running and reparented to init. #206's contract —
//!   take in-flight subprocesses down with the run — did not cover it,
//!   because the handler only sweeps the registry the probe was not in.
//!
//! This is the one place in the two backends' lifecycles where the two
//! copies agreed and were both wrong (issue #584).
//!
//! Its own test binary: the probe reads `CLAUDE_CODE_BIN` from the
//! environment, and a test that sets it must not be sharing a process with
//! tests that do not want it set.
#![cfg(unix)]

use std::time::{Duration, Instant};

/// A `claude` whose `--version` never answers, with a descendant holding on
/// — the shape a wrapper script gives, and the reason killing the direct
/// child is not enough.
fn wedged_version_stub(dir: &std::path::Path) -> std::path::PathBuf {
    let bin = dir.join("claude");
    std::fs::write(
        &bin,
        concat!(
            "#!/bin/sh\n",
            "for a in \"$@\"; do\n",
            "  if [ \"$a\" = \"--version\" ]; then echo $$ > \"$MARK\"; sleep 600 & wait; exit 0; fi\n",
            "done\n",
            "exit 1\n",
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn pid_alive(pid: i32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

#[test]
fn a_wedged_version_probe_gives_up_instead_of_hanging_the_run() {
    let dir = std::env::temp_dir().join(format!("mtg-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = wedged_version_stub(&dir);
    let mark = dir.join("probe.pid");

    std::env::set_var("CLAUDE_CODE_BIN", &bin);
    std::env::set_var("MTG_CLAUDE_CODE_PROBE_TIMEOUT_SECS", "2");
    std::env::set_var("MARK", &mark);

    let started = Instant::now();
    let runnable = mtg_player::llm::claude_code_available();
    let elapsed = started.elapsed();

    // It answers, and it answers "no". Before this it never answered.
    assert!(!runnable, "a CLI that cannot answer --version is not runnable");
    assert!(
        elapsed < Duration::from_secs(20),
        "the probe took {elapsed:?} — it is supposed to give up after its deadline, \
         and with no deadline it never returns at all"
    );
    assert!(
        elapsed >= Duration::from_secs(1),
        "the probe returned in {elapsed:?}, before its 2s deadline — the stub is \
         probably not being run, so this test is not exercising the hang"
    );

    // And it takes the whole tree with it, not just the process it spawned.
    // The stub's own pid is recorded by the stub; `sleep 600` is its child.
    let probe_pid: i32 = std::fs::read_to_string(&mark)
        .expect("the stub should have run and recorded its pid")
        .trim()
        .parse()
        .expect("a pid");
    // Give the kill a moment to land.
    let deadline = Instant::now() + Duration::from_secs(5);
    while pid_alive(probe_pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !pid_alive(probe_pid),
        "the probe (pid {probe_pid}) outlived the run that spawned it"
    );

    let survivors = std::process::Command::new("sh")
        .arg("-c")
        .arg("ps -eo pid,ppid,args | grep 'sleep 600' | grep -v grep || true")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    assert!(
        !survivors.contains(&probe_pid.to_string()),
        "a descendant of the probe survived it:\n{survivors}"
    );

    std::env::remove_var("CLAUDE_CODE_BIN");
    std::env::remove_var("MTG_CLAUDE_CODE_PROBE_TIMEOUT_SECS");
    std::env::remove_var("MARK");
    let _ = std::fs::remove_dir_all(&dir);
}

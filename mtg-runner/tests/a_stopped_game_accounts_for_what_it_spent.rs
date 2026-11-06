//! #912's review: a game stopped by Ctrl-C, SIGTERM or SIGHUP took its
//! `claude -p` calls down and ended on the spot, printing nothing of what
//! the seats had spent; the token usage was printed only after game over.
//! It is printed now on a stop signal (and on a fatal), with the reason as
//! the log's last record and the exit status the signal gives.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

#[test]
fn an_interrupted_game_prints_what_its_seats_spent() {
    let have_python = std::process::Command::new("python3").arg("--version").output().is_ok_and(|o| o.status.success());
    if !have_python {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-runner-sigint-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("calls")).unwrap();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    // Three calls are answered by the prompt stub; the fourth hangs.
    let bin = dir.join("seat.py");
    std::fs::write(&bin, format!(r#"#!/usr/bin/env python3
import os, subprocess, sys, time, uuid
d = "{calls}"
if sys.argv[1:2] == ["--version"]:
    print("1.0.0 (stub)"); sys.exit(0)
message = sys.stdin.read()
n = len(os.listdir(d))
open(os.path.join(d, uuid.uuid4().hex), "w").write(str(os.getpid()))
if n >= 3:
    open(os.path.join("{dir}", "hung"), "w").write(str(os.getpid()))
    time.sleep(600)
r = subprocess.run(["{root}/scripts/llm-prompt-stub.py"] + sys.argv[1:], input=message, capture_output=True, text=True)
sys.stdout.write(r.stdout)
"#, calls = dir.join("calls").display(), dir = dir.display(), root = root)).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let log = dir.join("game.log");
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "cc", "--p2", "random", "--seed", "3", "--quiet", "--log", log.to_str().unwrap()])
        .current_dir(root)
        .env("CLAUDE_CODE_BIN", &bin)
        .env("MTG_CLAUDE_CODE_TIMEOUT_SECS", "900")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the runner runs");
    let deadline = Instant::now() + Duration::from_secs(60);
    while !dir.join("hung").exists() {
        assert!(Instant::now() < deadline, "the seat never reached its fourth call");
        std::thread::sleep(Duration::from_millis(50));
    }
    let killed = std::process::Command::new("kill").args(["-INT", &child.id().to_string()]).status().unwrap();
    assert!(killed.success());
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let hung: i32 = std::fs::read_to_string(dir.join("hung")).unwrap().trim().parse().unwrap();
    let logged = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out.status.code(), Some(130), "the status a SIGINT gives:\n{stderr}");
    assert!(stdout.contains("claude-code: 3 calls"), "what the seat spent:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stderr.contains("Stopped: stopped by Ctrl-C (SIGINT)"), "{stderr}");
    assert!(logged.contains("\tTOKEN_USAGE"), "the log has the account too");
    let last = logged.lines().filter(|l| l.starts_with("20")).last().unwrap_or_default();
    assert!(last.contains("NOTE stopped by Ctrl-C (SIGINT); the game is not finished"), "{last}");
    // And the hung call went with the run, as before.
    std::thread::sleep(Duration::from_millis(200));
    let alive = unsafe { libc::kill(hung, 0) } == 0;
    if alive {
        unsafe { libc::kill(hung, libc::SIGKILL) };
    }
    assert!(!alive, "the hung call outlived the interrupt");
}

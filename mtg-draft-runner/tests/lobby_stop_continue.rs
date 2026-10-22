//! A terminal seat survives Ctrl-Z and `fg` (#880).
//!
//! A process stopped and continued gets `EINTR` from a socket read that has
//! a timeout set, with no handler installed. The client, the host's `cli`
//! seat and the server's per-socket pump all took that for the end of the
//! connection: the client exited, the host's seat was gone for the rest of
//! the table and every other terminal seat was reset.
#![cfg(unix)]

mod lobby_support;

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use lobby_support::Server;

/// What Ctrl-Z, a moment, then `fg` does to `pid`.
fn stop_and_continue(pid: u32) {
    for sig in ["-STOP", "-CONT"] {
        let ok = Command::new("kill").args([sig, &pid.to_string()]).status().expect("kill runs").success();
        assert!(ok, "kill {sig} {pid}");
        std::thread::sleep(Duration::from_millis(300));
    }
}

#[test]
fn a_stopped_and_continued_terminal_seat_stays_at_the_table() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("stop-continue", "cli,human", &["--quiet", "--pick-seconds", "600"]);
    let url = server.ws_url(1, server.key(1));
    let mut client = Command::new(env!("CARGO_BIN_EXE_mtg-draft-client"))
        .arg(&url).args(["--name", "Pipe"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("the client starts");
    let out = Arc::new(Mutex::new(String::new()));
    for pipe in [Box::new(client.stdout.take().unwrap()) as Box<dyn std::io::Read + Send>,
                 Box::new(client.stderr.take().unwrap())] {
        let sink = Arc::clone(&out);
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                let mut s = sink.lock().unwrap();
                s.push_str(&line);
                s.push('\n');
            }
        });
    }
    let wait_client = |needle: &str| {
        let deadline = Instant::now() + Duration::from_secs(60);
        while !out.lock().unwrap().contains(needle) {
            assert!(Instant::now() < deadline, "the client never printed {needle:?}:\n{}", out.lock().unwrap());
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    server.wait_for_stdout("type a number to pick", Duration::from_secs(60));
    wait_client("type a number to pick");

    // Each process is stopped and continued while its reads are waiting.
    for _ in 0..3 {
        stop_and_continue(server.child.id());
        stop_and_continue(client.id());
    }

    assert!(client.try_wait().unwrap().is_none(), "the client is still running:\n{}", out.lock().unwrap());
    let both = format!("{}{}{}", out.lock().unwrap(), server.stdout(), server.stderr());
    assert!(!both.contains("the connection ended"), "no seat's connection ended:\n{both}");

    // And both seats still pick: the host's at the server's terminal, the
    // friend's in the client, and the packs pass.
    server.type_line("0");
    use std::io::Write;
    writeln!(client.stdin.as_mut().unwrap(), "0").unwrap();
    server.wait_for_stdout("Pack 1 pick 2", Duration::from_secs(30));
    wait_client("Pack 1 pick 2");
    let _ = client.kill();
}

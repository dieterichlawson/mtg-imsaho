//! The terminal client, driven through a pipe: it joins, prints the pack,
//! picks by number, builds with `add`/`lands`/`ready`, and prints the
//! game link when the match is paired.
#![cfg(unix)]

mod lobby_support;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use lobby_support::Server;

fn wait_for(buf: &Arc<Mutex<String>>, needle: &str, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        let text = buf.lock().unwrap().clone();
        if text.contains(needle) {
            return text;
        }
        assert!(Instant::now() < deadline, "the client never printed {needle:?}:\n{text}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn the_terminal_client_drafts_builds_and_names_the_game_link() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let server = Server::start("client", "human,ai", &[]);
    let url = server.ws_url(0, server.key(0));
    let mut child = Command::new(env!("CARGO_BIN_EXE_mtg-draft-client"))
        .arg(&url).args(["--name", "Pipe"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("the client starts");
    let out = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&out);
    let stdout = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let mut s = sink.lock().unwrap();
            s.push_str(&line);
            s.push('\n');
        }
    });
    let mut stdin = child.stdin.take().unwrap();

    let text = wait_for(&out, "type a number to pick", Duration::from_secs(30));
    assert!(text.contains("Pack 1 pick 1 of 14"), "{text}");
    assert!(text.contains("  0: "), "the pack is a numbered list:\n{text}");
    assert!(text.contains("seat 1 ai"), "{text}");

    // Type 0 until the draft is over: a 0 with no pack in front is
    // answered with a line, not applied to the next pack.
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let text = out.lock().unwrap().clone();
        if text.contains("your pool — add <n>") {
            break;
        }
        assert!(Instant::now() < deadline, "the draft never finished:\n{text}\n{}", server.stderr());
        writeln!(stdin, "0").unwrap();
        std::thread::sleep(Duration::from_millis(150));
    }
    let text = out.lock().unwrap().clone();
    assert!(text.contains("no pack in front of you") || text.contains("Pack 1 pick 2 of 14"), "{text}");
    assert!(text.contains("your pool (") , "the pool is listed while drafting:\n{text}");
    assert!(server.log_text().contains("[Seat 0] PICK Pack 3 Pick 14 |"), "every pick was the person's");

    // Build: 23 spells, 17 lands, ready.
    for i in 0..23 {
        writeln!(stdin, "add {i}").unwrap();
    }
    // A refusal of something typed is printed and the client stays: it
    // used to quit on any reason containing "key" or "seat", and
    // "'Turkey' is not a basic land" contains "key" (#851).
    writeln!(stdin, "lands turkey=1").unwrap();
    wait_for(&out, "refused: 'Turkey' is not a basic land", Duration::from_secs(30));
    writeln!(stdin, "lands turkey=0 island=9 swamp=8").unwrap();
    // The count is the client's own arithmetic; the verdict is the
    // server's and arrives a beat later, so that is what is waited for.
    let text = wait_for(&out, "legal — type ready when it is final", Duration::from_secs(30));
    assert!(text.contains("23 spells + 17 lands = 40 cards"), "{text}");
    writeln!(stdin, "drop 0").unwrap();
    let text = wait_for(&out, "not legal yet: Deck has 39 cards", Duration::from_secs(30));
    assert!(text.contains("22 spells + 17 lands = 39 cards"), "the refusal's reason is shown:\n{text}");
    writeln!(stdin, "add 0").unwrap();
    wait_for(&out, "23 spells + 17 lands = 40 cards\nlegal", Duration::from_secs(30));
    writeln!(stdin, "ready").unwrap();
    wait_for(&out, "your deck is final", Duration::from_secs(30));

    // Paired: the game link is printed.
    let text = wait_for(&out, "Round 1 vs seat 1: playing — open http://127.0.0.1:", Duration::from_secs(60));
    assert!(text.contains("== isd draft, seat 0: playing =="), "{text}");
    writeln!(stdin, "quit").unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "quit exits cleanly");
}

#[test]
fn the_client_refuses_a_bad_link_and_a_wrong_key() {
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-client")).arg("http://nope").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("is not a ws:// link"));
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-client")).arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage: mtg-draft-client"));

    if !lobby_support::python3_available() {
        return;
    }
    let server = Server::start("client-key", "human,ai", &[]);
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-client"))
        .arg(server.ws_url(0, "wrong")).stdin(Stdio::null()).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("refused: wrong key for seat 0"),
        "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn a_pick_typed_twice_is_not_spent_on_the_next_pack() {
    // #809: the client stamped a typed number with whatever pack was on
    // screen when it got round to the line, so a `0` typed twice picked
    // card 0 of the next pack 50 ms after it was printed.
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let server = Server::start("client-twice", "human,ai", &[]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_mtg-draft-client"))
        .arg(server.ws_url(0, server.key(0)))
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("the client starts");
    let out = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&out);
    let stdout = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let mut s = sink.lock().unwrap();
            s.push_str(&line);
            s.push('\n');
        }
    });
    let mut stdin = child.stdin.take().unwrap();
    wait_for(&out, "Pack 1 pick 1 of 14", Duration::from_secs(30));
    // The AI seat has passed its pack by now, so the next one is waiting.
    server.wait_for_log("[Seat 1] PICK Pack 1 Pick 1 |", Duration::from_secs(30));
    std::thread::sleep(Duration::from_millis(600));

    writeln!(stdin, "0").unwrap();
    // The second 0 the moment the next pack is printed, as a repeat would.
    let deadline = Instant::now() + Duration::from_secs(30);
    while !out.lock().unwrap().contains("Pack 1 pick 2 of 14") {
        assert!(Instant::now() < deadline, "the second pack never came:\n{}", out.lock().unwrap());
        std::thread::sleep(Duration::from_millis(5));
    }
    writeln!(stdin, "0").unwrap();
    let text = wait_for(&out, "0 was not picked", Duration::from_secs(10));
    assert!(text.contains("type it again to pick from this pack"), "{text}");
    std::thread::sleep(Duration::from_millis(500));
    let log = server.log_text();
    assert!(log.contains("[Seat 0] PICK Pack 1 Pick 1 |"), "{log}");
    assert!(!log.contains("[Seat 0] PICK Pack 1 Pick 2 |"), "the repeat picked from a pack nobody read:\n{log}");

    // Typed again once the pack has been up a moment, it picks.
    writeln!(stdin, "0").unwrap();
    server.wait_for_log("[Seat 0] PICK Pack 1 Pick 2 |", Duration::from_secs(10));
    writeln!(stdin, "quit").unwrap();
    assert!(child.wait().unwrap().success());
}

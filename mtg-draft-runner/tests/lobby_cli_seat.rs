//! The host's own seat, at the server's terminal (`--seats cli,...`).
//!
//! It was refused ("not implemented in this version"). The server now runs
//! the terminal client for it against its own port: the host's keyboard is
//! the seat's for every line that is not one of the host's commands, and
//! the seat's screen is the server's stdout. Its games are played on the
//! game page whose link it prints, as a person's are.
#![cfg(unix)]

mod lobby_support;

use std::time::{Duration, Instant};

use lobby_support::Server;

#[test]
fn the_host_drafts_and_builds_at_the_servers_own_terminal() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("cli-seat", "cli,ai", &["--quiet"]);
    let terminal = server.stderr();
    assert!(terminal.contains("seat 0 is yours, here"), "{terminal}");
    // The seat is at the table before the host's keyboard is read, so the
    // draft starts on its own; a name typed at once is the seat's.
    server.type_line("name Host");
    server.wait_for_stdout("type a number to pick", Duration::from_secs(60));

    // Picks are typed at the server's terminal, the host's commands still
    // the host's.
    // The name goes through the seat's own client, `status` straight to
    // the host: wait for the seat's screen to have it.
    server.wait_for_stdout("Host", Duration::from_secs(10));
    server.type_line("status");
    server.wait_for_stderr("seat 0 cli Host", Duration::from_secs(10));
    let deadline = Instant::now() + Duration::from_secs(120);
    while !server.stdout().contains("your pool — add <n>") {
        assert!(Instant::now() < deadline, "the draft never finished:\n{}", server.stderr());
        server.type_line("0");
        std::thread::sleep(Duration::from_millis(400));
    }
    let log = server.log_text();
    assert!(log.contains("[Seat 0] PICK Pack 3 Pick 14 |") && !log.contains("the seat is away"),
        "every pick was the host's:\n{}", server.stderr());

    for i in 0..23 {
        server.type_line(&format!("add {i}"));
    }
    server.type_line("lands island=9 swamp=8");
    server.wait_for_stdout("legal — type ready when it is final", Duration::from_secs(30));
    server.type_line("ready");
    let screen = server.wait_for_stdout("Round 1 vs seat 1: playing", Duration::from_secs(60));
    assert!(screen.contains(&format!("open http://127.0.0.1:{}/game/", server.port)), "{screen}");
    server.type_line("review");
    server.wait_for_stdout("your draft:\nPack 1\n  1.1 ", Duration::from_secs(10));
    server.type_line("quit");
    let status = server.child.wait().unwrap();
    assert!(status.success(), "quit ends the table: {status}");
    assert!(server.wait_for_stdout("the table is closed", Duration::from_secs(5)).contains("the host quit"));
}

/// #909: the host's own seat is at the keyboard the host kicks from, and a
/// kick is for good: `kick 0` on it asks for `yes` first.
#[test]
fn kicking_the_hosts_own_seat_asks_first() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("cli-kick", "cli,ai", &["--quiet"]);
    // #902: start-up says which model the AI seat is (a plan-quota cc one).
    let started = server.stderr();
    assert!(started.contains("seat 1  ai cc:") && !started.contains("metered"), "{started}");
    server.wait_for_stdout("type a number to pick", Duration::from_secs(60));
    server.type_line("kick 0");
    server.wait_for_stderr("seat 0 is yours, at this terminal: `kick 0 yes`", Duration::from_secs(10));
    server.type_line("status");
    let status = server.wait_for_stderr("seat 0 cli", Duration::from_secs(10));
    assert!(!status.contains("kicked"), "not kicked: {status}");
    server.type_line("kick 0 yes");
    std::thread::sleep(Duration::from_millis(500));
    server.type_line("status");
    server.wait_for_stderr("kicked", Duration::from_secs(10));
}

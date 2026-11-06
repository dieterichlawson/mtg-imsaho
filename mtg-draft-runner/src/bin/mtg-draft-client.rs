//! `mtg-draft-client`: a seat at a hosted draft, from the terminal.
//!
//! A plain line-oriented client, not a TUI: it prints the seat's view when
//! it changes and reads commands. It is also what the integration tests
//! drive through a pipe. See `docs/draft-with-friends.md`.

use std::io::BufRead;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use mtg_draft_runner::client::{self, Ended, USAGE};

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut url: Option<String> = None;
    let mut name: Option<String> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return;
            }
            "--name" => {
                i += 1;
                name = argv.get(i).cloned();
            }
            other if other.starts_with("ws://") || other.starts_with("wss://") => url = Some(other.to_string()),
            other => {
                eprintln!("Error: '{other}' is not a ws:// link (see --help)");
                std::process::exit(1);
            }
        }
        i += 1;
    }
    let Some(url) = url else {
        eprintln!("Error: the join link is needed\n{USAGE}");
        std::process::exit(1);
    };

    let (mut ws, _) = match tungstenite::connect(&url) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: cannot connect to {url}: {e}");
            std::process::exit(1);
        }
    };
    if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_ref() {
        let _ = s.set_read_timeout(Some(Duration::from_millis(100)));
    }
    if let Some(name) = name {
        let _ = ws.send(tungstenite::Message::Text(
            serde_json::json!({"type": "name", "name": name}).to_string().into()));
    }

    // Commands come in on a channel; the one thread reads the socket and
    // writes to it, the way the server's pump does.
    // Each line carries the moment it was read, so a pick can be held
    // against the pack that was on screen when it was typed.
    let (tx, rx) = mpsc::channel::<(Instant, String)>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if tx.send((Instant::now(), line)).is_err() {
                break;
            }
        }
        let _ = tx.send((Instant::now(), "\u{0}eof".to_string()));
    });

    let origin = client::lobby_origin(&url).unwrap_or_default();
    match client::run(&mut ws, &rx, &origin) {
        Ended::Over => {}
        Ended::Refused => std::process::exit(1),
    }
}

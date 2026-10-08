//! `mtg-draft-client`: a seat at a hosted draft, from the terminal.
//!
//! A plain line-oriented client, not a TUI: it prints the seat's view when
//! it changes and reads commands. It is also what the integration tests
//! drive through a pipe. See `docs/draft-with-friends.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;

const USAGE: &str = "\
mtg-draft-client — sit at a hosted draft from the terminal

Usage: mtg-draft-client ws://host:8800/ws?seat=N&key=K [--name <name>]

While drafting, type a card's number to pick it. While building: add <n>,
drop <n> (by the pool numbers shown), lands plains=7 island=10, show, ready.
Any time: name <name>, help, quit.";

fn out(line: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

/// What the client keeps between views: the deck it is building.
#[derive(Default)]
struct Build {
    /// Pool indices in the main deck.
    main: BTreeSet<usize>,
    lands: BTreeMap<String, u32>,
}

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
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
        let _ = tx.send("\u{0}eof".to_string());
    });

    let mut view: Option<Value> = None;
    let mut shown = String::new();
    let mut shown_table = String::new();
    let mut build = Build::default();
    let mut built_for_phase = String::new();
    loop {
        while let Ok(line) = rx.try_recv() {
            if line == "\u{0}eof" {
                // Nobody is typing: stay attached and keep printing.
                continue;
            }
            match command(&line, view.as_ref(), &mut build) {
                Command::Send(msgs) => {
                    for m in msgs {
                        if ws.send(tungstenite::Message::Text(m.to_string().into())).is_err() {
                            out("the server went away");
                            return;
                        }
                    }
                }
                Command::Print(text) => out(&text),
                Command::Quit => return,
                Command::Nothing => {}
            }
        }
        match ws.read() {
            Ok(tungstenite::Message::Text(text)) => {
                let Ok(msg) = serde_json::from_str::<Value>(&text) else { continue };
                match msg["type"].as_str() {
                    Some("view") => {
                        let phase = msg["phase"].as_str().unwrap_or("").to_string();
                        if phase != built_for_phase {
                            built_for_phase = phase;
                            if let Some(deck) = msg["deck"].as_object() {
                                build = build_from_view(&msg, deck);
                            }
                        }
                        // Reprinted when something of this seat's own
                        // changed, not on every other seat's pick; the
                        // table's state is one line when only that moved.
                        let (table, mine) = render(&msg, &build);
                        if mine != shown {
                            out(&format!("{table}{mine}"));
                            shown = mine;
                        } else if table != shown_table {
                            out(&table_line(&msg));
                        }
                        shown_table = table;
                        view = Some(msg);
                    }
                    Some("refused") => {
                        out(&format!("refused: {}", msg["reason"].as_str().unwrap_or("?")));
                        if msg["reason"].as_str().is_some_and(|r| r.contains("key") || r.contains("seat")) {
                            std::process::exit(1);
                        }
                    }
                    _ => {}
                }
            }
            Ok(tungstenite::Message::Close(_)) => {
                out("the server closed the connection");
                return;
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => {
                out(&format!("the connection ended: {e}"));
                return;
            }
        }
    }
}

enum Command {
    Send(Vec<Value>),
    Print(String),
    Quit,
    Nothing,
}

/// The deck the server holds, as pool indices, when a view arrives in a
/// new phase (a reconnect mid-build picks the deck back up).
fn build_from_view(view: &Value, deck: &serde_json::Map<String, Value>) -> Build {
    let pool: Vec<&str> = view["pool"].as_array().map(|p| p.iter()
        .filter_map(|c| c["name"].as_str()).collect()).unwrap_or_default();
    let mut build = Build::default();
    let mut remaining: Vec<Option<&str>> = pool.iter().map(|n| Some(*n)).collect();
    for name in deck["main"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        if let Some(i) = remaining.iter().position(|n| *n == Some(name)) {
            remaining[i] = None;
            build.main.insert(i);
        }
    }
    if let Some(lands) = deck["lands"].as_object() {
        for (land, n) in lands {
            if let Some(n) = n.as_u64() {
                build.lands.insert(land.clone(), u32::try_from(n).unwrap_or(u32::MAX));
            }
        }
    }
    build
}

fn deck_message(view: &Value, build: &Build) -> Value {
    let pool: Vec<&str> = view["pool"].as_array().map(|p| p.iter()
        .filter_map(|c| c["name"].as_str()).collect()).unwrap_or_default();
    let main: Vec<&str> = build.main.iter().filter_map(|&i| pool.get(i).copied()).collect();
    let side: Vec<&str> = pool.iter().enumerate()
        .filter(|(i, _)| !build.main.contains(i)).map(|(_, n)| *n).collect();
    serde_json::json!({"type": "deck", "main": main, "lands": build.lands, "sideboard": side})
}

fn command(line: &str, view: Option<&Value>, build: &mut Build) -> Command {
    let words: Vec<&str> = line.split_whitespace().collect();
    let Some(first) = words.first() else { return Command::Nothing };
    let phase = view.map(|v| v["phase"].as_str().unwrap_or("")).unwrap_or("");
    match *first {
        "help" | "?" => Command::Print(USAGE.to_string()),
        "quit" | "q" | "exit" => Command::Quit,
        "name" => Command::Send(vec![serde_json::json!({"type": "name", "name": words[1..].join(" ")})]),
        "show" => view.map_or(Command::Nothing, |v| Command::Print({ let (t, m) = render(v, build); format!("{t}{m}") })),
        "ready" => {
            let Some(v) = view else { return Command::Print("no view yet".into()) };
            Command::Send(vec![deck_message(v, build), serde_json::json!({"type": "ready"})])
        }
        "add" | "drop" => {
            let Some(v) = view else { return Command::Print("no view yet".into()) };
            let pool_len = v["pool"].as_array().map_or(0, Vec::len);
            let Some(n) = words.get(1).and_then(|w| w.parse::<usize>().ok()).filter(|n| *n < pool_len) else {
                return Command::Print(format!("{first} takes a pool number (0-{})", pool_len.saturating_sub(1)));
            };
            if *first == "add" { build.main.insert(n); } else { build.main.remove(&n); }
            Command::Send(vec![deck_message(v, build)])
        }
        "lands" => {
            let Some(v) = view else { return Command::Print("no view yet".into()) };
            for word in &words[1..] {
                let Some((land, n)) = word.split_once('=') else {
                    return Command::Print("lands takes plains=7 island=10 ...".into());
                };
                let Ok(n) = n.parse::<u32>() else { return Command::Print(format!("'{n}' is not a count")) };
                let mut land = land.to_lowercase();
                if let Some(c) = land.get_mut(0..1) { c.make_ascii_uppercase(); }
                if n == 0 { build.lands.remove(&land); } else { build.lands.insert(land, n); }
            }
            Command::Send(vec![deck_message(v, build)])
        }
        word => match (word.parse::<usize>(), view) {
            (Ok(index), Some(v)) if phase == "drafting" => match v["pack"]["id"].as_u64() {
                Some(id) => Command::Send(vec![serde_json::json!({"type": "pick", "pack_id": id, "index": index})]),
                None => Command::Print("no pack in front of you right now".into()),
            },
            (Ok(_), _) => Command::Print(format!("a number picks a card while drafting (phase: {phase})")),
            _ => Command::Print(format!("'{word}'? type help")),
        },
    }
}

/// Everybody's state on one line: `seat 1 (seat 1) waiting 4 picks; ...`.
fn table_line(v: &Value) -> String {
    let parts: Vec<String> = v["seats"].as_array().into_iter().flatten().map(|x| {
        let joined = if x["joined"].as_bool().unwrap_or(false) { "" } else { ", not here yet" };
        let auto = if x["auto"].as_bool().unwrap_or(false) { ", table picks" } else { "" };
        format!("seat {} {} {}p{joined}{auto}", x["seat"], x["status"].as_str().unwrap_or("?"), x["picks"])
    }).collect();
    format!("[{}]", parts.join("; "))
}

/// The view as the terminal shows it: the table (everybody's state) and
/// this seat's own part.
fn render(v: &Value, build: &Build) -> (String, String) {
    let mut table = String::new();
    let mut s = String::new();
    let seat = v["seat"].as_u64().unwrap_or(0);
    let phase = v["phase"].as_str().unwrap_or("?");
    let names: Vec<String> = v["seats"].as_array().map(|a| a.iter()
        .map(|x| x["name"].as_str().unwrap_or("?").to_string()).collect()).unwrap_or_default();
    table.push_str(&format!("== {} draft, seat {seat}: {phase} ==\n", v["set"].as_str().unwrap_or("?")));
    for x in v["seats"].as_array().into_iter().flatten() {
        let joined = if x["joined"].as_bool().unwrap_or(false) { "" } else { " (not here yet)" };
        let auto = if x["auto"].as_bool().unwrap_or(false) { " [table picks]" } else { "" };
        table.push_str(&format!("  seat {} {:<6} {:<12} {} {} picks{joined}{auto}\n",
            x["seat"], x["kind"].as_str().unwrap_or("?"), x["name"].as_str().unwrap_or("?"),
            x["status"].as_str().unwrap_or("?"), x["picks"]));
    }
    if let Some(notice) = v["notice"].as_str() {
        s.push_str(&format!("!! {notice}\n"));
    }
    match phase {
        "lobby" => s.push_str("waiting for everybody to join\n"),
        "drafting" => {
            if let Some(pack) = v["pack"].as_object() {
                let waiting = pack["waiting"].as_u64().unwrap_or(0);
                let dir = v["pass_direction"].as_str().unwrap_or("");
                s.push_str(&format!("\nPack {} pick {} of {}, {waiting} waiting, passing {dir}",
                    pack["round"], pack["pick"], pack["size"]));
                s.push('\n');
                if let Some(ms) = pack["deadline_ms"].as_u64() {
                    table.push_str(&format!("  {}s left to pick\n", ms / 1000));
                }
                for c in pack["cards"].as_array().into_iter().flatten() {
                    s.push_str(&format!("  {:>2}: {}\n", c["index"], c["line"].as_str().unwrap_or("?")));
                }
                s.push_str("type a number to pick\n");
            } else {
                s.push_str("\nno pack in front of you; waiting on the seat upstream\n");
            }
            s.push_str(&pool_listing(v));
        }
        "building" => {
            s.push_str(&build_listing(v, build));
        }
        "playing" | "done" => {
            for m in v["matches"].as_array().into_iter().flatten() {
                let opp = m["opponent"].as_u64().unwrap_or(0) as usize;
                let name = names.get(opp).cloned().unwrap_or_else(|| format!("seat {opp}"));
                let status = m["status"].as_str().unwrap_or("?");
                s.push_str(&format!("Round {} vs {name}: {status}", m["round"]));
                // The link is for a match still to be played; a finished
                // one is its result (the page stays readable, but it is
                // not something to open).
                if let (Some(url), false) = (m["url"].as_str(), status == "done") {
                    s.push_str(&format!(" — open {url}"));
                }
                if let Some(r) = m["result"].as_str() {
                    s.push_str(&format!(" — {r}"));
                }
                s.push('\n');
            }
            for p in v["pairings"].as_array().into_iter().flatten() {
                let b = p["b"].as_u64().map_or("bye".to_string(), |b| format!("seat {b}"));
                s.push_str(&format!("  round {}: seat {} vs {b} ({}{})\n", p["round"], p["a"],
                    p["status"].as_str().unwrap_or("?"),
                    p["result"].as_str().map(|r| format!(", {r}")).unwrap_or_default()));
            }
            if let Some(st) = v["standings"].as_array() {
                s.push_str("Standings:\n");
                for (rank, x) in st.iter().enumerate() {
                    s.push_str(&format!("  {}. seat {} {}-{} ({} points)\n", rank + 1, x["seat"],
                        x["wins"], x["losses"], x["points"]));
                }
            }
            if phase == "done" {
                s.push_str("the tournament is over\n");
            }
        }
        _ => {}
    }
    (table, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finished_match_is_its_result_and_a_live_one_is_its_link() {
        let v = serde_json::json!({
            "type": "view", "phase": "playing", "seat": 0, "set": "isd",
            "seats": [{"seat": 0, "name": "seat 0", "kind": "human", "status": "playing", "picks": 42, "joined": true},
                      {"seat": 1, "name": "Lawson", "kind": "human", "status": "playing", "picks": 42, "joined": true}],
            "matches": [
                {"round": 1, "opponent": 1, "url": "http://h:8801/", "status": "done", "games": [], "result": "0-2"},
                {"round": 2, "opponent": 1, "url": "http://h:8803/", "status": "playing", "games": [], "result": null},
                {"round": 3, "opponent": 1, "url": null, "status": "waiting", "games": [], "result": null},
            ],
            "pairings": [], "standings": [], "pool": [], "picks": [], "deck": null, "notice": null,
        });
        let (_, mine) = render(&v, &Build::default());
        assert!(mine.contains("Round 1 vs Lawson: done — 0-2\n"), "{mine}");
        assert!(!mine.contains("8801"), "a finished match's page is not offered:\n{mine}");
        assert!(mine.contains("Round 2 vs Lawson: playing — open http://h:8803/\n"), "{mine}");
        assert!(mine.contains("Round 3 vs Lawson: waiting\n"), "{mine}");
    }
}

fn pool_listing(v: &Value) -> String {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for c in v["pool"].as_array().into_iter().flatten() {
        let line = c["line"].as_str().unwrap_or("?").to_string();
        match counts.iter_mut().find(|(l, _)| *l == line) {
            Some((_, n)) => *n += 1,
            None => counts.push((line, 1)),
        }
    }
    if counts.is_empty() {
        return "your pool is empty\n".to_string();
    }
    counts.sort();
    let mut s = format!("your pool ({} cards):\n", v["pool"].as_array().map_or(0, Vec::len));
    for (line, n) in counts {
        s.push_str(&format!("  {n}x {line}\n"));
    }
    s
}

fn build_listing(v: &Value, build: &Build) -> String {
    let mut s = String::from("\nyour pool — add <n> / drop <n> to move a card, lands plains=7 island=10, ready:\n");
    for (i, c) in v["pool"].as_array().into_iter().flatten().enumerate() {
        let tag = if build.main.contains(&i) { "MAIN" } else { "side" };
        s.push_str(&format!("  {i:>2} [{tag}] {}\n", c["line"].as_str().unwrap_or("?")));
    }
    let lands: u32 = build.lands.values().sum();
    let spells = build.main.len();
    s.push_str(&format!("lands: {}\n", build.lands.iter().map(|(l, n)| format!("{l}={n}"))
        .collect::<Vec<_>>().join(" ")));
    s.push_str(&format!("{spells} spells + {lands} lands = {} cards\n", spells as u32 + lands));
    if let Some(deck) = v["deck"].as_object() {
        if let Some(problem) = deck["problem"].as_str() {
            s.push_str(&format!("not legal yet: {problem}\n"));
        } else if deck["ready"].as_bool().unwrap_or(false) {
            s.push_str("your deck is final; waiting for the others\n");
        } else if deck["valid"].as_bool().unwrap_or(false) {
            s.push_str("legal — type ready when it is final\n");
        }
    }
    s
}

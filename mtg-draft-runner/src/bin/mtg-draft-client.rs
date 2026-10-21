//! `mtg-draft-client`: a seat at a hosted draft, from the terminal.
//!
//! A plain line-oriented client, not a TUI: it prints the seat's view when
//! it changes and reads commands. It is also what the integration tests
//! drive through a pipe. See `docs/draft-with-friends.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;

const USAGE: &str = "\
mtg-draft-client — sit at a hosted draft from the terminal

Usage: mtg-draft-client ws://host:8800/ws?seat=N&key=K [--name <name>]

While drafting, type a card's number to pick it (a number typed before a
new pack has been up 300 ms is not taken: type it again). While building:
add <n>, drop <n> (by the pool numbers shown), lands plains=7 island=10,
show, ready.
Between games of a match: play or draw (when the choice is yours), the
same add, drop and lands to sideboard (from game 2), and ready to start
the game.
text <n> prints a card's rules text: the pack's card <n> while drafting,
the pool's card <n> while building or sideboarding.
review prints every pack you were handed and what you took from it.
Any time: name <name>, help, quit.";

fn out(line: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

/// A pick typed this soon after its pack was printed is not taken: it was
/// typed for the pack before (a number typed twice, a repeated Enter), and
/// the pack it would land on has not been read (#809). The page ignores a
/// repeat for its first 300 ms on screen and the game's terminal drains
/// keys typed ahead of its prompt (#806, #71); this is the same guard.
const PICK_GRACE: Duration = Duration::from_millis(300);

/// What the client keeps between views: the deck it is building.
#[derive(Default)]
struct Build {
    /// Pool indices in the main deck.
    main: BTreeSet<usize>,
    lands: BTreeMap<String, u32>,
    /// Deck messages sent and not yet seen recorded, oldest first. A view
    /// built before the server read them carries an older record, which
    /// must not undo them (#815's review).
    in_flight: Vec<Value>,
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

    let mut view: Option<Value> = None;
    let mut shown = String::new();
    let mut shown_table = String::new();
    let mut build = Build::default();
    let mut built_for_phase = String::new();
    // The pack on screen (id and pick) and when it was printed.
    let mut pack_shown: Option<(Value, Instant)> = None;
    loop {
        while let Ok((typed_at, line)) = rx.try_recv() {
            if line == "\u{0}eof" {
                // Nobody is typing: stay attached and keep printing.
                continue;
            }
            let early = pack_shown.as_ref()
                .and_then(|(_, at)| (*at + PICK_GRACE).checked_duration_since(typed_at));
            match command(&line, view.as_ref(), &mut build, early) {
                Command::Send(msgs) => {
                    for m in msgs {
                        if m["type"] == "deck" {
                            build.in_flight.push(m.clone());
                        }
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
                        refresh_build(&msg, &mut build, &mut built_for_phase);
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
                        let pack = serde_json::json!([msg["pack"]["id"], msg["pack"]["pick"]]);
                        if !msg["pack"].is_object() {
                            pack_shown = None;
                        } else if pack_shown.as_ref().is_none_or(|(p, _)| *p != pack) {
                            pack_shown = Some((pack, Instant::now()));
                        }
                        view = Some(msg);
                    }
                    Some("review") => out(&review_listing(&msg)),
                    // The host ended the table: said, and nothing to wait for
                    // (#824). The close frame follows.
                    Some("closed") => {
                        out(&format!("the table is closed: {}", msg["reason"].as_str().unwrap_or("?")));
                        return;
                    }
                    Some("refused") => {
                        out(&format!("refused: {}", msg["reason"].as_str().unwrap_or("?")));
                        // A refusal with nothing echoed, before any view, is
                        // the join itself (a wrong key, no such seat), as the
                        // page reads it; any other is an answer to something
                        // typed, whatever its text says (#851).
                        if msg["echo"].is_null() && view.is_none() {
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

/// Bring the client's deck back to the server's record when a view arrives
/// in a new phase (a reconnect mid-build picks the deck back up) or with a
/// recorded deck that is not the one on screen. The server's record is the
/// deck that is played: when the build clock ran out and the table built
/// the deck, or topped a short one up, the client kept printing the
/// person's own MAIN marks and lands over "your deck is final" (#815).
///
/// A view built before the server read the client's latest changes (every
/// seat is sent a view on every change at the table) carries an older
/// record, and taking it would undo those changes and send the undone deck
/// with the next command — with `ready`, for good. So while deck messages
/// are in flight a record that is none of them is not taken, unless it is
/// final: a deck the table finished is the one played whatever was sent.
fn refresh_build(view: &Value, build: &mut Build, built_for_phase: &mut String) {
    // Between games the deck starts from the one played last game, once
    // per game: the server takes a whole deck or refuses it, so its record
    // is the last legal deck sent, never a command behind.
    if let Some((m, b)) = between(view) {
        let key = format!("between {} {}", m["round"], b["game"]);
        if key != *built_for_phase {
            if let Some(deck) = b["deck"].as_object() {
                *build = build_from_view(view, deck);
            }
            *built_for_phase = key;
        }
        return;
    }
    let phase = view["phase"].as_str().unwrap_or("");
    let new_phase = phase != built_for_phase.as_str();
    *built_for_phase = phase.to_string();
    let Some(deck) = view["deck"].as_object() else {
        if new_phase { build.in_flight.clear(); }
        return;
    };
    // The record has caught up with the oldest message sent. Only the
    // oldest: the server answers every message it reads with a view, in
    // order, and a refused deck is recorded too, so each one is seen; a
    // stale record equal to a later message (an add, then its undo) must
    // not clear the ones before it.
    if build.in_flight.first().is_some_and(|m| same_deck(deck, m)) {
        build.in_flight.remove(0);
    }
    let final_deck = deck["ready"].as_bool().unwrap_or(false);
    let differs = !same_deck(deck, &deck_message(view, build));
    if new_phase || (differs && (final_deck || build.in_flight.is_empty())) {
        let in_flight = if new_phase || final_deck { Vec::new() } else { std::mem::take(&mut build.in_flight) };
        *build = build_from_view(view, deck);
        build.in_flight = in_flight;
    }
}

/// The same main deck (as names, in any order) and the same nonzero lands.
fn same_deck(recorded: &serde_json::Map<String, Value>, sent: &Value) -> bool {
    let names = |v: &Value| {
        let mut n: Vec<String> = v.as_array().into_iter().flatten()
            .filter_map(Value::as_str).map(str::to_string).collect();
        n.sort();
        n
    };
    let lands = |v: &Value| -> BTreeMap<String, u64> {
        v.as_object().into_iter().flatten()
            .filter_map(|(k, n)| n.as_u64().filter(|n| *n > 0).map(|n| (k.clone(), n)))
            .collect()
    };
    names(&recorded["main"]) == names(&sent["main"]) && lands(&recorded["lands"]) == lands(&sent["lands"])
}

/// The deck the server holds, as pool indices.
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

/// The match of this seat's that is between games, and its `between`.
fn between(v: &Value) -> Option<(&Value, &Value)> {
    v["matches"].as_array()?.iter().find(|m| m["between"].is_object()).map(|m| (m, &m["between"]))
}

/// The deck being edited, sent as the phase takes it: every change while
/// building (a short deck is kept as work in progress), and between games
/// only a deck of 40 or more, which is all the server takes there.
fn send_edit(v: &Value, build: &Build) -> Command {
    let Some((_, b)) = between(v) else { return Command::Send(vec![deck_message(v, build)]) };
    if b["sideboarding"] != true {
        return Command::Print(format!("no sideboarding before game {}", b["game"]));
    }
    let total = build.main.len() + build.lands.values().sum::<u32>() as usize;
    if total < 40 {
        return Command::Print(format!("{total} cards: the deck for the next game needs 40 or more; nothing sent yet"));
    }
    Command::Send(vec![sideboard_message(v, build)])
}

fn sideboard_message(v: &Value, build: &Build) -> Value {
    let deck = deck_message(v, build);
    serde_json::json!({"type": "sideboard", "main": deck["main"], "lands": deck["lands"]})
}

/// `early` is set when the line was typed before the pack on screen had
/// been up for `PICK_GRACE`: how much sooner than that.
fn command(line: &str, view: Option<&Value>, build: &mut Build, early: Option<Duration>) -> Command {
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
            if let Some((_, b)) = between(v) {
                let mut msgs = Vec::new();
                if b["sideboarding"] == true {
                    match send_edit(v, build) {
                        Command::Send(m) => msgs.extend(m),
                        other => return other,
                    }
                }
                msgs.push(serde_json::json!({"type": "sideboard_ready"}));
                return Command::Send(msgs);
            }
            Command::Send(vec![deck_message(v, build), serde_json::json!({"type": "ready"})])
        }
        "review" => Command::Send(vec![serde_json::json!({"type": "review"})]),
        "play" | "draw" => {
            let Some(v) = view else { return Command::Print("no view yet".into()) };
            match between(v) {
                Some((_, b)) if b["you_choose"] == true =>
                    Command::Send(vec![serde_json::json!({"type": "play_draw", "play": *first == "play"})]),
                Some(_) => Command::Print("the choice to play or draw is your opponent's".into()),
                None => Command::Print("play and draw choose who starts the next game of a match, between games".into()),
            }
        }
        "add" | "drop" => {
            let Some(v) = view else { return Command::Print("no view yet".into()) };
            let pool_len = v["pool"].as_array().map_or(0, Vec::len);
            let Some(n) = words.get(1).and_then(|w| w.parse::<usize>().ok()).filter(|n| *n < pool_len) else {
                return Command::Print(format!("{first} takes a pool number (0-{})", pool_len.saturating_sub(1)));
            };
            if *first == "add" { build.main.insert(n); } else { build.main.remove(&n); }
            send_edit(v, build)
        }
        "text" => {
            let Some(v) = view else { return Command::Print("no view yet".into()) };
            Command::Print(card_text(v, words.get(1).copied()))
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
            send_edit(v, build)
        }
        word => match (word.parse::<usize>(), view) {
            (Ok(index), Some(v)) if phase == "drafting" => match v["pack"]["id"].as_u64() {
                Some(_) if early.is_some() => Command::Print(format!(
                    "{index} was not picked: it was typed {} this pack came up; type it again to pick from this pack",
                    match early.map(|e| PICK_GRACE.saturating_sub(e)) {
                        Some(after) if after > Duration::ZERO => format!("{} ms after", after.as_millis()),
                        _ => "before".to_string(),
                    })),
                Some(id) => Command::Send(vec![serde_json::json!({"type": "pick", "pack_id": id, "index": index})]),
                None => Command::Print("no pack in front of you right now".into()),
            },
            (Ok(_), _) => Command::Print(format!("a number picks a card while drafting (phase: {phase})")),
            _ => Command::Print(format!("'{word}'? type help")),
        },
    }
}

/// Everybody's state on one line: `[seat 0 picking 4p; Bo (seat 1) waiting 4p]`.
fn table_line(v: &Value) -> String {
    let parts: Vec<String> = v["seats"].as_array().into_iter().flatten().map(|x| {
        // The table's presence word, when it is not the plain "here": a
        // person in a match is wherever their game page is (#822).
        let joined = match x["presence"].as_str() {
            Some("here") => String::new(),
            Some(p) => format!(", {p}"),
            None if x["joined"].as_bool().unwrap_or(false) => String::new(),
            None => ", not here yet".to_string(),
        };
        let auto = if x["auto"].as_bool().unwrap_or(false) { ", table picks" } else { "" };
        // Named as the page names a seat (#816).
        let n = x["seat"].as_u64().unwrap_or(0);
        let who = match x["name"].as_str() {
            Some(name) if name != format!("seat {n}") => format!("{name} (seat {n})"),
            _ => format!("seat {n}"),
        };
        format!("{who} {} {}p{joined}{auto}", x["status"].as_str().unwrap_or("?"), x["picks"])
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
    // A seat as the page names it: "seat 3", or "Ann (seat 3)" once named.
    // The client's own match lines used the bare name and its pairings and
    // standings the bare number, beside a page that says "Bo (seat 1)"
    // (#816, #826).
    let who = |n: u64| -> String {
        let name = v["seats"].as_array().into_iter().flatten()
            .find(|x| x["seat"].as_u64() == Some(n)).and_then(|x| x["name"].as_str());
        match name {
            Some(name) if name != format!("seat {n}") => format!("{name} (seat {n})"),
            _ => format!("seat {n}"),
        }
    };
    table.push_str(&format!("== {} draft, seat {seat}: {phase} ==\n", v["set"].as_str().unwrap_or("?")));
    for x in v["seats"].as_array().into_iter().flatten() {
        let joined = match x["presence"].as_str() {
            Some("here") => String::new(),
            Some(p) => format!(" ({p})"),
            None if x["joined"].as_bool().unwrap_or(false) => String::new(),
            None => " (not here yet)".to_string(),
        };
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
                s.push_str("type a number to pick, or text <n> to read the card\n");
            } else {
                s.push_str("\nno pack in front of you; waiting on the seat upstream\n");
            }
            s.push_str(&pool_listing(v));
        }
        "building" => {
            // The clock the page shows beside the deck, and what happens
            // when it runs out (#813, #810).
            if let (Some(ms), false) = (v["build_deadline_ms"].as_u64(), v["deck"]["ready"].as_bool().unwrap_or(false)) {
                table.push_str(&format!(
                    "  {}s left to build; then the deck as you last sent it is played \
(basics fill a short one, up to 18 lands), or else the table builds one\n", ms / 1000));
            }
            s.push_str(&build_listing(v, build));
        }
        "playing" | "done" => {
            // This seat's own byes beside its matches, in round order: the
            // view's `matches` holds played matches only, so a seat given a
            // bye had no line of its own for it (#814).
            let me = v["seat"].as_u64();
            let mut own: Vec<(u64, Option<&Value>)> = v["matches"].as_array().into_iter().flatten()
                .map(|m| (m["round"].as_u64().unwrap_or(0), Some(m))).collect();
            own.extend(v["pairings"].as_array().into_iter().flatten()
                .filter(|p| p["b"].is_null() && p["a"].as_u64() == me)
                .map(|p| (p["round"].as_u64().unwrap_or(0), None)));
            own.sort_by_key(|(round, _)| *round);
            for (round, m) in own {
                let Some(m) = m else {
                    s.push_str(&format!("Round {round}: a bye — counted as a match won\n"));
                    continue;
                };
                let name = who(m["opponent"].as_u64().unwrap_or(0));
                let status = m["status"].as_str().unwrap_or("?");
                s.push_str(&format!("Round {} vs {name}: {status}", m["round"]));
                // The games so far, as the page lists them: a match in
                // progress read "playing" from its first game to its last,
                // with the score nowhere (#855).
                if status == "playing" {
                    if let Some(line) = games_so_far(m, me.unwrap_or(0), &who) {
                        s.push_str(&format!(" ({line})"));
                    }
                }
                // The link is for a match still to be played; a finished
                // one is its result (the page stays readable, but it is
                // not something to open).
                if let (Some(url), false) = (m["url"].as_str(), status == "done") {
                    s.push_str(&format!(" — open {url}"));
                }
                if let Some(r) = m["result"].as_str() {
                    s.push_str(&format!(" — {r}"));
                }
                // A forfeit or an abandoned game is said, as the table's own
                // score line says it; a bare "1-0" read as a game won (#744).
                let games = m["games"].as_array().cloned().unwrap_or_default();
                let count = |n: usize, what: &str| match n {
                    0 => String::new(),
                    1 => format!(" [1 game {what}]"),
                    n => format!(" [{n} games {what}]"),
                };
                s.push_str(&count(games.iter().filter(|g| !g["forfeited_by"].is_null()).count(), "forfeited"));
                s.push_str(&count(games.iter().filter(|g| g["abandoned"] == true).count(), "abandoned, no winner"));
                s.push('\n');
            }
            for p in v["pairings"].as_array().into_iter().flatten() {
                let a = who(p["a"].as_u64().unwrap_or(0));
                match p["b"].as_u64() {
                    None => s.push_str(&format!("  round {}: {a} has a bye\n", p["round"])),
                    Some(b) => s.push_str(&format!("  round {}: {a} vs {} ({}{})\n", p["round"], who(b),
                        p["status"].as_str().unwrap_or("?"),
                        p["result"].as_str().map(|r| format!(", {r}")).unwrap_or_default())),
                }
            }
            if let Some(st) = v["standings"].as_array() {
                // What they count, as the page heads them: a round is
                // recorded when its last match ends, so a bye or a match
                // "done" above is not in them yet (#818).
                s.push_str(&match v["standings_after"].as_u64() {
                    _ if phase == "done" => "Final standings:\n".to_string(),
                    Some(0) => "Standings (counted when round 1 ends):\n".to_string(),
                    Some(n) => format!("Standings after round {n}:\n"),
                    None => "Standings:\n".to_string(),
                });
                for (rank, x) in st.iter().enumerate() {
                    // W-L-D as the terminal standings print it, with their
                    // tags: a drawn match vanished from "0-0" (#744).
                    let draws = x["draws"].as_u64().filter(|d| *d > 0).map(|d| format!("-{d}")).unwrap_or_default();
                    let tags = x["tags"].as_str().filter(|t| !t.is_empty()).map(|t| format!(" {t}")).unwrap_or_default();
                    // The tiebreakers the order is by, as the runner's
                    // standings print them.
                    let pct = |k: &str| x[k].as_f64().map(|f| format!("{:.1}%", 100.0 * f))
                        .filter(|_| x["omw"].as_f64().is_some_and(|o| o > 0.0));
                    let breaks = match (pct("omw"), pct("gw"), pct("ogw")) {
                        (Some(o), Some(g), Some(og)) => format!("; OMW {o}, GW {g}, OGW {og}"),
                        _ => String::new(),
                    };
                    s.push_str(&format!("  {}. {} {}-{}{draws} ({} points{breaks}){tags}\n", rank + 1,
                        who(x["seat"].as_u64().unwrap_or(0)), x["wins"], x["losses"], x["points"]));
                }
            }
            if let Some((m, b)) = between(v) {
                if let (Some(ms), false) = (b["deadline_ms"].as_u64(), b["ready"] == true) {
                    table.push_str(&format!("  {}s left before game {}; then it starts with the deck as it stands\n",
                        ms / 1000, b["game"]));
                }
                s.push_str(&between_listing(v, m, b, build, &who));
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
        assert!(mine.contains("Round 1 vs Lawson (seat 1): done — 0-2\n"), "{mine}");
        assert!(!mine.contains("8801"), "a finished match's page is not offered:\n{mine}");
        assert!(mine.contains("Round 2 vs Lawson (seat 1): playing (game 1) — open http://h:8803/\n"), "{mine}");
        assert!(mine.contains("Round 3 vs Lawson (seat 1): waiting\n"), "{mine}");
    }

    /// #814: a seat given a bye is told so in its own section, in round
    /// order among its matches, not only in the table's pairings list.
    #[test]
    fn a_seats_own_bye_is_in_its_own_matches() {
        let v = serde_json::json!({
            "type": "view", "phase": "playing", "seat": 1, "set": "isd",
            "seats": [{"seat": 0, "name": "Ann", "kind": "human", "status": "playing", "picks": 42, "joined": true},
                      {"seat": 1, "name": "Bea", "kind": "human", "status": "waiting", "picks": 42, "joined": true},
                      {"seat": 2, "name": "Cy", "kind": "ai", "status": "playing", "picks": 42, "joined": true}],
            "matches": [{"round": 1, "opponent": 0, "url": null, "status": "done", "games": [], "result": "2-0"},
                        {"round": 3, "opponent": 2, "url": null, "status": "waiting", "games": [], "result": null}],
            "pairings": [{"round": 2, "a": 1, "b": null, "status": "done", "result": "bye"},
                         {"round": 2, "a": 0, "b": 2, "status": "playing", "result": null},
                         {"round": 1, "a": 2, "b": null, "status": "done", "result": "bye"}],
            "standings": [], "pool": [], "picks": [], "deck": null, "notice": null,
        });
        let (_, mine) = render(&v, &Build::default());
        let r1 = mine.find("Round 1 vs Ann").expect(&mine);
        let bye = mine.find("Round 2: a bye — counted as a match won\n").expect(&mine);
        let r3 = mine.find("Round 3 vs Cy").expect(&mine);
        assert!(r1 < bye && bye < r3, "{mine}");
        assert!(!mine.contains("Round 1: a bye"), "another seat's bye is not this seat's:\n{mine}");
        // The table's lines name a seat as the page does (#816).
        assert!(mine.contains("  round 2: Bea (seat 1) has a bye\n"), "{mine}");
        assert!(mine.contains("  round 2: Ann (seat 0) vs Cy (seat 2) (playing)\n"), "{mine}");
    }

    /// #816: the one-line table summary names a seat as the page does.
    #[test]
    fn the_table_line_names_a_named_seat() {
        let v = serde_json::json!({"seats": [
            {"seat": 0, "name": "seat 0", "status": "picking", "picks": 4, "joined": true},
            {"seat": 1, "name": "Bo", "status": "waiting", "picks": 4, "joined": true}]});
        assert_eq!(table_line(&v), "[seat 0 picking 4p; Bo (seat 1) waiting 4p]");
    }

    /// #792: a pool card's rules text can be read while building, and a
    /// pack card's while drafting, by the number the listing shows.
    #[test]
    fn text_reads_a_card() {
        let mut v = serde_json::json!({
            "type": "view", "phase": "building", "seat": 0, "set": "isd",
            "pool": [{"name": "Bonds of Faith", "line": "Bonds of Faith {1}{W} | Enchantment — Aura",
                      "text": "Enchant creature. Enchanted creature gets +2/+2 as long as it's a Human."}],
            "pack": null,
        });
        let build = Build::default();
        let Command::Print(t) = command("text 0", Some(&v), &mut Build::default(), None) else { panic!("printed nothing") };
        assert!(t.starts_with("Bonds of Faith {1}{W}") && t.contains("+2/+2 as long as it's a Human"), "{t}");
        let Command::Print(t) = command("text 4", Some(&v), &mut Build::default(), None) else { panic!() };
        assert!(t.contains("no card 4"), "{t}");
        assert!(build_listing(&v, &build).contains("text <n> to read it"));
        v["phase"] = "drafting".into();
        v["pack"] = serde_json::json!({"id": 1, "round": 1, "pick": 1, "size": 14, "waiting": 0, "deadline_ms": null,
            "cards": [{"index": 3, "name": "Brain Weevil", "line": "Brain Weevil {3}{B}", "text": "Intimidate"}]});
        let Command::Print(t) = command("text 3", Some(&v), &mut Build::default(), None) else { panic!() };
        assert!(t.ends_with("\nIntimidate"), "{t}");
        // With no pack in front of the seat there is no number on screen to read by.
        v["pack"] = Value::Null;
        let Command::Print(t) = command("text 0", Some(&v), &mut Build::default(), None) else { panic!() };
        assert!(t.contains("there is none now"), "{t}");
    }

    /// #815: when the table builds the deck, or tops a short one up, the
    /// screen is rebuilt from the server's record, not left on the
    /// person's own marks under "your deck is final".
    #[test]
    fn the_screen_follows_the_deck_the_table_recorded() {
        let card = |n: &str| serde_json::json!({"name": n, "line": n});
        let mut v = serde_json::json!({
            "type": "view", "phase": "building", "seat": 0, "set": "isd",
            "pool": [card("Selfless Cathar"), card("Village Ironsmith"), card("Rotting Fensnake")],
            "deck": {"main": ["Rotting Fensnake"], "lands": {"Island": 9, "Swamp": 8}, "sideboard": [],
                     "valid": true, "problem": null, "ready": false},
        });
        let mut build = Build::default();
        let mut phase = String::new();
        refresh_build(&v, &mut build, &mut phase);
        assert_eq!(build.main, BTreeSet::from([2]));
        // The person's own change, echoed back: nothing moves.
        build.main.insert(0);
        v["deck"]["main"] = serde_json::json!(["Selfless Cathar", "Rotting Fensnake"]);
        refresh_build(&v, &mut build, &mut phase);
        assert_eq!(build.main, BTreeSet::from([0, 2]));
        // A view built before the server read the next change carries the
        // older record: it does not undo the change, and the next command
        // carries it.
        build.main.insert(1);
        build.in_flight.push(deck_message(&v, &build));
        let before = v.clone();
        refresh_build(&before, &mut build, &mut phase);
        assert_eq!(build.main, BTreeSet::from([0, 1, 2]), "a stale view undid an add");
        v["deck"]["main"] = serde_json::json!(["Selfless Cathar", "Village Ironsmith", "Rotting Fensnake"]);
        refresh_build(&v, &mut build, &mut phase);
        assert!(build.in_flight.is_empty(), "the echo settles it");
        // An add and its undo, and a stale view equal to the undo: it does
        // not settle the add before it, and the screen does not jump.
        let before = v.clone();
        build.main.remove(&1);
        let without = deck_message(&v, &build);
        build.in_flight.push(without.clone());
        build.main.insert(1);
        build.in_flight.push(deck_message(&v, &build));
        v["deck"]["main"] = without["main"].clone();
        refresh_build(&v, &mut build, &mut phase); // the drop's echo
        assert_eq!(build.main, BTreeSet::from([0, 1, 2]), "the undo's echo moved the screen");
        refresh_build(&before, &mut build, &mut phase); // the add-back's echo
        assert!(build.in_flight.is_empty());
        assert_eq!(build.main, BTreeSet::from([0, 1, 2]));
        // Stale first: a record equal to the later message only.
        build.main.remove(&1);
        build.in_flight.push(deck_message(&v, &build));
        build.main.insert(1);
        build.in_flight.push(deck_message(&before, &build));
        refresh_build(&before, &mut build, &mut phase);
        assert_eq!(build.in_flight.len(), 2, "a record equal to a later message clears nothing");
        refresh_build(&v, &mut build, &mut phase);
        refresh_build(&before, &mut build, &mut phase);
        assert!(build.in_flight.is_empty());
        assert_eq!(build.main, BTreeSet::from([0, 1, 2]));
        v = before;
        // The clock ran out and the table built its own, in the same phase,
        // with a change of the person's still in flight: the final deck wins.
        build.main.remove(&0);
        build.in_flight.push(deck_message(&v, &build));
        v["deck"] = serde_json::json!({"main": ["Village Ironsmith"], "lands": {"Plains": 9, "Mountain": 8},
            "sideboard": [], "valid": true, "problem": null, "ready": true});
        refresh_build(&v, &mut build, &mut phase);
        assert_eq!(build.main, BTreeSet::from([1]));
        assert!(build.in_flight.is_empty());
        assert_eq!(build.lands.get("Island"), None);
        assert_eq!(build.lands.get("Plains"), Some(&9));
        // A short deck topped up with an Island: the count is the record's.
        v["deck"]["lands"] = serde_json::json!({"Plains": 9, "Mountain": 8, "Island": 1});
        refresh_build(&v, &mut build, &mut phase);
        assert_eq!(build.lands.get("Island"), Some(&1));
    }

    /// #813: a seat building under --build-seconds is shown its clock, and
    /// what the clock does, as the page shows it; a final deck has none.
    #[test]
    fn the_build_clock_is_shown_while_the_deck_is_not_final() {
        let mut v = serde_json::json!({
            "type": "view", "phase": "building", "seat": 0, "set": "isd",
            "seats": [{"seat": 0, "name": "seat 0", "kind": "human", "status": "building", "picks": 42, "joined": true}],
            "matches": [], "pairings": [], "standings": [], "pool": [], "picks": [], "notice": null,
            "deck": {"main": [], "lands": {}, "sideboard": [], "valid": false, "problem": null, "ready": false},
            "build_seconds": 60, "build_deadline_ms": 41_500,
        });
        let (table, _) = render(&v, &Build::default());
        assert!(table.contains("41s left to build; then the deck as you last sent it is played"), "{table}");
        v["deck"]["ready"] = true.into();
        let (table, _) = render(&v, &Build::default());
        assert!(!table.contains("left to build"), "{table}");
    }

    /// #818: a bye is "done" from the moment it is paired, and a match the
    /// moment it ends, while the standings count only finished rounds: the
    /// heading says which, as the page's does.
    #[test]
    fn the_standings_say_which_rounds_they_count() {
        let mut v = serde_json::json!({
            "type": "view", "phase": "playing", "seat": 0, "set": "isd",
            "seats": [{"seat": 0, "name": "seat 0", "kind": "human", "status": "bye", "picks": 42, "joined": true}],
            "matches": [], "pool": [], "picks": [], "deck": null, "notice": null,
            "pairings": [{"round": 1, "a": 0, "b": null, "status": "done", "result": "bye"}],
            "standings": [{"seat": 0, "wins": 0, "losses": 0, "draws": 0, "points": 0, "tags": ""}],
            "standings_after": 0,
        });
        let (_, mine) = render(&v, &Build::default());
        assert!(mine.contains("Standings (counted when round 1 ends):\n"), "{mine}");
        v["standings_after"] = 1.into();
        let (_, mine) = render(&v, &Build::default());
        assert!(mine.contains("Standings after round 1:\n"), "{mine}");
        v["phase"] = "done".into();
        let (_, mine) = render(&v, &Build::default());
        assert!(mine.contains("Final standings:\n"), "{mine}");
    }

    /// Issue #744: a forfeit, an abandoned game, a drawn match and the
    /// standings' tags are said, as the terminal standings say them.
    #[test]
    fn a_forfeit_a_draw_and_the_tags_are_said() {
        let v = serde_json::json!({
            "type": "view", "phase": "done", "seat": 0, "set": "isd",
            "seats": [{"seat": 0, "name": "seat 0", "kind": "human", "status": "done", "picks": 42, "joined": true},
                      {"seat": 1, "name": "Lawson", "kind": "ai", "status": "done", "picks": 42, "joined": true}],
            "matches": [
                {"round": 1, "opponent": 1, "url": null, "status": "done", "result": "0-1",
                 "games": [{"winner": 1, "forfeited_by": 0, "abandoned": false}]},
                {"round": 2, "opponent": 1, "url": null, "status": "done", "result": "0-0",
                 "games": [{"winner": null, "forfeited_by": null, "abandoned": true}]},
            ],
            "pairings": [], "pool": [], "picks": [], "deck": null, "notice": null,
            "standings": [{"seat": 1, "wins": 1, "losses": 0, "draws": 1, "points": 4, "tags": ""},
                          {"seat": 0, "wins": 0, "losses": 1, "draws": 1, "points": 1, "tags": "[substitute deck] [1 game forfeited]"}],
        });
        let (_, mine) = render(&v, &Build::default());
        assert!(mine.contains("Round 1 vs Lawson (seat 1): done — 0-1 [1 game forfeited]\n"), "{mine}");
        assert!(mine.contains("Round 2 vs Lawson (seat 1): done — 0-0 [1 game abandoned, no winner]\n"), "{mine}");
        assert!(mine.contains("  1. Lawson (seat 1) 1-0-1 (4 points)\n"), "{mine}");
        assert!(mine.contains("  2. seat 0 0-1-1 (1 points) [substitute deck] [1 game forfeited]\n"), "{mine}");
    }

    /// #855: a match in progress said "playing" from its first game to its
    /// last, where the page lists each game; and a person in a match is
    /// wherever their game page is (#822).
    #[test]
    fn a_match_in_progress_says_which_game_and_how_the_others_went() {
        let v = serde_json::json!({
            "type": "view", "phase": "playing", "seat": 0, "set": "isd",
            "seats": [{"seat": 0, "name": "seat 0", "kind": "human", "status": "playing", "picks": 42, "joined": true, "presence": "in game"},
                      {"seat": 1, "name": "Lawson", "kind": "human", "status": "playing", "picks": 42, "joined": true, "presence": "game page closed"}],
            "matches": [{"round": 1, "opponent": 1, "url": "http://h:1/", "status": "playing",
                         "games": [{"winner": 1}, {"winner": 0}], "result": null}],
            "pool": [], "picks": [], "deck": null, "notice": null, "pairings": [], "standings": [],
        });
        let (table, mine) = render(&v, &Build::default());
        assert!(mine.contains("Round 1 vs Lawson (seat 1): playing (game 3; G1 Lawson (seat 1), G2 you) — open http://h:1/\n"), "{mine}");
        assert!(table.contains("Lawson") && table.contains("(game page closed)"), "{table}");
        assert!(table_line(&v).contains("Lawson (seat 1) playing 42p, game page closed"), "{}", table_line(&v));
    }

    /// Between games: who chooses, what the opponent showed, the pool to
    /// sideboard with; `play`/`draw` for the chooser only, a short deck is
    /// kept on the screen and not sent, and `ready` sends the deck and
    /// then says so.
    #[test]
    fn between_games_the_client_sideboards_and_chooses() {
        let card = |n: &str| serde_json::json!({"name": n, "line": format!("{n} {{1}}{{R}}"), "text": ""});
        let pool: Vec<Value> = (0..30).map(|i| card(&format!("Card {i}"))).collect();
        let main: Vec<String> = (0..23).map(|i| format!("Card {i}")).collect();
        let mut v = serde_json::json!({
            "type": "view", "phase": "playing", "seat": 0, "set": "isd",
            "seats": [{"seat": 0, "name": "seat 0", "kind": "human", "status": "playing", "picks": 30, "joined": true},
                      {"seat": 1, "name": "Bo", "kind": "human", "status": "playing", "picks": 30, "joined": true}],
            "matches": [{"round": 1, "opponent": 1, "url": "http://h:1/", "status": "playing",
                         "games": [{"winner": 1}], "result": null,
                         "between": {"game": 2, "sideboarding": true, "you_choose": true, "play_first": null,
                                     "ready": false, "opponent_ready": false, "deadline_ms": 90_500,
                                     "deck": {"main": main, "lands": {"Mountain": 17}, "sideboard": [], "valid": true, "problem": null, "ready": false},
                                     "opponent_showed": [card("Brimstone Volley")]}}],
            "pool": pool, "picks": [], "deck": null, "notice": null, "pairings": [], "standings": [],
        });
        let mut build = Build::default();
        let mut phase = String::new();
        refresh_build(&v, &mut build, &mut phase);
        assert_eq!(build.main.len(), 23, "the deck played last game");
        let (table, mine) = render(&v, &build);
        assert!(table.contains("90s left before game 2"), "{table}");
        assert!(mine.contains("Before game 2 vs Bo (seat 1) (they lead 1-0):"), "{mine}");
        assert!(mine.contains("you lost game 1: type play or draw (not chosen yet: you play first)"), "{mine}");
        assert!(mine.contains("Bo (seat 1) has shown: Brimstone Volley"), "{mine}");
        assert!(mine.contains(" 23 [side] Card 23"), "{mine}");
        assert!(mine.contains("23 spells + 17 lands = 40 cards"), "{mine}");

        let sent = |c: Command| match c { Command::Send(m) => m, Command::Print(p) => panic!("printed {p}"), _ => panic!("nothing") };
        let printed = |c: Command| match c { Command::Print(p) => p, _ => panic!("not printed") };
        assert_eq!(sent(command("draw", Some(&v), &mut build, None)), vec![serde_json::json!({"type": "play_draw", "play": false})]);
        assert!(printed(command("drop 0", Some(&v), &mut build, None)).contains("39 cards"), "a short deck is not sent");
        let m = sent(command("add 25", Some(&v), &mut build, None));
        assert_eq!(m[0]["type"], "sideboard");
        assert!(m[0]["main"].as_array().unwrap().iter().any(|c| c == "Card 25") && m[0].get("sideboard").is_none());
        let m = sent(command("ready", Some(&v), &mut build, None));
        assert_eq!(m.iter().map(|x| x["type"].as_str().unwrap()).collect::<Vec<_>>(), ["sideboard", "sideboard_ready"]);
        // A later view of the same game keeps the edit; the next game starts from the record.
        refresh_build(&v, &mut build, &mut phase);
        assert!(build.main.contains(&25) && !build.main.contains(&0));
        v["matches"][0]["between"]["you_choose"] = false.into();
        assert!(printed(command("play", Some(&v), &mut build, None)).contains("your opponent's"));
        v["matches"][0]["between"]["game"] = 3.into();
        refresh_build(&v, &mut build, &mut phase);
        assert!(build.main.contains(&0) && !build.main.contains(&25), "game 3 starts from the deck recorded");
        v["matches"][0]["between"]["sideboarding"] = false.into();
        v["matches"][0]["between"]["game"] = 1.into();
        assert!(printed(command("add 25", Some(&v), &mut build, None)).contains("no sideboarding before game 1"));
        assert_eq!(sent(command("ready", Some(&v), &mut build, None)), vec![serde_json::json!({"type": "sideboard_ready"})]);
    }

    /// `review` asks for the seat's draft and prints it by pack.
    #[test]
    fn review_prints_every_pack_and_what_was_taken() {
        let mut build = Build::default();
        match command("review", None, &mut build, None) {
            Command::Send(m) => assert_eq!(m, vec![serde_json::json!({"type": "review"})]),
            _ => panic!("review is sent"),
        }
        let msg = serde_json::json!({"type": "review", "picks": [
            {"round": 1, "pick": 1, "card": "Chapel Geist", "auto": false,
             "pack": [{"name": "Doomed Traveler"}, {"name": "Chapel Geist"}, {"name": "Moan of the Unhallowed"}]},
            {"round": 1, "pick": 2, "card": "Typhoid Rats", "auto": true, "pack": [{"name": "Typhoid Rats"}]}]});
        let text = review_listing(&msg);
        assert!(text.contains("Pack 1\n  1.1 Chapel Geist — over Doomed Traveler, Moan of the Unhallowed\n"), "{text}");
        assert!(text.contains("  1.2 Typhoid Rats (the table's pick)\n"), "{text}");
    }

    /// The order a tie on points is broken by is printed beside the
    /// points, as the runner's standings print it.
    #[test]
    fn the_standings_carry_their_tiebreakers() {
        let v = serde_json::json!({
            "type": "view", "phase": "done", "seat": 0, "set": "isd",
            "seats": [{"seat": 0, "name": "seat 0", "kind": "human", "status": "idle", "picks": 42, "joined": true}],
            "matches": [], "pool": [], "picks": [], "deck": null, "notice": null, "pairings": [],
            "standings": [{"seat": 0, "wins": 1, "losses": 1, "draws": 0, "points": 3, "tags": "",
                           "omw": 0.75, "gw": 0.4, "ogw": 0.5}],
        });
        let (_, mine) = render(&v, &Build::default());
        assert!(mine.contains("  1. seat 0 1-1 (3 points; OMW 75.0%, GW 40.0%, OGW 50.0%)\n"), "{mine}");
    }
}

/// "game 2; G1 you": the game a match in progress is on and how each one
/// before it went, the viewing seat as "you".
fn games_so_far(m: &Value, me: u64, who: &dyn Fn(u64) -> String) -> Option<String> {
    let games = m["games"].as_array()?;
    let name = |s: u64| if s == me { "you".to_string() } else { who(s) };
    let done: Vec<String> = games.iter().enumerate().map(|(i, g)| {
        let won = match g["winner"].as_u64() {
            Some(w) => name(w),
            None if g["abandoned"] == true => "abandoned".to_string(),
            None => "drawn".to_string(),
        };
        format!("G{} {won}", i + 1)
    }).collect();
    let on = format!("game {}", games.len() + 1);
    Some(if done.is_empty() { on } else { format!("{on}; {}", done.join(", ")) })
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

/// `text <n>`: a card's headline and its rules text. The listings print
/// the headline only, and the client had no way to read a card at all,
/// where the page shows the text of the card under the cursor (#792).
/// While drafting `n` is the pack's number for the card; otherwise the
/// pool's.
fn card_text(v: &Value, arg: Option<&str>) -> String {
    // While drafting the pool is listed by counts, not numbers, so the only
    // numbers on screen are the pack's.
    if v["phase"].as_str() == Some("drafting") && !v["pack"].is_object() {
        return "text reads a card of the pack in front of you; there is none now".to_string();
    }
    let drafting = v["phase"].as_str() == Some("drafting");
    let cards: Vec<&Value> = if drafting {
        v["pack"]["cards"].as_array().into_iter().flatten().collect()
    } else {
        v["pool"].as_array().into_iter().flatten().collect()
    };
    let what = if drafting { "a card number from the pack" } else { "a pool number" };
    let Some(n) = arg.and_then(|a| a.parse::<usize>().ok()) else {
        return format!("text takes {what}");
    };
    let card = if drafting {
        cards.iter().find(|c| c["index"].as_u64() == Some(n as u64)).copied()
    } else {
        cards.get(n).copied()
    };
    let Some(card) = card else {
        return format!("no card {n}: text takes {what}");
    };
    let body = card["text"].as_str().filter(|t| !t.is_empty()).unwrap_or("(no rules text)");
    format!("{}\n{body}", card["line"].as_str().unwrap_or("?"))
}

/// Before a game of a match: who chooses to play or draw, and from game 2
/// the deck, sideboarded from the pool, beside what the opponent showed.
fn between_listing(v: &Value, m: &Value, b: &Value, build: &Build, who: &dyn Fn(u64) -> String) -> String {
    let game = b["game"].as_u64().unwrap_or(1);
    let opponent = who(m["opponent"].as_u64().unwrap_or(0));
    let me = v["seat"].as_u64();
    let wins = |s: Option<u64>| m["games"].as_array().into_iter().flatten().filter(|g| g["winner"].as_u64() == s).count();
    let (mine, theirs) = (wins(me), wins(m["opponent"].as_u64()));
    let score = match mine.cmp(&theirs) {
        _ if game == 1 => String::new(),
        std::cmp::Ordering::Equal => format!(" ({mine}-{theirs})"),
        std::cmp::Ordering::Greater => format!(" (you lead {mine}-{theirs})"),
        std::cmp::Ordering::Less => format!(" (they lead {theirs}-{mine})"),
    };
    let mut s = format!("\nBefore game {game} vs {opponent}{score}:\n");
    // The last game's loser chooses; after a draw, whoever chose it (MTR 2.2).
    let last_winner = m["games"].as_array().and_then(|g| g.last()).and_then(|g| g["winner"].as_u64());
    if b["you_choose"] == true {
        let why = match last_winner {
            _ if game == 1 => "you won the coin flip".to_string(),
            Some(w) if Some(w) != me => format!("you lost game {}", game - 1),
            _ => format!("game {} was a draw, and the choice is still yours", game - 1),
        };
        let now = match b["play_first"].as_bool() {
            None => "not chosen yet: you play first",
            Some(true) => "you play first",
            Some(false) => "you draw first",
        };
        s.push_str(&format!("  {why}: type play or draw ({now})\n"));
    } else if game == 1 {
        s.push_str(&format!("  {opponent} won the coin flip and chooses whether to play or draw\n"));
    } else if last_winner.is_some() && last_winner == me {
        s.push_str(&format!("  {opponent} lost game {} and chooses whether to play or draw\n", game - 1));
    } else {
        s.push_str(&format!("  {opponent} chose last game and chooses whether to play or draw\n"));
    }
    if b["sideboarding"] == true {
        let shown: Vec<&str> = b["opponent_showed"].as_array().into_iter().flatten()
            .filter_map(|c| c["name"].as_str()).collect();
        if !shown.is_empty() {
            s.push_str(&format!("  {opponent} has shown: {}\n", shown.join(", ")));
        }
        s.push_str("your pool — add <n> / drop <n> to sideboard, text <n> to read a card, lands plains=7 island=10:\n");
        for (i, c) in v["pool"].as_array().into_iter().flatten().enumerate() {
            let tag = if build.main.contains(&i) { "MAIN" } else { "side" };
            s.push_str(&format!("  {i:>2} [{tag}] {}\n", c["line"].as_str().unwrap_or("?")));
        }
        let lands: u32 = build.lands.values().sum();
        s.push_str(&format!("lands: {}\n", build.lands.iter().map(|(l, n)| format!("{l}={n}"))
            .collect::<Vec<_>>().join(" ")));
        s.push_str(&format!("{} spells + {lands} lands = {} cards\n", build.main.len(), build.main.len() as u32 + lands));
    }
    s.push_str(&if b["ready"] == true {
        if b["opponent_ready"] == true { format!("ready — game {game} is starting\n") } else { format!("ready — waiting for {opponent}\n") }
    } else {
        let them = if b["opponent_ready"] == true { "they are ready" } else { "they are still deciding" };
        format!("type ready to start game {game} ({them})\n")
    });
    s
}

/// `review`: every pack the seat was handed, by pack, the card taken first.
fn review_listing(msg: &Value) -> String {
    let picks = msg["picks"].as_array().cloned().unwrap_or_default();
    if picks.is_empty() {
        return "no picks yet".to_string();
    }
    let mut s = String::from("your draft:\n");
    for p in &picks {
        if p["pick"] == 1 {
            s.push_str(&format!("Pack {}\n", p["round"]));
        }
        let card = p["card"].as_str().unwrap_or("?");
        let rest: Vec<&str> = p["pack"].as_array().into_iter().flatten()
            .filter_map(|c| c["name"].as_str()).filter(|n| *n != card).collect();
        let auto = if p["auto"] == true { " (the table's pick)" } else { "" };
        s.push_str(&format!("  {}.{} {card}{auto}", p["round"], p["pick"]));
        if !rest.is_empty() {
            s.push_str(&format!(" — over {}", rest.join(", ")));
        }
        s.push('\n');
    }
    s
}

fn build_listing(v: &Value, build: &Build) -> String {
    let mut s = String::from("\nyour pool — add <n> / drop <n> to move a card, text <n> to read it, lands plains=7 island=10, ready:\n");
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

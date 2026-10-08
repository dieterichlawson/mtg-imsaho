//! The draft server: one `Lobby` behind a mutex, and the threads that
//! drive it — the page and client connections, an LLM worker per AI seat,
//! the pick and build timers, the tournament, and the host's keyboard.
//!
//! The network half is the game page's (`mtg-player/src/gui.rs`): a raw
//! `TcpListener`, a thread per connection that peeks the request head and
//! either serves a file or upgrades to a WebSocket, and whole-state
//! messages so a page that reconnects is simply sent the state again.

use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use mtg_draft::tournament::GameOutcome;
use mtg_engine::cards::CardRegistry;
use mtg_player::gui::GuiPlayer;

use crate::card_lines::CardLines;
use crate::deck::build_deck_with_llm;
use crate::game::{make_game_player, match_seed, play_match, GameSeat, MatchSeat};
use crate::llm_client::{DraftLlmClient, Table};
use crate::lobby::{Lobby, Phase, Refused, Request, SeatKind};
use crate::log_system_prompt;
use crate::pick::parse_pick_response;

/// The page the server serves at `/`.
pub const PAGE: &str = "draft.html";

/// How the server is set up, beyond the lobby.
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
    /// The host name the join lines and the game links carry.
    pub advertise: String,
    pub game_ports: (u16, u16),
    pub web_dir: PathBuf,
    pub quiet: bool,
    /// Where `games/` goes.
    pub out_dir: Option<PathBuf>,
    pub guide: Option<String>,
}

/// What every thread shares.
pub struct Shared {
    pub lobby: Mutex<Lobby>,
    /// Signalled after every change to the lobby.
    changed: Condvar,
    clients: Mutex<Vec<Client>>,
    next_client: AtomicU64,
    pub config: ServerConfig,
    pub registry: Arc<CardRegistry>,
    pub card_lines: CardLines,
    pub card_reference: String,
    pub set_name: String,
    /// The host asked to quit.
    quit: AtomicBool,
}

struct Client {
    id: u64,
    seat: usize,
    tx: mpsc::Sender<String>,
}

impl Shared {
    #[must_use]
    pub fn new(
        lobby: Lobby, config: ServerConfig, registry: Arc<CardRegistry>, card_lines: CardLines,
        card_reference: String, set_name: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            lobby: Mutex::new(lobby),
            changed: Condvar::new(),
            clients: Mutex::new(Vec::new()),
            next_client: AtomicU64::new(0),
            config,
            registry,
            card_lines,
            card_reference,
            set_name,
            quit: AtomicBool::new(false),
        })
    }

    /// The lobby, poisoned or not: a panic in one worker must not take the
    /// table down for the people at it.
    pub fn lobby(&self) -> MutexGuard<'_, Lobby> {
        self.lobby.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn clients(&self) -> MutexGuard<'_, Vec<Client>> {
        self.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Print a line on the host's terminal.
    pub fn say(&self, line: &str) {
        if !self.config.quiet {
            mtg_player::stderr_line!("{line}");
        }
    }

    /// Something changed: print what, send every page its view, wake the
    /// workers.
    pub fn notify(&self) {
        let (events, views) = {
            let mut lobby = self.lobby();
            let events = lobby.take_events();
            let clients = self.clients();
            let mut views: Vec<(usize, String)> = Vec::new();
            let mut seen: Vec<usize> = Vec::new();
            for c in clients.iter() {
                if !seen.contains(&c.seat) {
                    seen.push(c.seat);
                    if let Ok(json) = serde_json::to_string(&lobby.view(c.seat)) {
                        views.push((c.seat, json));
                    }
                }
            }
            let mut sent: Vec<usize> = Vec::new();
            for c in clients.iter() {
                if let Some((_, json)) = views.iter().find(|(s, _)| *s == c.seat) {
                    if c.tx.send(json.clone()).is_ok() {
                        sent.push(c.seat);
                    }
                }
            }
            // A notice is read once: the view that carried it went out.
            for seat in sent {
                if lobby.has_notice(seat) {
                    lobby.clear_notice(seat);
                }
            }
            (events, views.len())
        };
        let _ = views;
        for line in events {
            self.say(&line);
        }
        self.changed.notify_all();
    }

    /// Wait for a change, or `timeout`.
    fn wait<'a>(&'a self, guard: MutexGuard<'a, Lobby>, timeout: Duration) -> MutexGuard<'a, Lobby> {
        self.changed.wait_timeout(guard, timeout)
            .unwrap_or_else(std::sync::PoisonError::into_inner).0
    }

    fn add_client(&self, seat: usize, tx: mpsc::Sender<String>) -> u64 {
        let id = self.next_client.fetch_add(1, Ordering::Relaxed);
        self.clients().push(Client { id, seat, tx });
        id
    }

    fn drop_client(&self, id: u64) {
        self.clients().retain(|c| c.id != id);
    }

    fn send_to(&self, id: u64, msg: &str) {
        if let Some(c) = self.clients().iter().find(|c| c.id == id) {
            let _ = c.tx.send(msg.to_string());
        }
    }

    /// The host said to quit.
    pub fn quit(&self) {
        self.quit.store(true, Ordering::SeqCst);
        self.changed.notify_all();
    }

    #[must_use]
    pub fn quitting(&self) -> bool {
        self.quit.load(Ordering::SeqCst)
    }
}

// ───────────────────────────────────────────────────────────── addresses

/// The address a friend should use for a server bound on `bind`: the bind
/// address itself, unless it is the wildcard, in which case this machine's
/// first non-loopback IPv4 if that can be had cheaply.
#[must_use]
pub fn advertised_host(bind: &str) -> Option<String> {
    if bind != "0.0.0.0" {
        return Some(bind.to_string());
    }
    // A UDP socket "connected" to a public address sends nothing and tells
    // us which interface the kernel would route it from.
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("192.0.2.1:9").ok()?;
    let ip = sock.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

/// The join line for a human seat.
#[must_use]
pub fn join_lines(host: &str, port: u16, seat: usize, key: &str) -> String {
    format!(
        "seat {seat}  http://{host}:{port}/?seat={seat}&key={key}   \
         (or: mtg-draft-client ws://{host}:{port}/ws?seat={seat}&key={key})"
    )
}

// ───────────────────────────────────────────────────────────── run

/// Serve the table until the host quits. Returns when the host does.
///
/// # Errors
/// The listening port cannot be bound.
pub fn run(shared: &Arc<Shared>) -> Result<(), String> {
    let listener = TcpListener::bind((shared.config.bind.as_str(), shared.config.port))
        .map_err(|e| format!("cannot listen on {}:{}: {e}", shared.config.bind, shared.config.port))?;

    let accept = Arc::clone(shared);
    thread::Builder::new().name("draft-accept".into()).spawn(move || {
        for stream in listener.incoming().flatten() {
            let shared = Arc::clone(&accept);
            let _ = thread::Builder::new().name("draft-conn".into())
                .spawn(move || serve_connection(stream, &shared));
        }
    }).map_err(|e| e.to_string())?;

    let ai_seats: Vec<(usize, String)> = {
        let lobby = shared.lobby();
        (0..lobby.pod_size()).filter_map(|seat| match lobby.kind(seat) {
            Some(SeatKind::Ai(model)) => Some((seat, model.clone())),
            _ => None,
        }).collect()
    };
    for (seat, model) in ai_seats {
        let shared = Arc::clone(shared);
        thread::Builder::new().name(format!("seat {seat}")).spawn(move || ai_worker(&shared, seat, &model))
            .map_err(|e| e.to_string())?;
    }

    let timer = Arc::clone(shared);
    thread::Builder::new().name("draft-timer".into()).spawn(move || {
        while !timer.quitting() {
            thread::sleep(Duration::from_millis(250));
            let changed = timer.lobby().tick(Instant::now());
            if changed {
                timer.notify();
            }
        }
    }).map_err(|e| e.to_string())?;

    let tourney = Arc::clone(shared);
    thread::Builder::new().name("tournament".into()).spawn(move || tournament_thread(&tourney))
        .map_err(|e| e.to_string())?;

    host_loop(shared);
    Ok(())
}

/// The host's keyboard: Enter or `start` starts the draft, `kick N` hands
/// a seat to the table, `status` prints where everybody is, `quit` ends
/// the server. Returns when the host quits, or waits forever once stdin
/// is closed.
fn host_loop(shared: &Arc<Shared>) {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first().copied() {
            None | Some("start") => {
                let started = {
                    let mut lobby = shared.lobby();
                    if lobby.phase() == Phase::Lobby { lobby.start(); true } else { false }
                };
                if started { shared.notify(); } else { shared.say("the draft has already started"); }
            }
            Some("kick") => {
                let result = words.get(1).and_then(|n| n.parse::<usize>().ok())
                    .ok_or_else(|| "kick takes a seat number".to_string())
                    .and_then(|seat| shared.lobby().kick(seat));
                match result {
                    Ok(()) => shared.notify(),
                    Err(e) => shared.say(&format!("kick: {e}")),
                }
            }
            Some("status") => {
                let lobby = shared.lobby();
                shared.say(&format!("phase: {:?}", lobby.phase()));
                for seat in 0..lobby.pod_size() {
                    let v = lobby.view(seat);
                    let s = &v.seats[seat];
                    shared.say(&format!("  seat {seat} {} {} — {} ({} picks, {} connected)",
                        s.kind, s.name, s.status, s.picks, s.connected));
                }
            }
            Some("quit" | "q" | "exit") => {
                shared.quit();
                return;
            }
            Some(other) => shared.say(&format!(
                "'{other}': the commands are start (or Enter), kick <seat>, status, quit")),
        }
    }
    // stdin closed: nothing to read, so wait for the quit nobody can type.
    loop {
        if shared.quitting() {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

// ───────────────────────────────────────────────────────────── AI seats

/// What an AI worker does next.
enum Work {
    Pick(crate::lobby::PickJob),
    Deck(Vec<String>),
}

fn ai_worker(shared: &Arc<Shared>, seat: usize, model: &str) {
    let (pod_size, pack_size) = {
        let lobby = shared.lobby();
        (lobby.pod_size(), lobby.pack_size())
    };
    let table = Table { seat, pod_size, pack_size };
    let client = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| DraftLlmClient::new(
        model, &shared.set_name, shared.config.guide.as_deref(), &shared.card_reference, table,
    )));
    let mut client = match client {
        Ok(c) => c,
        Err(payload) => {
            shared.lobby().seat_gave_up(seat, &crate::panic_message(&payload));
            shared.notify();
            return;
        }
    };
    log_system_prompt!((), seat, client.system_prompt(), None);

    loop {
        let work = {
            let mut lobby = shared.lobby();
            loop {
                if shared.quitting() || lobby.phase() == Phase::Done || lobby.is_auto(seat) {
                    return;
                }
                if let Some(job) = lobby.pick_job(seat) {
                    break Work::Pick(job);
                }
                if let Some(pool) = lobby.deck_job(seat) {
                    break Work::Deck(pool);
                }
                lobby = shared.wait(lobby, Duration::from_secs(1));
            }
        };
        match work {
            Work::Pick(job) => {
                // The seat's notes from its last pick open the prompt, as
                // in the runner: the conversation does not carry the
                // earlier picks.
                let prompt = DraftLlmClient::notes_section(client.notes())
                    + &DraftLlmClient::build_pick_prompt(
                        table, job.round, job.pick, &job.cards, &job.pool, &shared.card_lines);
                let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || client.send_pick_message(&prompt, job.cards.len())));
                let response = match response {
                    Ok(r) => r,
                    Err(payload) => {
                        shared.lobby().seat_gave_up(seat, &crate::panic_message(&payload));
                        shared.notify();
                        return;
                    }
                };
                let pick = parse_pick_response(&response, &job.cards);
                let substituted = pick.was_substituted();
                let index = job.cards.iter().position(|c| c == pick.card()).unwrap_or(0);
                let applied = shared.lobby().ai_pick(seat, job.pack_id, index, &prompt, &response, substituted);
                if let Err(e) = applied {
                    shared.say(&format!("WARN seat {seat}: a pick could not be applied: {e}"));
                }
                shared.notify();
            }
            Work::Deck(pool) => {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || build_deck_with_llm(&mut client, &pool, &shared.registry, &shared.card_lines)));
                match result {
                    Ok(result) => shared.lobby().ai_deck(seat, &result),
                    Err(payload) => shared.lobby().seat_gave_up(seat, &crate::panic_message(&payload)),
                }
                shared.notify();
            }
        }
    }
}

// ───────────────────────────────────────────────────────────── the games

fn tournament_thread(shared: &Arc<Shared>) {
    let mut to_play = {
        let mut lobby = shared.lobby();
        loop {
            if shared.quitting() {
                return;
            }
            if lobby.all_ready() {
                break lobby.begin_tournament();
            }
            lobby = shared.wait(lobby, Duration::from_secs(1));
        }
    };
    shared.notify();
    while !to_play.is_empty() && !shared.quitting() {
        let next: Vec<(usize, usize, usize)> = thread::scope(|s| {
            let handles: Vec<_> = to_play.iter().map(|&(round, a, b)| {
                let shared = Arc::clone(shared);
                thread::Builder::new().name(format!("seat {a} v {b}"))
                    .spawn_scoped(s, move || play_one(&shared, round, a, b))
                    .expect("a match thread")
            }).collect();
            handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
        });
        to_play = next;
        shared.notify();
    }
    shared.notify();
}

/// Play one match, or forfeit it for a seat the table plays for. Returns
/// the next round's matches when this one ended the round.
fn play_one(shared: &Arc<Shared>, round: usize, a: usize, b: usize) -> Vec<(usize, usize, usize)> {
    let (kind_a, kind_b, forfeit, deck_a, deck_b, best_of, seed) = {
        let lobby = shared.lobby();
        let forfeit = (lobby.is_auto(a) || lobby.is_auto(b)).then(|| lobby.forfeit_result(a, b));
        (
            lobby.kind(a).cloned().unwrap_or(SeatKind::Human),
            lobby.kind(b).cloned().unwrap_or(SeatKind::Human),
            forfeit,
            lobby.decklist(a),
            lobby.decklist(b),
            lobby.best_of(),
            match_seed(lobby.seed(), round, a, b),
        )
    };
    if let Some(result) = forfeit {
        return finish(shared, round, a, b, result);
    }
    let (Some(deck_a), Some(deck_b)) = (deck_a, deck_b) else {
        shared.say(&format!("WARN round {round}: seat {a} vs seat {b} has no decks to play with"));
        return Vec::new();
    };

    let mut urls: [Option<String>; 2] = [None, None];
    let mut seats: Vec<GameSeat> = Vec::new();
    for (i, (seat, kind)) in [(a, kind_a), (b, kind_b)].into_iter().enumerate() {
        let name = format!("Seat{seat}");
        match kind {
            SeatKind::Ai(model) => seats.push(GameSeat::Llm(
                make_game_player(&model, &name, shared.config.guide.as_deref()))),
            SeatKind::Human | SeatKind::Cli => match game_page(shared, &name) {
                Ok(gui) => {
                    urls[i] = Some(gui.url.clone());
                    seats.push(GameSeat::Gui(gui));
                }
                Err(e) => {
                    shared.say(&format!("WARN round {round}: no game page for seat {seat}: {e}; \
the match is forfeit"));
                    let result = shared.lobby().forfeit_result(a, b);
                    return finish(shared, round, a, b, result);
                }
            },
        }
    }
    shared.lobby().match_started(round, a, b, urls);
    shared.notify();

    let mut seats = seats.into_iter();
    let (player_a, player_b) = (seats.next().expect("seat a"), seats.next().expect("seat b"));
    let mut games = 0usize;
    let result = play_match(
        MatchSeat { seat: a, deck: &deck_a, player: player_a },
        MatchSeat { seat: b, deck: &deck_b, player: player_b },
        &shared.registry,
        best_of,
        seed,
        &mut |outcome: &GameOutcome| {
            games += 1;
            write_game_log(shared.config.out_dir.as_deref(), round, a, b, games, outcome);
            shared.lobby().game_finished(round, a, b, outcome);
            shared.notify();
        },
    );
    let next = shared.lobby().match_finished(round, a, b, result);
    shared.notify();
    next
}

/// Record a match that was not played (a forfeit), and tell the pages.
fn finish(shared: &Arc<Shared>, round: usize, a: usize, b: usize, result: mtg_draft::tournament::MatchResult) -> Vec<(usize, usize, usize)> {
    let next = {
        let mut lobby = shared.lobby();
        lobby.match_started(round, a, b, [None, None]);
        lobby.match_finished(round, a, b, result)
    };
    shared.notify();
    next
}

/// A browser page for one human's game, on the next free game port.
fn game_page(shared: &Shared, name: &str) -> Result<GuiPlayer, String> {
    let (lo, hi) = shared.config.game_ports;
    let mut last = String::from("no ports to try");
    for port in lo..=hi {
        match GuiPlayer::new_at(name, &shared.config.bind, Some(port)) {
            Ok(mut gui) => {
                gui.url = format!("http://{}:{port}/", shared.config.advertise);
                return Ok(gui);
            }
            Err(e) => last = e,
        }
    }
    Err(format!("no free port in {lo}-{hi} ({last})"))
}

fn write_game_log(out_dir: Option<&Path>, round: usize, a: usize, b: usize, game: usize, outcome: &GameOutcome) {
    let Some(dir) = out_dir else { return };
    let dir = dir.join("games");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("r{round}-{a}v{b}-g{game}.log"));
    let mut text = outcome.game_log.join("\n");
    text.push('\n');
    let _ = std::fs::write(path, text);
}

// ───────────────────────────────────────────────────────────── connections

/// One TCP connection: a page fetch, the view API, or a WebSocket.
fn serve_connection(mut stream: TcpStream, shared: &Arc<Shared>) {
    let mut head = [0u8; 2048];
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let Ok(n) = stream.peek(&mut head) else { return };
    let head = String::from_utf8_lossy(&head[..n]);
    let lower = head.to_ascii_lowercase();
    let path = head.lines().next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();
    if lower.contains("upgrade: websocket") {
        serve_websocket(stream, shared, &path);
    } else {
        serve_http(&mut stream, shared, &path);
    }
}

/// `seat` and `key` out of a query string.
fn seat_and_key(path: &str) -> (Option<usize>, Option<String>) {
    let query = path.split_once('?').map_or("", |(_, q)| q);
    let mut seat = None;
    let mut key = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("seat", v)) => seat = v.parse().ok(),
            Some(("key", v)) => key = Some(v.to_string()),
            _ => {}
        }
    }
    (seat, key)
}

fn check_join(shared: &Shared, path: &str) -> Result<usize, String> {
    let (seat, key) = seat_and_key(path);
    let seat = seat.ok_or("the link needs ?seat=N")?;
    let key = key.ok_or("the link needs &key=...")?;
    shared.lobby().check_key(seat, &key)?;
    Ok(seat)
}

fn http_response(stream: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n\
         Cache-Control: no-cache\r\nConnection: close\r\n\r\n", body.len());
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

/// `GET /` is the draft page, `/dist/*` and `/assets/*` its files,
/// `/api/view?seat=N&key=K` the seat's view as JSON.
fn serve_http(stream: &mut TcpStream, shared: &Shared, path: &str) {
    // Drain the request head.
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 16 * 1024 {
                    break;
                }
            }
            Ok(_) | Err(_) => break,
        }
    }
    let bare = path.split('?').next().unwrap_or("/");
    if bare == "/api/view" {
        return match check_join(shared, path) {
            Ok(seat) => {
                let json = serde_json::to_string(&shared.lobby().view(seat)).unwrap_or_default();
                http_response(stream, "200 OK", "application/json", json.as_bytes());
            }
            Err(e) => http_response(stream, "403 Forbidden", "text/plain", e.as_bytes()),
        };
    }
    let rel = if bare == "/" { PAGE } else { bare.trim_start_matches('/') };
    let allowed = rel == PAGE || rel.starts_with("dist/") || rel.starts_with("assets/");
    let safe = allowed && !rel.split('/').any(|seg| seg == ".." || seg.is_empty());
    let file = shared.config.web_dir.join(rel);
    let body = if safe { std::fs::read(&file).ok() } else { None };
    match body {
        Some(b) => http_response(stream, "200 OK", content_type(rel), &b),
        None if rel == PAGE => http_response(stream, "404 Not Found", "text/plain", format!(
            "the draft page is not at {} — run from the repository root or set {} to the mtg-gui directory",
            file.display(), mtg_player::gui::WEB_DIR_ENV).as_bytes()),
        None => http_response(stream, "404 Not Found", "text/plain", b"not found"),
    }
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("ttf") => "font/ttf",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn refused(reason: &str, echo: serde_json::Value) -> String {
    serde_json::to_string(&Refused { kind: "refused", reason: reason.to_string(), echo })
        .expect("a refusal serializes")
}

/// One client's WebSocket: the seat's view on connect and after every
/// change, its requests in, refusals back to it alone.
fn serve_websocket(stream: TcpStream, shared: &Arc<Shared>, path: &str) {
    let Ok(mut ws) = tungstenite::accept(stream) else { return };
    let seat = match check_join(shared, path) {
        Ok(seat) => seat,
        Err(e) => {
            let _ = ws.send(tungstenite::Message::Text(refused(&e, serde_json::Value::Null).into()));
            let _ = ws.close(None);
            let _ = ws.flush();
            return;
        }
    };
    let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(50)));
    let (tx, rx) = mpsc::channel::<String>();
    let id = shared.add_client(seat, tx);
    shared.lobby().connected(seat);
    shared.notify();
    pump(&mut ws, &rx, shared, id, seat);
    shared.drop_client(id);
    shared.lobby().disconnected(seat);
    shared.notify();
}

fn pump(
    ws: &mut tungstenite::WebSocket<TcpStream>,
    rx: &mpsc::Receiver<String>,
    shared: &Arc<Shared>,
    id: u64,
    seat: usize,
) {
    loop {
        if shared.quitting() {
            let _ = ws.close(None);
            return;
        }
        while let Ok(msg) = rx.try_recv() {
            if ws.send(tungstenite::Message::Text(msg.into())).is_err() {
                return;
            }
        }
        match ws.read() {
            Ok(tungstenite::Message::Text(text)) => {
                let echo: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
                let request: Result<Request, _> = serde_json::from_str(&text);
                let outcome = match request {
                    Ok(Request::Hello) => {
                        let json = serde_json::to_string(&shared.lobby().view(seat)).unwrap_or_default();
                        shared.send_to(id, &json);
                        continue;
                    }
                    Ok(Request::Pick { pack_id, index }) => shared.lobby().pick(seat, pack_id, index),
                    Ok(Request::Deck { main, lands, sideboard }) =>
                        shared.lobby().submit_deck(seat, &main, &lands, &sideboard),
                    Ok(Request::Ready) => shared.lobby().ready(seat),
                    Ok(Request::Name { name }) => { shared.lobby().set_name(seat, &name); Ok(()) }
                    Err(e) => Err(format!("unreadable message: {e}")),
                };
                match outcome {
                    Ok(()) => shared.notify(),
                    Err(reason) => {
                        shared.send_to(id, &refused(&reason, echo));
                        // The view too: a refused deck is kept with its
                        // problem, and the page shows both.
                        shared.notify();
                    }
                }
            }
            Ok(tungstenite::Message::Close(_)) => return,
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_string_names_the_seat_and_the_key() {
        assert_eq!(seat_and_key("/ws?seat=3&key=abc"), (Some(3), Some("abc".into())));
        assert_eq!(seat_and_key("/ws?key=abc&seat=0"), (Some(0), Some("abc".into())));
        assert_eq!(seat_and_key("/ws"), (None, None));
        assert_eq!(seat_and_key("/ws?seat=x"), (None, None));
    }

    #[test]
    fn a_bound_address_is_advertised_as_itself_and_the_wildcard_as_a_real_one() {
        assert_eq!(advertised_host("127.0.0.1").as_deref(), Some("127.0.0.1"));
        assert_eq!(advertised_host("192.168.1.20").as_deref(), Some("192.168.1.20"));
        if let Some(host) = advertised_host("0.0.0.0") {
            assert_ne!(host, "0.0.0.0");
            assert_ne!(host, "127.0.0.1");
        }
    }

    #[test]
    fn the_join_line_carries_both_ways_in() {
        let line = join_lines("192.168.1.20", 8800, 1, "k3");
        assert!(line.starts_with("seat 1  http://192.168.1.20:8800/?seat=1&key=k3"));
        assert!(line.contains("mtg-draft-client ws://192.168.1.20:8800/ws?seat=1&key=k3"));
    }
}

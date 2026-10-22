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
use mtg_player::gui::{GuiPlayer, PageHandle, Withdraw};

use crate::card_lines::CardLines;
use crate::deck::build_deck_with_llm;
use crate::deck::{sideboard_with_llm, BetweenGames};
use crate::game::{last_game_line, make_game_player, match_seed, play_match, BeforeGame, GameSeat, GameSetup, MatchSeat};
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
    /// Where the table's snapshot is written after every change, for
    /// `--resume`; nowhere when `None`.
    pub save: Option<PathBuf>,
    /// Hand out each game page's own port, bound on `bind`, instead of a
    /// link through this port (`/game/<port>/<key>/`, the page on loopback).
    pub direct_game_ports: bool,
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
    /// Why the table is ending, as the log's last line and every page say
    /// it: the host's `quit`, or a signal (#823, #824).
    stop_reason: Mutex<Option<String>>,
    /// The game pages of the people in a match now, by seat: what a kick
    /// reaches into a running match with (#821).
    in_game: Mutex<Vec<InGame>>,
    /// Where the next game page's port search starts: past the last one
    /// handed out, so a finished match's port, let go when its seat is
    /// dropped (#853), is not the next match's while the range has others.
    /// A page left open from that match reconnects to its own port.
    next_game_port: Mutex<Option<u16>>,
    /// The host's own seat (`cli`): the lines the host types that are not
    /// the host's commands, for the terminal client the server runs for it.
    cli_lines: Mutex<Option<mpsc::Sender<(Instant, String)>>>,
    /// The last snapshot write failed: said once, and again when one
    /// succeeds (#891).
    snapshot_failing: AtomicBool,
}

/// A person's game page in a running match.
struct InGame {
    seat: usize,
    opponent: usize,
    /// Takes this seat out of the game.
    withdraw: Withdraw,
    /// Stops this seat waiting on its page when the opponent is taken
    /// out (#854).
    wake: Withdraw,
    /// The page itself: whether anybody is at it, and what it is told
    /// about the opponent's (#822).
    page: PageHandle,
    /// Where the page is served: what `/game/<port>/` on the lobby's own
    /// port is passed to, and nothing else is; the page itself answers
    /// only under its key (#872).
    addr: std::net::SocketAddr,
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
            snapshot_failing: AtomicBool::new(false),
            stop_reason: Mutex::new(None),
            in_game: Mutex::new(Vec::new()),
            next_game_port: Mutex::new(None),
            cli_lines: Mutex::new(None),
        })
    }

    /// Hand a human seat to the table, and take it out of the game it is
    /// playing, if any: the game and the rest of the match are forfeit,
    /// as the kick says. The kick used to reach only matches not yet
    /// started, so a running one waited for ever on the kicked person's
    /// page and the round never ended (#821).
    ///
    /// # Errors
    /// As `Lobby::kick`.
    pub fn kick(&self, seat: usize) -> Result<(), String> {
        self.lobby().kick(seat)?;
        self.withdraw_if_auto(seat);
        Ok(())
    }

    /// Withdraw the seat's game page if the table now plays for the seat.
    fn withdraw_if_auto(&self, seat: usize) {
        if !self.lobby().is_auto(seat) {
            return;
        }
        let who = self.lobby().who(seat);
        let why = format!("the host kicked {who}");
        let in_game = self.in_game.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut withdrawn = false;
        for page in in_game.iter().filter(|p| p.seat == seat) {
            page.withdraw.withdraw(&why);
            withdrawn = true;
        }
        // An AI seat the table now plays for has no page to withdraw, and
        // waking its opponent then would end the game with nobody's loss.
        if !withdrawn {
            return;
        }
        // The opponent may be the one holding the decision, and the kicked
        // seat is not asked again until it answers: it is woken so the game
        // ends now, as the kick says (#854). Only after the withdrawal, so
        // the woken seat's loop finds the kicked seat already out.
        for page in in_game.iter().filter(|p| p.opponent == seat) {
            page.wake.withdraw(&why);
        }
    }

    fn set_in_game(&self, seat: usize, page: Option<InGame>) {
        let mut in_game = self.in_game.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        in_game.retain(|p| p.seat != seat);
        if let Some(page) = page {
            in_game.push(page);
        }
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

    /// The answer to something the host typed: printed under `--quiet` as
    /// well, which quiets the table's own lines, not the host's commands
    /// (`status` printed nothing at all under it).
    pub fn reply(&self, line: &str) {
        mtg_player::stderr_line!("{line}");
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
        self.keep_snapshot();
        self.changed.notify_all();
    }

    /// Write the table's snapshot, whole, to a file beside it and then over
    /// it, so a server that dies mid-write leaves the last good one.
    ///
    /// # Errors
    /// The snapshot could not be written, and why.
    pub fn write_snapshot(&self) -> Result<(), String> {
        let Some(path) = &self.config.save else { return Ok(()) };
        let json = serde_json::to_string(&self.lobby().snapshot()).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json).and_then(|()| std::fs::rename(&tmp, path))
            .map_err(|e| format!("the snapshot cannot be written to {}: {e}", path.display()))
    }

    /// [`Shared::write_snapshot`] after a change. A write that fails is
    /// said on the host's terminal and in the log, once until one succeeds
    /// again: both errors were dropped, and a host told where the snapshot
    /// was found nothing to resume from (#891).
    fn keep_snapshot(&self) {
        let written = self.write_snapshot();
        let was_failing = self.snapshot_failing.swap(written.is_err(), Ordering::SeqCst);
        let line = match (&written, was_failing) {
            (Err(e), false) => format!("WARN {e}; if the server stops now, --resume has nothing newer to carry on from"),
            (Ok(()), true) => format!("NOTE the snapshot is written again{}",
                self.config.save.as_ref().map(|p| format!(" to {}", p.display())).unwrap_or_default()),
            _ => return,
        };
        mtg_player::game_log::write(file!(), line!(), &line, "");
        self.reply(&line);
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
        self.quit_because("the host quit");
    }

    /// End the table, saying why. The first reason given stands, and is
    /// queued to every client before the flag is set: a connection that
    /// sees the flag closes, and one that saw it before the message was
    /// queued closed without it.
    pub fn quit_because(&self, reason: &str) {
        {
            let mut slot = self.stop_reason.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if slot.is_some() {
                return;
            }
            *slot = Some(reason.to_string());
        }
        let closed = serde_json::json!({"type": "closed", "reason": reason}).to_string();
        for c in self.clients().iter() {
            let _ = c.tx.send(closed.clone());
        }
        self.quit.store(true, Ordering::SeqCst);
        self.changed.notify_all();
    }

    /// Why the table ended, once it has.
    #[must_use]
    pub fn stop_reason(&self) -> Option<String> {
        self.stop_reason.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    /// Every page and client is told the table is over and why, and given
    /// a moment to be closed properly: the process used to exit before the
    /// connection threads had their next turn, so the sockets were reset
    /// and every page retried for ever, reading the stop as a dropped
    /// network (#824).
    pub fn close_everything(&self) {
        let reason = self.stop_reason().unwrap_or_else(|| "the table is over".to_string());
        for page in self.in_game.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter() {
            page.page.close(&reason);
        }
        let until = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < until && !self.clients().is_empty() {
            thread::sleep(Duration::from_millis(25));
        }
        // The game pages' sockets are closed by their own threads, on
        // their next beat (50 ms) and a moment for the page's answer.
        thread::sleep(Duration::from_millis(600));
    }

    /// Bring the table's record of who is at a game page up to date, and
    /// tell each person in a match whether their opponent's page is open:
    /// "Opponent is thinking" over a closed tab was all they got (#822).
    fn poll_game_pages(&self) {
        let pages: Vec<(usize, usize, usize, PageHandle)> = self.in_game.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner).iter()
            .map(|p| (p.seat, p.opponent, p.page.connected(), p.page.clone())).collect();
        let mut changed = false;
        for &(seat, opponent, open, _) in &pages {
            changed |= self.lobby().set_game_pages(seat, Some(open));
            let Some((_, _, _, their_page)) = pages.iter().find(|(s, ..)| *s == opponent) else { continue };
            let line = (open == 0).then(|| format!(
                "{}'s game page is closed: the game waits for them to open it again",
                self.lobby().who(seat)));
            their_page.set_opponent(line);
        }
        if changed {
            self.notify();
        }
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

/// Take the table's port. Done before anything is said or started: an
/// all-AI table used to start its draft, and log "the draft has started"
/// and "--- Pack 1 ---", before finding the port taken, leaving a log of a
/// draft that never ran (#745, the shape of #735).
///
/// # Errors
/// The listening port cannot be bound.
pub fn bind(shared: &Arc<Shared>) -> Result<TcpListener, String> {
    TcpListener::bind((shared.config.bind.as_str(), shared.config.port))
        .map_err(|e| format!("cannot listen on {}:{}: {e}", shared.config.bind, shared.config.port))
}

/// Serve the table on `listener` until the host quits. Returns when the
/// host does.
///
/// # Errors
/// A worker thread cannot be started.
pub fn run(shared: &Arc<Shared>, listener: TcpListener) -> Result<(), String> {
    let accept = Arc::clone(shared);
    thread::Builder::new().name("draft-accept".into()).spawn(move || {
        for stream in listener.incoming().flatten() {
            let shared = Arc::clone(&accept);
            let _ = thread::Builder::new().name("draft-conn".into())
                .spawn(move || serve_connection(stream, &shared));
        }
    }).map_err(|e| e.to_string())?;

    // The host's own seat: the terminal client, run here against this
    // table's own port, its lines taken from the host's keyboard.
    let cli = {
        let lobby = shared.lobby();
        (0..lobby.pod_size()).find(|s| lobby.kind(*s) == Some(&SeatKind::Cli))
            .and_then(|seat| lobby.human_keys().into_iter().find(|(s, _)| *s == seat))
    };
    if let Some((seat, key)) = cli {
        let host = match shared.config.bind.as_str() {
            "0.0.0.0" | "::" | "[::]" => "127.0.0.1".to_string(),
            other => other.to_string(),
        };
        let url = format!("ws://{host}:{}/ws?seat={seat}&key={key}", shared.config.port);
        // Joined before the keyboard is read: a `start` or a name typed at
        // once went to a table the seat was not at yet, which picked for it.
        let join = move || -> Result<_, String> {
            let (ws, _) = tungstenite::connect(&url)
                .map_err(|e| format!("your seat at this terminal could not join: {e}"))?;
            if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_ref() {
                let _ = s.set_read_timeout(Some(Duration::from_millis(100)));
            }
            Ok(ws)
        };
        let mut ws = join()?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while shared.lobby().view(seat).seats[seat].presence == "not yet" && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        let (tx, rx) = mpsc::channel();
        *shared.cli_lines.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(tx);
        let seat_shared = Arc::clone(shared);
        thread::Builder::new().name("cli-seat".into()).spawn(move || loop {
            let ended = crate::client::run(&mut ws, &rx);
            if seat_shared.quitting() || matches!(ended, crate::client::Ended::Refused) {
                return;
            }
            // The table is still on: the host's seat joins it again rather
            // than leaving nobody at it, with what was typed meanwhile
            // still queued (#880).
            seat_shared.reply("your seat at this terminal lost its connection; joining again");
            let mut tries = 0;
            ws = loop {
                match join() {
                    Ok(ws) => break ws,
                    Err(e) if tries >= 10 => {
                        seat_shared.reply(&format!("{e}; what you type for it now goes nowhere"));
                        return;
                    }
                    Err(_) => { tries += 1; thread::sleep(Duration::from_millis(500)); }
                }
            };
        }).map_err(|e| e.to_string())?;
    }

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

    let pages = Arc::clone(shared);
    thread::Builder::new().name("game-pages".into()).spawn(move || {
        while !pages.quitting() {
            thread::sleep(Duration::from_secs(1));
            pages.poll_game_pages();
        }
    }).map_err(|e| e.to_string())?;

    // The keyboard on a thread of its own, so a Ctrl-C is seen while the
    // host's terminal is waiting for a line (#823).
    let keyboard = Arc::clone(shared);
    thread::Builder::new().name("host".into()).spawn(move || host_loop(&keyboard))
        .map_err(|e| e.to_string())?;
    while !shared.quitting() {
        if let Some(sig) = mtg_player::llm::stop_signal() {
            shared.quit_because(&format!("the server was stopped by {}", signal_name(sig)));
        }
        thread::sleep(Duration::from_millis(100));
    }
    shared.close_everything();
    Ok(())
}

/// A stop signal as a host knows it.
#[must_use]
pub fn signal_name(sig: i32) -> String {
    match sig {
        2 => "Ctrl-C (SIGINT)".to_string(),
        15 => "SIGTERM".to_string(),
        1 => "SIGHUP".to_string(),
        n => format!("signal {n}"),
    }
}

/// The host's keyboard: Enter or `start` starts the draft, `kick N` hands
/// a seat to the table, `status` prints where everybody is, `quit` ends
/// the server. Returns when the host quits, or waits forever once stdin
/// is closed.
fn host_loop(shared: &Arc<Shared>) {
    let stdin = std::io::stdin();
    // With a `cli` seat the keyboard is the seat's too: what is not one of
    // the host's commands is the seat's, as `mtg-draft-client` reads it.
    let cli = shared.cli_lines.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let typed_at = Instant::now();
        let line = line.trim();
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first().copied() {
            None | Some("start") => {
                let started = {
                    let mut lobby = shared.lobby();
                    if lobby.phase() == Phase::Lobby { lobby.start(); true } else { false }
                };
                if started {
                    shared.notify();
                } else if words.is_empty() && cli.is_some() {
                    // Enter at the seat's prompt, not a second start.
                } else {
                    shared.reply("the draft has already started");
                }
            }
            Some("help" | "?") => {
                shared.reply("host: start (or Enter), kick <seat>, status, quit");
                if cli.is_some() {
                    shared.reply(&format!("your seat:\n{}", crate::client::USAGE));
                }
            }
            Some("kick") => {
                let result = words.get(1).and_then(|n| n.parse::<usize>().ok())
                    .ok_or_else(|| "kick takes a seat number".to_string())
                    .and_then(|seat| {
                        // The host's own seat is at this keyboard, and a kick
                        // is for good: a mistyped seat number ended the
                        // host's tournament with no question asked (#909).
                        let own = shared.lobby().kind(seat) == Some(&SeatKind::Cli);
                        if own && words.get(2) != Some(&"yes") {
                            return Err(format!("seat {seat} is yours, at this terminal: `kick {seat} yes` \
hands it to the table for good, and its games are forfeit"));
                        }
                        shared.kick(seat)
                    });
                match result {
                    Ok(()) => shared.notify(),
                    Err(e) => shared.reply(&format!("kick: {e}")),
                }
            }
            Some("status") => {
                let lobby = shared.lobby();
                shared.reply(&format!("phase: {:?}", lobby.phase()));
                for seat in 0..lobby.pod_size() {
                    let v = lobby.view(seat);
                    let s = &v.seats[seat];
                    let kind = match lobby.kind(seat) {
                        Some(SeatKind::Ai(spec)) => crate::llm_client::ai_seat_label(spec),
                        _ => s.kind.to_string(),
                    };
                    shared.reply(&format!("  seat {seat} {kind} {} — {}, {} ({} picks)",
                        s.name, s.presence, s.status, s.picks));
                }
            }
            Some("quit" | "q" | "exit") => {
                shared.quit();
                return;
            }
            Some(other) => match &cli {
                Some(tx) => if tx.send((typed_at, line.to_string())).is_err() {
                    shared.reply(&format!("'{other}' went nowhere: your seat at this terminal is no longer joined"));
                },
                None => shared.reply(&format!(
                    "'{other}': the commands are start (or Enter), kick <seat>, status, quit")),
            },
        }
    }
    if let Some(tx) = &cli {
        let _ = tx.send((Instant::now(), "\u{0}eof".to_string()));
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
    // A table carried on from a snapshot: the seat's notes from its last
    // pick open its next prompt, as they would have.
    client.restore_notes(shared.lobby().ai_notes(seat));

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
            Work::Pick(job) if job.cards.len() == 1 => {
                // A pack's last card is no choice: taken without a call,
                // as in the runner.
                if let Err(e) = shared.lobby().ai_pick(seat, job.pack_id, 0, "", "", false) {
                    let who = shared.lobby().who(seat);
                    shared.say(&format!("WARN {who}: a pick could not be applied: {e}"));
                }
                shared.notify();
            }
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
                let applied = {
                    let mut lobby = shared.lobby();
                    lobby.set_ai_notes(seat, client.notes().map(str::to_string));
                    lobby.ai_pick(seat, job.pack_id, index, &prompt, &response, substituted)
                };
                if let Err(e) = applied {
                    let who = shared.lobby().who(seat);
                    shared.say(&format!("WARN {who}: a pick could not be applied: {e}"));
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
        let (wa, wb) = { let l = shared.lobby(); (l.who(a), l.who(b)) };
        shared.say(&format!("WARN round {round}: {wa} vs {wb} has no decks to play with"));
        return Vec::new();
    };

    let mut urls: [Option<String>; 2] = [None, None];
    let mut seats: Vec<GameSeat> = Vec::new();
    let kinds = [kind_a.clone(), kind_b.clone()];
    // Bound, not inline: a guard in a `match` scrutinee is held through
    // its arms, which take the lobby again.
    let label_of = |seat: usize| shared.lobby().who(seat);
    for (i, (seat, kind)) in [(a, kind_a), (b, kind_b)].into_iter().enumerate() {
        let name = format!("Seat{seat}");
        match kind {
            SeatKind::Ai(model) => seats.push(GameSeat::Llm(
                make_game_player(&model, &name, shared.config.guide.as_deref()))),
            SeatKind::Human | SeatKind::Cli => match game_page(shared, &name, &label_of(seat)) {
                Ok(gui) => {
                    urls[i] = Some(gui.url.clone());
                    shared.set_in_game(seat, Some(InGame {
                        seat, opponent: if seat == a { b } else { a },
                        withdraw: gui.withdraw_handle(), wake: gui.wake_handle(),
                        page: gui.page_handle(), addr: gui.local_addr(),
                    }));
                    shared.lobby().set_game_pages(seat, Some(0));
                    seats.push(GameSeat::Gui(gui));
                }
                Err(e) => {
                    let who = shared.lobby().who(seat);
                    shared.say(&format!("WARN round {round}: no game page for {who}: {e}; \
{who} forfeits the match"));
                    let result = shared.lobby().forfeit_by(a, b, seat,
                        &format!("no game page for {who}: {e}"));
                    return finish(shared, round, a, b, result);
                }
            },
        }
    }
    shared.lobby().match_started(round, a, b, urls);
    shared.notify();
    // A kick between the check above and the page being registered.
    shared.withdraw_if_auto(a);
    shared.withdraw_if_auto(b);

    let mut seats = seats.into_iter();
    let (player_a, player_b) = (seats.next().expect("seat a"), seats.next().expect("seat b"));
    let mut games = 0usize;
    let (label_a, label_b) = { let l = shared.lobby(); (l.who(a), l.who(b)) };
    let mut between_clients: [Option<DraftLlmClient>; 2] = [None, None];
    let mut before_game = |bg: &BeforeGame| -> GameSetup {
        between_games(shared, round, a, b, best_of, &kinds, &mut between_clients, bg)
    };
    let result = play_match(
        MatchSeat { seat: a, label: label_a, deck: deck_a, player: player_a },
        MatchSeat { seat: b, label: label_b, deck: deck_b, player: player_b },
        &shared.registry,
        best_of,
        seed,
        &mut before_game,
        &mut |outcome: &GameOutcome| {
            games += 1;
            if let Err(e) = write_game_log(shared.config.out_dir.as_deref(), round, a, b, games, outcome) {
                let line = format!("WARN {e}");
                mtg_player::game_log::write(file!(), line!(), &line, "");
                shared.reply(&line);
            }
            shared.lobby().game_finished(round, a, b, outcome);
            shared.notify();
        },
    );
    shared.set_in_game(a, None);
    shared.set_in_game(b, None);
    {
        let mut lobby = shared.lobby();
        lobby.set_game_pages(a, None);
        lobby.set_game_pages(b, None);
    }
    let next = shared.lobby().match_finished(round, a, b, result);
    shared.notify();
    next
}

/// Before a game of a hosted match: an AI seat sideboards and chooses
/// through its model (from game 2; the coin flip's AI winner plays game
/// 1), a person on the page or the client under the sideboard timer, and
/// the match waits for both. A person's game page says where to go.
#[allow(clippy::too_many_arguments)]
fn between_games(
    shared: &Arc<Shared>, round: usize, a: usize, b: usize, best_of: usize, kinds: &[SeatKind; 2],
    clients: &mut [Option<DraftLlmClient>; 2], bg: &BeforeGame,
) -> GameSetup {
    let mut decided: [Option<(Option<mtg_draft::deckbuilding::DraftDeck>, Option<bool>)>; 2] = [None, None];
    for (i, seat) in [a, b].into_iter().enumerate() {
        let SeatKind::Ai(model) = &kinds[i] else { continue };
        if bg.game == 1 || shared.lobby().is_auto(seat) {
            decided[i] = Some((None, None));
            continue;
        }
        let (current, pool, table) = {
            let lobby = shared.lobby();
            (lobby.match_deck(round, a, b, seat).unwrap_or_default(), lobby.pool(seat),
             Table { seat, pod_size: lobby.pod_size(), pack_size: lobby.pack_size() })
        };
        let client = clients[i].get_or_insert_with(|| DraftLlmClient::new(
            model, &shared.set_name, shared.config.guide.as_deref(), &shared.card_reference, table));
        let (you, them) = if i == 0 { bg.wins } else { (bg.wins.1, bg.wins.0) };
        let situation = BetweenGames {
            game: bg.game, best_of, your_wins: you, their_wins: them,
            last: bg.games.last().map(|g| last_game_line(g, seat, bg.games.len())),
            chooses: bg.chooser == seat, opponent_showed: &bg.shown[1 - i],
        };
        let decision = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || sideboard_with_llm(client, &pool, &current, &shared.card_lines, &situation)));
        decided[i] = Some(match decision {
            Ok(d) => {
                crate::deck::log_sideboard(seat, bg.game, &current, &d, situation.chooses);
                (d.deck, d.play_first)
            }
            // A seat whose backend gave up keeps its deck and plays; its
            // games go on, and the game loop deals with a dead seat.
            Err(_) => (None, None),
        });
    }
    shared.lobby().open_between(round, a, b, bg.game, bg.chooser, bg.shown.clone(), decided);
    // The game page is where a person is looking: it says where to go. A
    // seat with nothing to do before game 1 (it lost the coin flip) is not
    // sent anywhere.
    for page in shared.in_game.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter()
        .filter(|p| p.seat == a || p.seat == b)
    {
        let kind = if page.seat == a { &kinds[0] } else { &kinds[1] };
        let last_winner = bg.games.last().and_then(|g| g.winner);
        if let Some(line) = between_line(bg.game, bg.chooser == page.seat, last_winner, page.seat, kind) {
            page.page.set_match_line(Some(line));
        }
    }
    shared.notify();
    loop {
        if shared.quitting() {
            return GameSetup::default();
        }
        let outcome = shared.lobby().take_between(round, a, b);
        if let Some(out) = outcome {
            shared.notify();
            let [da, db] = out.decks;
            let list = |d: mtg_draft::deckbuilding::DraftDeck| mtg_engine::engine::Decklist {
                entries: mtg_draft::deckbuilding::to_decklist(&d),
            };
            return GameSetup { deck_a: da.map(list), deck_b: db.map(list), chooser_plays: out.chooser_plays };
        }
        let lobby = shared.lobby();
        drop(shared.wait(lobby, Duration::from_millis(250)));
    }
}

/// What a seat's game page says before game `game` of a match, or nothing
/// for a seat with nothing to do (it lost the coin flip). The host's own
/// seat has no draft page: it answers at the server's terminal; a person
/// may be on the page or the terminal client, both where they drafted
/// (#879's review).
fn between_line(game: usize, chooses: bool, last_winner: Option<usize>, seat: usize, kind: &SeatKind) -> Option<String> {
    let what = match (game, chooses) {
        (1, false) => return None,
        (1, true) => "you won the coin flip: choose play or draw".to_string(),
        (_, true) => format!("{}: sideboard and choose play or draw", last_game_why(last_winner, seat, game - 1)),
        (_, false) => "sideboard".to_string(),
    };
    let place = if *kind == SeatKind::Cli { "at the server's terminal" } else { "where you drafted" };
    Some(format!("Before game {game}: {what} {place}"))
}

/// Why a seat chooses to play or draw: it lost the last game, or that game
/// was a draw and the choice stays with whoever had it (MTR 2.2).
fn last_game_why(winner: Option<usize>, seat: usize, game: usize) -> String {
    match winner {
        Some(w) if w != seat => format!("you lost game {game}"),
        _ => format!("game {game} was a draw"),
    }
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
/// `name` is the game's (`Seat{n}`, as the runner names seats); `label` is
/// how the host's terminal names the person (#816, #850).
fn game_page(shared: &Shared, name: &str, label: &str) -> Result<GuiPlayer, String> {
    let (lo, hi) = shared.config.game_ports;
    let mut next = shared.next_game_port.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut last = String::from("no ports to try");
    // Through the lobby's port, the page itself on loopback: a friend
    // across the internet needs one port forwarded, not a range of them.
    let bind = if shared.config.direct_game_ports { shared.config.bind.as_str() } else { "127.0.0.1" };
    // The page answers only under a key of its own, which only this seat's
    // link carries: the ports are handed out in order, and without it
    // anyone reaching the lobby could read the person's hand and answer
    // for them (#872).
    let key = crate::lobby::new_key();
    for port in game_port_order(lo, hi, *next) {
        match GuiPlayer::new_keyed(name, bind, Some(port), Some(&key)) {
            Ok(mut gui) => {
                gui.url = if shared.config.direct_game_ports {
                    format!("http://{}:{port}/{key}/", shared.config.advertise)
                } else {
                    format!("http://{}:{}/game/{port}/{key}/", shared.config.advertise, shared.config.port)
                };
                gui.notice_name = label.to_string();
                *next = port.checked_add(1);
                return Ok(gui);
            }
            Err(e) => last = e,
        }
    }
    Err(format!("no free port in {lo}-{hi} ({last})"))
}

/// Every port in `lo..=hi` once, from `from` (when it is in the range)
/// round to the one before it.
fn game_port_order(lo: u16, hi: u16, from: Option<u16>) -> impl Iterator<Item = u16> {
    let start = from.filter(|p| (lo..=hi).contains(p)).unwrap_or(lo);
    (start..=hi).chain(lo..start)
}

/// A finished game's log under `games/`, which the host is told is there.
///
/// # Errors
/// It could not be written, and where: both errors were dropped, and the
/// game's log was missing without a word (#891's review).
fn write_game_log(out_dir: Option<&Path>, round: usize, a: usize, b: usize, game: usize, outcome: &GameOutcome) -> Result<(), String> {
    let Some(dir) = out_dir else { return Ok(()) };
    let dir = dir.join("games");
    let path = dir.join(format!("r{round}-{a}v{b}-g{game}.log"));
    let mut text = outcome.game_log.join("\n");
    text.push('\n');
    std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, text))
        .map_err(|e| format!("could not write {}: {e}", path.display()))
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
    if let Some(rest) = path.strip_prefix("/game/") {
        let (port, tail) = rest.split_once('/').map_or((rest, None), |(p, t)| (p, Some(t)));
        match (port.parse::<u16>(), tail) {
            (Ok(port), Some(tail)) => proxy_game_page(stream, shared, port, &format!("/{tail}")),
            // The page's own links are relative: `/game/8803` would put
            // its socket at `/game/ws`.
            (Ok(port), None) => {
                drain_head(&mut stream);
                redirect(&mut stream, &format!("/game/{port}/"));
            }
            (Err(_), _) => {
                drain_head(&mut stream);
                http_response(&mut stream, "404 Not Found", "text/plain", b"not found");
            }
        }
    } else if lower.contains("upgrade: websocket") {
        serve_websocket(stream, shared, &path);
    } else {
        serve_http(&mut stream, shared, &path);
    }
}

/// The request head off the socket, and whatever followed it in the same
/// reads.
fn drain_head(stream: &mut TcpStream) -> Vec<u8> {
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
    buf
}

fn redirect(stream: &mut TcpStream, to: &str) {
    let _ = stream.write_all(format!(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes());
    let _ = stream.flush();
}

/// `/game/<port>/...`: a match's game page through the lobby's own port.
/// The page itself refuses anything not under its key (`/game/<port>/<key>/`,
/// #872). Only a port one of this table's game pages is served on is reached —
/// the lobby is not a way into anything else on the machine — and the
/// request goes on with the prefix taken off, its bytes then passed both
/// ways as they come, so the page's socket works through it as its fetches
/// do.
fn proxy_game_page(mut client: TcpStream, shared: &Shared, port: u16, rest: &str) {
    let head = drain_head(&mut client);
    let addr = shared.in_game.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter().map(|g| g.addr).find(|a| a.port() == port);
    let Some(mut addr) = addr else {
        http_response(&mut client, "404 Not Found", "text/plain",
            b"no game is being played on this link now: the match is over, or has not started; the draft page has the link to your game");
        return;
    };
    if addr.ip().is_unspecified() {
        addr.set_ip(std::net::Ipv4Addr::LOCALHOST.into());
    }
    let Ok(mut upstream) = TcpStream::connect_timeout(&addr, Duration::from_secs(5)) else {
        http_response(&mut client, "502 Bad Gateway", "text/plain", b"the game page did not answer");
        return;
    };
    // The request line with the prefix off, the rest as it came.
    let line_end = head.windows(2).position(|w| w == b"\r\n").unwrap_or(head.len());
    let line = String::from_utf8_lossy(&head[..line_end]).to_string();
    let mut words = line.split_whitespace();
    let (method, _, version) = (words.next().unwrap_or("GET"), words.next(), words.next().unwrap_or("HTTP/1.1"));
    let mut forward = format!("{method} {rest} {version}").into_bytes();
    forward.extend_from_slice(&head[line_end..]);
    if upstream.write_all(&forward).is_err() {
        return;
    }
    let _ = client.set_read_timeout(None);
    let (Ok(mut from_up), Ok(mut to_client)) = (upstream.try_clone(), client.try_clone()) else { return };
    let back = thread::Builder::new().name("game-proxy".into()).spawn(move || {
        let _ = std::io::copy(&mut from_up, &mut to_client);
        let _ = to_client.shutdown(std::net::Shutdown::Both);
    });
    let _ = std::io::copy(&mut client, &mut upstream);
    let _ = upstream.shutdown(std::net::Shutdown::Both);
    if let Ok(back) = back {
        let _ = back.join();
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
    drain_head(stream);
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
        while let Ok(msg) = rx.try_recv() {
            if ws.send(tungstenite::Message::Text(msg.into())).is_err() {
                return;
            }
        }
        if shared.quitting() {
            // What was queued went out above — the "closed" message among
            // it — and the socket is closed with a close frame, answered
            // before the process goes (#824).
            let _ = ws.close(None);
            let _ = ws.flush();
            let until = Instant::now() + Duration::from_millis(500);
            while Instant::now() < until {
                match ws.read() {
                    Ok(_) => {}
                    Err(e) if mtg_player::gui::nothing_to_read(&e) => {}
                    Err(_) => break,
                }
            }
            return;
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
                    Ok(Request::Sideboard { main, lands }) => shared.lobby().sideboard(seat, &main, &lands),
                    Ok(Request::PlayDraw { play }) => shared.lobby().play_draw(seat, play),
                    Ok(Request::SideboardReady) => shared.lobby().sideboard_ready(seat),
                    Ok(Request::Review) => {
                        let json = serde_json::to_string(&shared.lobby().review(seat)).unwrap_or_default();
                        shared.send_to(id, &json);
                        continue;
                    }
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
            Err(e) if mtg_player::gui::nothing_to_read(&e) => {}
            Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game log that cannot be written is said, not dropped.
    #[test]
    fn a_game_log_that_cannot_be_written_says_where() {
        let dir = std::env::temp_dir().join(format!("mtg-game-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let outcome = GameOutcome { winner: Some(0), turns: 3, game_log: vec!["a line".into()], stalled_seat: None, abandoned: false };
        write_game_log(Some(&dir), 1, 0, 1, 1, &outcome).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("games/r1-0v1-g1.log")).unwrap(), "a line\n");
        // `games` is a file: neither the directory nor the log can be made.
        std::fs::remove_dir_all(dir.join("games")).unwrap();
        std::fs::write(dir.join("games"), "").unwrap();
        let e = write_game_log(Some(&dir), 1, 0, 1, 2, &outcome).unwrap_err();
        assert!(e.contains("r1-0v1-g2.log"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// #879: the line names where to go for the seat it is sent to; the
    /// game page's test draws the longest of these whole.
    #[test]
    fn the_between_games_line_says_where_to_go() {
        let h = SeatKind::Human;
        assert_eq!(between_line(1, false, None, 0, &h), None);
        assert_eq!(between_line(1, true, None, 0, &SeatKind::Cli).as_deref(),
            Some("Before game 1: you won the coin flip: choose play or draw at the server's terminal"));
        assert_eq!(between_line(3, true, None, 0, &h).as_deref(),
            Some("Before game 3: game 2 was a draw: sideboard and choose play or draw where you drafted"));
        assert_eq!(between_line(2, true, Some(1), 0, &h).as_deref(),
            Some("Before game 2: you lost game 1: sideboard and choose play or draw where you drafted"));
        assert_eq!(between_line(2, false, Some(1), 1, &h).as_deref(), Some("Before game 2: sideboard where you drafted"));
    }

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
    fn game_ports_are_tried_from_past_the_last_one_round_the_range() {
        // #853: a finished match's port is let go; the next match starts
        // past it, so a page still open on it does not reconnect into
        // somebody else's game while the range has another port.
        let order = |from| game_port_order(8801, 8804, from).collect::<Vec<_>>();
        assert_eq!(order(None), [8801, 8802, 8803, 8804]);
        assert_eq!(order(Some(8803)), [8803, 8804, 8801, 8802]);
        assert_eq!(order(Some(8805)), [8801, 8802, 8803, 8804], "past the top it wraps");
        assert_eq!(game_port_order(8801, 8801, Some(8802)).collect::<Vec<_>>(), [8801]);
    }

    #[test]
    fn the_join_line_carries_both_ways_in() {
        let line = join_lines("192.168.1.20", 8800, 1, "k3");
        assert!(line.starts_with("seat 1  http://192.168.1.20:8800/?seat=1&key=k3"));
        assert!(line.contains("mtg-draft-client ws://192.168.1.20:8800/ws?seat=1&key=k3"));
    }
}

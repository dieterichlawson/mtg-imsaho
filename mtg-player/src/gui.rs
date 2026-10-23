//! The GUI seat: a browser page on a local port.
//!
//! The fourth surface, next to `cli.rs`, `llm.rs` and `random.rs`. It is
//! deliberately the thinnest of them: the page is sent the engine's own
//! `GameView` and `LegalActions` as JSON, exactly as the types are, and it
//! answers with one `Action` as JSON. There is no prompt text and no
//! response schema of its own to fall out of step with the engine — the
//! failure mode #398 and #404 were — so a new prompt kind is a message the
//! page has not seen yet, and the page's one rule is that an unknown prompt
//! is rendered as a generic list, never as nothing.
//!
//! One process serves one seat on one port. Two humans on one machine is
//! two seats on two ports; a page that reconnects is sent whatever it
//! missed — the decision still pending, or the latest board.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::engine::LegalActions;
use mtg_engine::ids::PlayerId;
use mtg_engine::view::GameView;
use serde::{Deserialize, Serialize};

use crate::Player;

/// The environment variable naming the directory the page is served from.
pub const WEB_DIR_ENV: &str = "MTG_GUI_DIR";
/// Where the page lives relative to the working directory, like `data/`
/// and `decks/`.
pub const DEFAULT_WEB_DIR: &str = "mtg-gui";

/// How often a seat blocked on an answer looks up to see whether anybody is
/// still at the page. Long enough to be free, short enough that an operator
/// who has just closed a tab is told where to go back to (#602).
const BROWSER_CHECK: Duration = Duration::from_secs(2);
/// The port tried first when the seat spec names none.
pub const DEFAULT_PORT: u16 = 8765;

/// A message from the seat to the page.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Outbound<'a> {
    /// Your decision: answer with an `Action`.
    Decision {
        seq: u64,
        seat: PlayerId,
        view: &'a GameView,
        legal: &'a LegalActions,
        combat: Option<&'a CombatPrompt>,
    },
    /// The board as it stands while somebody else decides.
    View { seat: PlayerId, view: &'a GameView },
    /// An answer was refused; the decision it was for stands.
    Notice { seq: u64, text: String },
    /// Decision `seq` has been answered. Every connection holds the same
    /// decision, and only one of them answers it; without this the others
    /// kept the answered prompt live and clickable, so a person at a second
    /// tab could make a real decision — a mulligan — that was dropped as
    /// stale and read on screen as accepted (issue #516).
    Answered { seq: u64 },
    /// The seat's settings, which belong to the seat and not to a page.
    /// "Stop at every priority" used to be per page while the auto-answer
    /// it governs is also per page, so a second tab passed the priorities
    /// the first was deliberately holding (issue #515).
    Settings { stop_at_pass: bool, auto_pass_since_turn: Option<u32> },
    /// The game is over.
    GameOver { seat: PlayerId, view: &'a GameView, summary: String },
    /// What a hosted table tells the page about the match around the game:
    /// which game this is and the score (#855), and whether the opponent's
    /// own page is open — "Opponent is thinking" over a closed tab was the
    /// only thing a waiting person was told (#822). Either is `None` when
    /// there is nothing to say.
    Table { match_line: Option<String>, opponent: Option<String> },
    /// The seat's host has ended the table: stop reconnecting and say why.
    /// A deliberate stop used to look like a dropped network, a page
    /// retrying for ever (#824).
    Closed { reason: String },
}

/// A message from the page to the seat.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Inbound {
    /// A page (re)connected and wants the current state.
    Hello,
    /// The answer to decision `seq`.
    Action { seq: u64, action: serde_json::Value },
    /// A page changed a seat setting; it applies to the seat, so it is
    /// recorded here and broadcast to every other page (issue #515).
    Settings { stop_at_pass: bool, auto_pass_since_turn: Option<u32> },
}

/// What a connection thread hands the seat: an answer to a decision.
struct Answer {
    seq: u64,
    action: serde_json::Value,
    /// The connection it came from, which is the only page a refusal is
    /// about (#647).
    from: u64,
}

/// State shared between the seat and its connection threads.
struct Shared {
    /// What a page connecting now needs first: the pending decision, else
    /// the latest board.
    latest: Mutex<Option<String>>,
    /// One outbound channel per connected page, each with the id its
    /// connection thread removes on the way out. Reaping them here rather
    /// than in `broadcast` is what lets the seat notice a page leaving while
    /// it is blocked waiting for an answer and broadcasting nothing (#602).
    clients: Mutex<Vec<(u64, mpsc::Sender<String>)>>,
    next_client: std::sync::atomic::AtomicU64,
    answers: mpsc::Sender<Answer>,
    web_dir: PathBuf,
    /// The seat's settings, shared by every page attached to it.
    settings: Mutex<(bool, Option<u32>)>,
    /// Set when the seat is dropped: the accept loop stops and the port
    /// is let go (#853).
    closed: std::sync::atomic::AtomicBool,
    /// The table's lines for the page (`Outbound::Table`), kept for a page
    /// that connects after they were said.
    table: Mutex<(Option<String>, Option<String>)>,
    /// Set when the host ends the table: every page is told why and closed.
    closing: Mutex<Option<String>>,
    /// When set, the page is served only under `/<key>/`: a hosted table's
    /// game link carries it, and a request without it reaches nothing —
    /// neither the person's hand nor their decisions (#872).
    path_key: Option<String>,
}

impl Shared {
    fn broadcast(&self, msg: &str) {
        let mut clients = self.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clients.retain(|(_, c)| c.send(msg.to_string()).is_ok());
    }

    /// Send to one page only. A gone page is reaped by its own connection
    /// thread, so a failed send here needs no cleanup.
    fn send_to(&self, id: u64, msg: &str) {
        let clients = self.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, c)) = clients.iter().find(|(cid, _)| *cid == id) {
            let _ = c.send(msg.to_string());
        }
    }

    fn connected(&self) -> usize {
        self.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner).len()
    }

    fn table_message(&self) -> Option<String> {
        let (match_line, opponent) = self.table.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        if match_line.is_none() && opponent.is_none() {
            return None;
        }
        serde_json::to_string(&Outbound::Table { match_line, opponent }).ok()
    }

    fn closing(&self) -> Option<String> {
        self.closing.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    /// Register a page's outbound channel; the id is how it deregisters.
    fn add_client(&self, tx: mpsc::Sender<String>) -> u64 {
        let id = self.next_client.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push((id, tx));
        id
    }

    /// This page is gone, however it went.
    fn drop_client(&self, id: u64) {
        self.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(cid, _)| *cid != id);
    }
}

/// The line to print about the browsers attached, or `None` when there is
/// nothing new to say.
///
/// `said` is "the operator has already been told nobody is here", and the
/// point of this function is that it is a latch *per disappearance* and not
/// for the life of the seat. It used to be the latter, set the first time a
/// decision came up — which is before anyone has had a chance to open the
/// page, every single run — so when the browser later closed, the one line
/// that says where to reconnect was never printed again and the seat waited
/// for ever in total silence. A game left that way is indistinguishable
/// from a hung process (#602, and #566 by another route).
fn browser_notice(name: &str, url: &str, connected: usize, said: &mut bool) -> Option<String> {
    if connected > 0 {
        // Somebody is here; the next time they all leave, say so again.
        *said = false;
        return None;
    }
    if *said {
        return None;
    }
    *said = true;
    Some(format!("{name}: no browser at {url} — open it to answer"))
}

/// What a hosted table holds of a seat's page while the seat is in a game:
/// whether anybody is at it, and a way to tell it about the match around
/// the game or that the table is over. Cloned out of the [`GuiPlayer`]
/// that the game loop owns.
#[derive(Clone)]
pub struct PageHandle(Arc<Shared>);

impl PageHandle {
    /// Pages attached to the seat right now.
    #[must_use]
    pub fn connected(&self) -> usize {
        self.0.connected()
    }

    /// Say where the match stands ("Game 2 of 3 · you 1, them 0"), or
    /// nothing.
    pub fn set_match_line(&self, line: Option<String>) {
        self.set(|t| t.0 = line);
    }

    /// Say what the opponent is doing when it is not "thinking" — its page
    /// is closed — or nothing.
    pub fn set_opponent(&self, line: Option<String>) {
        self.set(|t| t.1 = line);
    }

    fn set(&self, change: impl FnOnce(&mut (Option<String>, Option<String>))) {
        {
            let mut table = self.0.table.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let before = table.clone();
            change(&mut table);
            if *table == before {
                return;
            }
        }
        let msg = self.0.table_message().or_else(|| {
            serde_json::to_string(&Outbound::Table { match_line: None, opponent: None }).ok()
        });
        if let Some(msg) = msg {
            self.0.broadcast(&msg);
        }
    }

    /// The table is over: every page attached is told why and closed, and
    /// a page that connects from now on is told the same.
    pub fn close(&self, reason: &str) {
        let mut closing = self.0.closing.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if closing.is_none() {
            *closing = Some(reason.to_string());
        }
    }
}

/// Takes a page's seat out of its game from another thread: a hosted
/// table's host kicking a person mid-match (#821). Once withdrawn, the
/// seat answers its pending and every later decision with a forfeit and
/// reports why through `Player::gave_up`, so the game loop forfeits it the
/// game and the match, as it does an LLM seat that stopped answering.
#[derive(Clone, Default)]
pub struct Withdraw(Arc<Mutex<Option<String>>>);

impl Withdraw {
    /// Withdraw the seat, saying why. The first reason given stands.
    pub fn withdraw(&self, why: &str) {
        let mut slot = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some(why.to_string());
        }
    }

    fn why(&self) -> Option<String> {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
}

/// Dropping the seat stops serving its page and lets the port go, so a
/// hosted table's next match can bind it: the accept loop ran for the life
/// of the process and every finished match kept its port (#853). A page
/// still attached stays attached, showing how the game ended; it is not
/// sent anything more.
impl Drop for GuiPlayer {
    fn drop(&mut self) {
        self.shared.closed.store(true, std::sync::atomic::Ordering::SeqCst);
        // `incoming()` blocks until a connection arrives: make one. Bound
        // on every address, the loopback reaches it.
        let mut wake = self.bound;
        if wake.ip().is_unspecified() {
            wake.set_ip(match wake {
                std::net::SocketAddr::V4(_) => std::net::Ipv4Addr::LOCALHOST.into(),
                std::net::SocketAddr::V6(_) => std::net::Ipv6Addr::LOCALHOST.into(),
            });
        }
        let woke = TcpStream::connect_timeout(&wake, Duration::from_secs(1)).is_ok();
        // Joined only when the wake landed, so a drop never hangs; the
        // listener is closed when the thread returns.
        if let Some(accept) = self.accept.take().filter(|_| woke) {
            let _ = accept.join();
        }
    }
}

/// A seat played through a browser page.
pub struct GuiPlayer {
    name: String,
    shared: Arc<Shared>,
    answers: mpsc::Receiver<Answer>,
    seq: u64,
    /// The address the page is served at.
    pub url: String,
    /// How the operator's terminal names the seat in "no browser at ...":
    /// the seat's name unless the caller says otherwise, as a hosted table
    /// does with "Lawson (seat 0)" (#850).
    pub notice_name: String,
    said_waiting: bool,
    withdrawn: Withdraw,
    /// Set when the game is being ended for the OTHER seat's reason (its
    /// kick, #854): this seat stops waiting on its page and answers with
    /// the harness's stop, which the game loop replaces with the other
    /// seat's forfeit. Not a loss of this seat's.
    woken: Withdraw,
    /// The listening address, and the thread accepting on it, so that
    /// dropping the seat can stop it.
    bound: std::net::SocketAddr,
    accept: Option<thread::JoinHandle<()>>,
}

impl GuiPlayer {
    /// Start serving the page on the loopback address. `port` of `None`
    /// takes the first free port from [`DEFAULT_PORT`] upward.
    ///
    /// # Errors
    /// The page directory is missing, or no port could be bound.
    pub fn new(name: &str, port: Option<u16>) -> Result<Self, String> {
        Self::new_at(name, "127.0.0.1", port)
    }

    /// [`GuiPlayer::new`] on `bind` — `0.0.0.0` for a page a friend on the
    /// network opens, which is what a hosted draft's games need. The
    /// `url` is `http://<bind>:<port>/` as bound; a caller that advertises
    /// another host name substitutes it.
    ///
    /// # Errors
    /// The page directory is missing, the address is not one this machine
    /// has, or no port could be bound.
    pub fn new_at(name: &str, bind: &str, port: Option<u16>) -> Result<Self, String> {
        Self::new_keyed(name, bind, port, None)
    }

    /// [`GuiPlayer::new_at`], with the page and its socket served only
    /// under `/<key>/` when `key` is given; the `url` carries it. Anything
    /// else on the port is a 404, so the link is the access control.
    ///
    /// # Errors
    /// As [`GuiPlayer::new_at`].
    pub fn new_keyed(name: &str, bind: &str, port: Option<u16>, key: Option<&str>) -> Result<Self, String> {
        let web_dir = std::env::var(WEB_DIR_ENV)
            .map_or_else(|_| PathBuf::from(DEFAULT_WEB_DIR), PathBuf::from);
        if !web_dir.join("index.html").is_file() {
            return Err(format!(
                "the GUI page is not at '{}' (no index.html there); run from the repository \
                 root or set {WEB_DIR_ENV} to the mtg-gui directory",
                web_dir.display()));
        }
        let listener = match port {
            Some(p) => TcpListener::bind((bind, p))
                .map_err(|e| format!("cannot listen on {bind}:{p}: {e}"))?,
            // The next free port from the default upward, so two seats in
            // one game (`--p1 gui --p2 gui`) land on 8765 and 8766 and a
            // second runner on the next pair, rather than somewhere random.
            None => (DEFAULT_PORT..DEFAULT_PORT + 20)
                .find_map(|p| TcpListener::bind((bind, p)).ok())
                .ok_or_else(|| format!("no free port in {bind}:{DEFAULT_PORT}-{}", DEFAULT_PORT + 19))?,
        };
        let bound = listener.local_addr().map_err(|e| e.to_string())?;
        let (answer_tx, answer_rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            latest: Mutex::new(None),
            clients: Mutex::new(Vec::new()),
            next_client: std::sync::atomic::AtomicU64::new(0),
            answers: answer_tx,
            web_dir,
            settings: Mutex::new((false, None)),
            closed: std::sync::atomic::AtomicBool::new(false),
            table: Mutex::new((None, None)),
            closing: Mutex::new(None),
            path_key: key.map(str::to_string),
        });
        let accept_shared = Arc::clone(&shared);
        let accept = thread::Builder::new().name("gui-accept".into()).spawn(move || {
            for stream in listener.incoming() {
                // Looked at on a failed accept too: one that keeps failing
                // (out of file descriptors) never takes the wake-up
                // connection off the backlog, and the drop would wait on it.
                if accept_shared.closed.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let shared = Arc::clone(&accept_shared);
                let _ = thread::Builder::new().name("gui-conn".into())
                    .spawn(move || serve_connection(stream, &shared));
            }
        }).map_err(|e| e.to_string())?;
        Ok(Self {
            name: name.to_string(),
            shared,
            answers: answer_rx,
            seq: 0,
            url: match key {
                Some(k) => format!("http://{bound}/{k}/"),
                None => format!("http://{bound}/"),
            },
            notice_name: name.to_string(),
            said_waiting: false,
            withdrawn: Withdraw::default(),
            woken: Withdraw::default(),
            bound,
            accept: Some(accept),
        })
    }

    /// The address the page is served on.
    #[must_use]
    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.bound
    }

    /// A handle on this seat's page for a hosted table; see [`PageHandle`].
    #[must_use]
    pub fn page_handle(&self) -> PageHandle {
        PageHandle(Arc::clone(&self.shared))
    }

    /// A handle that withdraws this seat from its game; see [`Withdraw`].
    #[must_use]
    pub fn withdraw_handle(&self) -> Withdraw {
        self.withdrawn.clone()
    }

    /// A handle that stops this seat waiting on its page because the
    /// OTHER seat was withdrawn: a pending decision, and any later one, is
    /// answered with [`crate::watchdog::ceiling_move`] for the game loop to
    /// replace with that seat's forfeit (#854). This seat does not give up.
    #[must_use]
    pub fn wake_handle(&self) -> Withdraw {
        self.woken.clone()
    }

    /// Show the page the board while another seat decides.
    pub fn observe(&mut self, view: &GameView) {
        let Ok(msg) = serde_json::to_string(&Outbound::View { seat: view.you, view }) else { return };
        *self.shared.latest.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(msg.clone());
        self.shared.broadcast(&msg);
    }

    /// Tell the page the game is over, and give it a moment to hear it.
    pub fn game_over(&mut self, view: &GameView, summary: &str) {
        let Ok(msg) = serde_json::to_string(&Outbound::GameOver {
            seat: view.you, view, summary: summary.to_string(),
        }) else { return };
        *self.shared.latest.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(msg.clone());
        self.shared.broadcast(&msg);
        thread::sleep(Duration::from_millis(300));
    }

    /// Ask the page and wait for its answer.
    fn ask(&mut self, view: &GameView, legal: &LegalActions, combat: Option<&CombatPrompt>) -> Action {
        if self.withdrawn.why().is_some() {
            return crate::watchdog::forfeit_move();
        }
        if self.woken.why().is_some() {
            return crate::watchdog::ceiling_move();
        }
        self.seq += 1;
        let seq = self.seq;
        let msg = serde_json::to_string(&Outbound::Decision {
            seq, seat: view.you, view, legal, combat,
        }).expect("a decision serializes");
        *self.shared.latest.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(msg.clone());
        self.shared.broadcast(&msg);
        if let Some(line) = browser_notice(&self.notice_name, &self.url, self.shared.connected(), &mut self.said_waiting) {
            crate::stderr_line!("{line}");
        }
        loop {
            // Timed, so the wait is not a black hole: a page can close while
            // the seat is blocked here, and the only way to tell the operator
            // where to reconnect is to look again on a beat. `Shared` owns
            // the sending half and this seat owns the `Arc` for its whole
            // life, so the channel cannot be disconnected and both error
            // arms mean the same thing — nobody has answered yet. (The
            // `AbandonGame` this used to return on `Err` was unreachable
            // for that reason, and the comment above it described a state
            // that cannot occur.)
            let answer = self.answers.recv_timeout(BROWSER_CHECK);
            // Withdrawn while the page held the decision: the seat is no
            // longer the person's, and an answer from the page is not taken.
            if self.withdrawn.why().is_some() {
                return crate::watchdog::forfeit_move();
            }
            if self.woken.why().is_some() {
                return crate::watchdog::ceiling_move();
            }
            let answer = match answer {
                Ok(answer) => answer,
                Err(_) => {
                    if let Some(line) = browser_notice(
                        &self.notice_name, &self.url, self.shared.connected(), &mut self.said_waiting)
                    {
                        crate::stderr_line!("{line}");
                    }
                    continue;
                }
            };
            if let Some(action) = self.take(seq, answer) {
                return action;
            }
        }
    }

    /// Judge one answer against decision `seq`: the action it carries, or
    /// `None` with the page that sent it told why.
    fn take(&self, seq: u64, answer: Answer) -> Option<Action> {
        if answer.seq != seq {
            // An answer to an earlier decision (issue #71's rule: a
            // stale keystroke never lands on a new prompt).
            return None;
        }
        match serde_json::from_value::<Action>(answer.action) {
            Ok(Action::AbandonGame) => {
                self.notice(answer.from, seq, "AbandonGame is the harness's, not a player's");
                None
            }
            // The same: the harness ends a stalled seat's game (#742); a
            // page that wants out has Concede.
            Ok(Action::Forfeit) => {
                self.notice(answer.from, seq, "Forfeit is the harness's, not a player's");
                None
            }
            Ok(action) => {
                // Every other page is holding this same decision. Tell
                // them it is taken, before the next board arrives, so
                // none of them leaves a live prompt over a game that has
                // moved on (issue #516).
                if let Ok(msg) = serde_json::to_string(&Outbound::Answered { seq }) {
                    self.shared.broadcast(&msg);
                }
                Some(action)
            }
            Err(e) => {
                self.notice(answer.from, seq, &format!("not an action: {e}"));
                None
            }
        }
    }

    /// A refusal goes to the page that sent the refused answer and no
    /// other: every other tab sent nothing, and broadcasting it put an
    /// error about somebody else's answer over their prompt (#647).
    fn notice(&self, to: u64, seq: u64, text: &str) {
        let msg = serde_json::to_string(&Outbound::Notice { seq, text: text.to_string() })
            .expect("a notice serializes");
        self.shared.send_to(to, &msg);
    }

    /// Declare attackers or blockers.
    pub fn choose_combat(&mut self, view: &GameView, legal: &LegalActions, prompt: &CombatPrompt) -> Action {
        // A combat prompt with one legal answer is never put to the page.
        // The seat is deliberately the thinnest of the four and sends the
        // engine's types as they are — but a prompt with nothing eligible is
        // not a thin rendering of a question, it is a screen reading "CLICK
        // CREATURES TO ATTACK WITH" over a board with nothing to click. One
        // rule, shared with the other three seats (issue #517).
        if let Some(forced) = crate::forced_combat_answer(prompt) {
            return forced;
        }
        self.ask(view, legal, Some(prompt))
    }
}

impl Player for GuiPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, view: &GameView, legal: &LegalActions) -> Action {
        self.ask(view, legal, None)
    }

    fn gave_up(&self) -> Option<String> {
        self.withdrawn.why()
    }
}

// ------------------------------------------------------------ connections

/// One TCP connection: a page fetch, or a WebSocket for the game.
fn serve_connection(mut stream: TcpStream, shared: &Shared) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let Some(head) = peek_head(&stream) else { return };
    let path = head.lines().next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");
    let path = path.split('?').next().unwrap_or("/");
    let path = match keyed_path(path, shared.path_key.as_deref()) {
        Keyed::Page(p) => p.to_string(),
        Keyed::Directory(to) => {
            read_head(&mut stream);
            respond(&mut stream, "301 Moved Permanently", &format!("Location: {to}\r\n"), "text/plain", b"");
            return;
        }
        Keyed::Refused => {
            read_head(&mut stream);
            respond(&mut stream, "404 Not Found", "", "text/plain", b"not found");
            return;
        }
    };
    if head.to_ascii_lowercase().contains("upgrade: websocket") {
        serve_websocket(stream, shared);
    } else {
        serve_http(&mut stream, &shared.web_dir, &path);
    }
}

/// The request head, left on the socket for whoever reads it next: up to
/// its blank line, or what arrived in five seconds, or the first 2 KiB.
///
/// One peek returns whatever has arrived, and a client may send the head
/// in pieces (a `write!` to a `TcpStream` sends `GET `, the path and the
/// rest separately): a path read from `GET ` alone was `/`, which a keyed
/// page refuses, so its own link was a 404 now and then.
#[must_use]
pub fn peek_head(stream: &TcpStream) -> Option<String> {
    let mut head = [0u8; 2048];
    let until = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let n = stream.peek(&mut head).ok()?;
        let done = n == 0 || n == head.len() || head[..n].windows(4).any(|w| w == b"\r\n\r\n");
        if done || std::time::Instant::now() >= until {
            return Some(String::from_utf8_lossy(&head[..n]).into_owned());
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// What a request path reaches on a page served under a key.
#[derive(Debug, PartialEq, Eq)]
enum Keyed<'a> {
    /// The path under the key, with the key taken off.
    Page(&'a str),
    /// `/<key>` with no slash: sent on to `<key>/`, where the page's
    /// relative links work.
    Directory(String),
    /// No key, or the wrong one.
    Refused,
}

fn keyed_path<'a>(path: &'a str, key: Option<&str>) -> Keyed<'a> {
    let Some(key) = key else { return Keyed::Page(path) };
    match path.strip_prefix('/').and_then(|p| p.strip_prefix(key)) {
        Some("") => Keyed::Directory(format!("{key}/")),
        Some(rest) if rest.starts_with('/') => Keyed::Page(rest),
        _ => Keyed::Refused,
    }
}

/// The request head off the socket.
fn read_head(stream: &mut TcpStream) {
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
}

fn respond(stream: &mut TcpStream, status: &str, extra: &str, ctype: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {status}\r\n{extra}Content-Type: {ctype}\r\nContent-Length: {}\r\n\
         Cache-Control: no-cache\r\nConnection: close\r\n\r\n", body.len());
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

/// Serve one static file and close. Only files under the page directory,
/// only by a path with no `..` in it.
fn serve_http(stream: &mut TcpStream, web_dir: &Path, path: &str) {
    read_head(stream);
    let rel = if path == "/" { "index.html" } else { path.trim_start_matches('/') };
    let safe = !rel.split('/').any(|seg| seg == ".." || seg.is_empty());
    let file = web_dir.join(rel);
    let body = if safe { std::fs::read(&file).ok() } else { None };
    match body {
        Some(b) => respond(stream, "200 OK", "", content_type(rel), &b),
        None => respond(stream, "404 Not Found", "", "text/plain", b"not found"),
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

/// One page's WebSocket: forward its answers in, and everything the seat
/// broadcasts out, until it closes.
fn serve_websocket(stream: TcpStream, shared: &Shared) {
    let Ok(mut ws) = tungstenite::accept(stream) else { return };
    // Polled: the one thread both reads the page and writes to it, so
    // neither side can starve the other.
    let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(50)));
    let (tx, rx) = mpsc::channel::<String>();
    let id = shared.add_client(tx);
    pump_websocket(&mut ws, &rx, shared, id);
    // However this page went — closed, errored, or read to EOF — it is no
    // longer attached. Saying so here rather than leaving it to the next
    // `broadcast` is what makes `connected()` true while the seat sits in
    // `ask` broadcasting nothing (#602).
    shared.drop_client(id);
}

/// Forward one page's answers in and everything the seat broadcasts out,
/// until it closes.
fn pump_websocket(
    ws: &mut tungstenite::WebSocket<TcpStream>,
    rx: &mpsc::Receiver<String>,
    shared: &Shared,
    id: u64,
) {
    loop {
        // Everything the seat has broadcast since last time.
        while let Ok(msg) = rx.try_recv() {
            if ws.send(tungstenite::Message::Text(msg.into())).is_err() {
                return;
            }
        }
        if let Some(reason) = shared.closing() {
            close_page(ws, &reason);
            return;
        }
        match ws.read() {
            Ok(tungstenite::Message::Text(text)) => {
                match serde_json::from_str::<Inbound>(&text) {
                    Ok(Inbound::Hello) => {
                        // The seat's settings first: a page that joins a
                        // seat which is stopping at every priority must not
                        // spend its first decision auto-passing (#515).
                        let (stop, since) = *shared.settings.lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        if let Ok(msg) = serde_json::to_string(&Outbound::Settings {
                            stop_at_pass: stop, auto_pass_since_turn: since })
                        {
                            if ws.send(tungstenite::Message::Text(msg.into())).is_err() {
                                return;
                            }
                        }
                        let latest = shared.latest.lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner).clone();
                        if let Some(msg) = latest {
                            if ws.send(tungstenite::Message::Text(msg.into())).is_err() {
                                return;
                            }
                        }
                        // And what the table has said about the match, which
                        // a page that has just come back would otherwise not
                        // hear until it changes (#855).
                        if let Some(msg) = shared.table_message() {
                            if ws.send(tungstenite::Message::Text(msg.into())).is_err() {
                                return;
                            }
                        }
                    }
                    Ok(Inbound::Settings { stop_at_pass, auto_pass_since_turn }) => {
                        *shared.settings.lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) =
                            (stop_at_pass, auto_pass_since_turn);
                        if let Ok(msg) = serde_json::to_string(&Outbound::Settings {
                            stop_at_pass, auto_pass_since_turn })
                        {
                            shared.broadcast(&msg);
                        }
                    }
                    Ok(Inbound::Action { seq, action }) => {
                        if shared.answers.send(Answer { seq, action, from: id }).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        let msg = unreadable_notice(&text, &e);
                        if ws.send(tungstenite::Message::Text(msg.into())).is_err() {
                            return;
                        }
                    }
                }
            }
            Ok(tungstenite::Message::Close(_)) => return,
            Ok(_) => {}
            Err(e) if nothing_to_read(&e) => {}
            Err(_) => return,
        }
    }
}

/// A socket read that found nothing yet, rather than a socket that ended:
/// the read timeout, and a read cut short by a signal. A process stopped
/// and resumed (Ctrl-Z, then `fg`) gets `EINTR` from a read with a timeout
/// set even with no handler installed, and that ended every terminal seat
/// (#880).
#[must_use]
pub fn nothing_to_read(e: &tungstenite::Error) -> bool {
    matches!(e, tungstenite::Error::Io(e) if matches!(e.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted))
}

/// Tell a page the table is over, close its socket with a close frame, and
/// give it a moment to answer: a socket the process simply drops is a reset
/// to the page, which reads it as a network to reconnect to for ever (#824).
fn close_page(ws: &mut tungstenite::WebSocket<TcpStream>, reason: &str) {
    if let Ok(msg) = serde_json::to_string(&Outbound::Closed { reason: reason.to_string() }) {
        let _ = ws.send(tungstenite::Message::Text(msg.into()));
    }
    let _ = ws.close(None);
    let _ = ws.flush();
    let until = std::time::Instant::now() + Duration::from_millis(500);
    while std::time::Instant::now() < until {
        match ws.read() {
            Ok(_) => {}
            Err(e) if nothing_to_read(&e) => {}
            Err(_) => return,
        }
    }
}

/// The notice for a message from the page that is not one the seat reads.
///
/// It carries the `seq` the message names whenever that can be read. The
/// page clears its decision when it sends and restores it only on a notice
/// for the `seq` it sent, so a fixed `seq: 0` — which no decision has —
/// left the page with no prompt and the seat still waiting for an answer
/// (#648). Only a message with no readable `seq` falls back to 0.
fn unreadable_notice(text: &str, e: &serde_json::Error) -> String {
    let seq = serde_json::from_str::<serde_json::Value>(text).ok()
        .and_then(|v| v.get("seq").and_then(serde_json::Value::as_u64))
        .unwrap_or(0);
    serde_json::to_string(&Outbound::Notice { seq, text: format!("unreadable message: {e}") })
        .expect("a notice serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #880: a read interrupted by a stop and continue is not the end of
    /// the socket; nor is a timeout; a reset is.
    #[test]
    fn an_interrupted_read_is_nothing_to_read_yet() {
        use std::io::{Error, ErrorKind};
        for kind in [ErrorKind::Interrupted, ErrorKind::WouldBlock, ErrorKind::TimedOut] {
            assert!(nothing_to_read(&tungstenite::Error::Io(Error::from(kind))), "{kind:?}");
        }
        assert!(!nothing_to_read(&tungstenite::Error::Io(Error::from(ErrorKind::ConnectionReset))));
        assert!(!nothing_to_read(&tungstenite::Error::ConnectionClosed));
    }

    /// A head sent in pieces is read whole before its path is: a keyed
    /// page's own link answered 404 when the peek saw only `GET `.
    #[test]
    fn a_request_head_sent_in_pieces_is_peeked_whole() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = thread::spawn(move || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            for piece in ["GET ", "/k3y/", " HTTP/1.1\r\nHost: x\r\n", "\r\n"] {
                s.write_all(piece.as_bytes()).unwrap();
                s.flush().unwrap();
                thread::sleep(Duration::from_millis(50));
            }
            s
        });
        let (stream, _) = listener.accept().unwrap();
        let head = peek_head(&stream).expect("a head");
        assert_eq!(head, "GET /k3y/ HTTP/1.1\r\nHost: x\r\n\r\n");
        let mut left = [0u8; 4];
        assert_eq!(stream.peek(&mut left).unwrap(), 4, "the head is still on the socket");
        drop(client.join());
    }

    /// #872: under a key, the page and its socket are reached only through
    /// it; a bare port, another path or a wrong key reaches nothing.
    #[test]
    fn a_keyed_page_is_reached_only_through_its_key() {
        assert_eq!(keyed_path("/", None), Keyed::Page("/"));
        assert_eq!(keyed_path("/ws", None), Keyed::Page("/ws"));
        let k = Some("k3y");
        assert_eq!(keyed_path("/k3y/", k), Keyed::Page("/"));
        assert_eq!(keyed_path("/k3y/ws", k), Keyed::Page("/ws"));
        assert_eq!(keyed_path("/k3y/dist/main.js", k), Keyed::Page("/dist/main.js"));
        assert_eq!(keyed_path("/k3y", k), Keyed::Directory("k3y/".into()));
        for refused in ["/", "/ws", "/dist/main.js", "/k3", "/k3y2/", "/K3Y/", "/x/k3y/"] {
            assert_eq!(keyed_path(refused, k), Keyed::Refused, "{refused}");
        }
    }

    /// #602: the seat says where its page is every time the last browser
    /// goes away, not once for the life of the run.
    ///
    /// The old latch was set at the first decision, which happens before
    /// anybody has had a chance to open the page. So the line was always
    /// spent on the start of the game, and the one moment it matters — a
    /// browser closing mid-game, with the seat holding a decision — was
    /// silent.
    #[test]
    fn the_seat_names_its_url_once_per_disappearance_not_once_per_run() {
        let url = "http://127.0.0.1:8765/";
        let mut said = false;
        let first = browser_notice("P1", url, 0, &mut said);
        assert!(first.is_some_and(|l| l.contains(url)), "the first decision names the URL");
        assert!(browser_notice("P1", url, 0, &mut said).is_none(),
            "and does not repeat it on every beat of the wait");

        // Somebody is at the page: nothing to say.
        assert!(browser_notice("P1", url, 1, &mut said).is_none());

        // And they close it. This is the line that never came.
        let again = browser_notice("P1", url, 0, &mut said);
        assert!(again.is_some_and(|l| l.contains(url)),
            "the browser went away and the seat said where to go back to");
        assert!(browser_notice("P1", url, 0, &mut said).is_none(), "once per disappearance");
    }

    /// A page that has gone stops being counted straight away, without
    /// anything being broadcast.
    ///
    /// `broadcast`'s `retain` was the only thing that reaped a dead client,
    /// and a seat blocked in `ask` broadcasts nothing — so `connected()` went
    /// on counting a closed page for as long as the seat was waiting for it,
    /// which is exactly when the count is read (#602).
    #[test]
    fn a_page_that_has_gone_is_not_counted_and_is_not_broadcast_to() {
        let (answers, _answers_rx) = mpsc::channel();
        let shared = Shared {
            latest: Mutex::new(None),
            clients: Mutex::new(Vec::new()),
            next_client: std::sync::atomic::AtomicU64::new(0),
            answers,
            web_dir: PathBuf::from("."),
            settings: Mutex::new((false, None)),
            closed: std::sync::atomic::AtomicBool::new(false),
            table: Mutex::new((None, None)),
            closing: Mutex::new(None),
            path_key: None,
        };
        let (a_tx, a_rx) = mpsc::channel();
        let (b_tx, b_rx) = mpsc::channel();
        let a = shared.add_client(a_tx);
        let _b = shared.add_client(b_tx);
        assert_eq!(shared.connected(), 2);

        shared.drop_client(a);
        assert_eq!(shared.connected(), 1,
            "the page that left is gone before anything is broadcast");
        shared.broadcast("board");
        assert!(a_rx.try_recv().is_err(), "and is not sent to");
        assert_eq!(b_rx.try_recv().ok().as_deref(), Some("board"),
            "while the one still attached is");
    }

    fn test_seat() -> GuiPlayer {
        let (answers, answers_rx) = mpsc::channel();
        GuiPlayer {
            name: "P1".into(),
            shared: Arc::new(Shared {
                latest: Mutex::new(None),
                clients: Mutex::new(Vec::new()),
                next_client: std::sync::atomic::AtomicU64::new(0),
                answers,
                web_dir: PathBuf::from("."),
                settings: Mutex::new((false, None)),
                closed: std::sync::atomic::AtomicBool::new(false),
                table: Mutex::new((None, None)),
                closing: Mutex::new(None),
                path_key: None,
            }),
            answers: answers_rx,
            seq: 1,
            url: String::new(),
            notice_name: "P1".into(),
            said_waiting: false,
            withdrawn: Withdraw::default(),
            woken: Withdraw::default(),
            bound: ([127, 0, 0, 1], 0).into(),
            accept: None,
        }
    }

    /// #821: a seat withdrawn while its page holds a decision answers it
    /// with a forfeit and says why from then on; nobody was at the page,
    /// so it used to wait for ever, and a hosted table's kick never reached
    /// a running match.
    #[test]
    fn a_withdrawn_seat_stops_waiting_and_gives_up() {
        use mtg_engine::cards::CardRegistry;
        use mtg_engine::engine::{legal_actions, setup_game, Decklist, GameConfig};
        use mtg_engine::ids::PlayerId;
        let registry = CardRegistry::with_all_cards();
        let deck = Decklist { entries: vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)] };
        let state = setup_game(&GameConfig {
            player_names: vec!["you".into(), "opp".into()],
            decklists: vec![deck.clone(), deck],
            starting_life: 20,
            starting_player: Some(PlayerId(0)),
            rng_seed: Some(7),
        }, &registry);
        let view = GameView::for_player(&state, PlayerId(0), &registry);
        let legal = legal_actions(&state, &registry);

        let mut seat = test_seat();
        assert!(Player::gave_up(&seat).is_none());
        let handle = seat.withdraw_handle();
        let kicker = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            handle.withdraw("the host kicked seat 0");
        });
        let began = std::time::Instant::now();
        let answer = seat.choose_action(&view, &legal);
        kicker.join().unwrap();
        assert!(matches!(answer, Action::Forfeit), "{answer:?}");
        assert!(began.elapsed() < BROWSER_CHECK * 2, "it stopped waiting within a beat");
        assert_eq!(Player::gave_up(&seat).as_deref(), Some("the host kicked seat 0"));
        // Every later decision is the same, without asking the page.
        assert!(matches!(seat.choose_action(&view, &legal), Action::Forfeit));
    }

    /// #854: a seat woken because its OPPONENT was kicked stops waiting on
    /// its page within a beat, with the harness's stop for the game loop
    /// to replace — and has not given up itself.
    #[test]
    fn a_woken_seat_stops_waiting_without_giving_up() {
        use mtg_engine::cards::CardRegistry;
        use mtg_engine::engine::{legal_actions, setup_game, Decklist, GameConfig};
        use mtg_engine::ids::PlayerId;
        let registry = CardRegistry::with_all_cards();
        let deck = Decklist { entries: vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)] };
        let state = setup_game(&GameConfig {
            player_names: vec!["you".into(), "opp".into()],
            decklists: vec![deck.clone(), deck],
            starting_life: 20,
            starting_player: Some(PlayerId(0)),
            rng_seed: Some(7),
        }, &registry);
        let view = GameView::for_player(&state, PlayerId(0), &registry);
        let legal = legal_actions(&state, &registry);

        let mut seat = test_seat();
        let handle = seat.wake_handle();
        let kicker = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            handle.withdraw("the host kicked seat 1");
        });
        let began = std::time::Instant::now();
        let answer = seat.choose_action(&view, &legal);
        kicker.join().unwrap();
        assert!(matches!(answer, Action::AbandonGame), "{answer:?}");
        assert!(began.elapsed() < BROWSER_CHECK * 2, "it stopped waiting within a beat");
        assert!(Player::gave_up(&seat).is_none(), "the opponent's kick is no loss of this seat's");
    }

    /// #822, #855, #824: what a hosted table tells a seat's page — the
    /// match around the game, the opponent's closed page, the end of the
    /// table — goes to every page attached, is kept for a page that comes
    /// back, and is said once per change.
    #[test]
    fn the_tables_lines_reach_every_page_and_wait_for_the_next() {
        let seat = test_seat();
        let page = seat.page_handle();
        let (a_tx, a_rx) = mpsc::channel();
        let _a = seat.shared.add_client(a_tx);
        assert_eq!(page.connected(), 1);
        assert!(seat.shared.table_message().is_none(), "nothing said yet, nothing kept");

        page.set_match_line(Some("Game 2 of a best-of-3 · you 1, them 0".into()));
        let msg: serde_json::Value = serde_json::from_str(&a_rx.try_recv().unwrap()).unwrap();
        assert_eq!(msg, serde_json::json!({"type": "table",
            "match_line": "Game 2 of a best-of-3 · you 1, them 0", "opponent": null}));
        page.set_match_line(Some("Game 2 of a best-of-3 · you 1, them 0".into()));
        assert!(a_rx.try_recv().is_err(), "the same line again is no message");

        page.set_opponent(Some("Bea's game page is closed".into()));
        let msg: serde_json::Value = serde_json::from_str(&a_rx.try_recv().unwrap()).unwrap();
        assert_eq!(msg["opponent"], "Bea's game page is closed");
        // A page connecting now is sent both, after the board (Hello).
        let kept: serde_json::Value = serde_json::from_str(&seat.shared.table_message().unwrap()).unwrap();
        assert_eq!(kept["match_line"], "Game 2 of a best-of-3 · you 1, them 0");
        assert_eq!(kept["opponent"], "Bea's game page is closed");

        assert!(seat.shared.closing().is_none());
        page.close("the host quit");
        page.close("a second reason");
        assert_eq!(seat.shared.closing().as_deref(), Some("the host quit"), "the first reason stands");
    }

    /// #647: a refused answer is told to the page that sent it, and to no
    /// other tab on the seat — those sent nothing, and are still holding
    /// the decision.
    #[test]
    fn a_refusal_reaches_only_the_page_that_sent_the_answer() {
        let seat = test_seat();
        let (a_tx, a_rx) = mpsc::channel();
        let (b_tx, b_rx) = mpsc::channel();
        let a = seat.shared.add_client(a_tx);
        let _b = seat.shared.add_client(b_tx);

        for action in [serde_json::json!({"Nope": 1}), serde_json::json!("AbandonGame"),
                       serde_json::json!("Forfeit")] {
            assert!(seat.take(1, Answer { seq: 1, action, from: a }).is_none());
            let to_a = a_rx.try_recv().expect("the sender is told its answer was refused");
            assert!(to_a.contains("\"notice\"") && to_a.contains("\"seq\":1"), "{to_a}");
            assert!(b_rx.try_recv().is_err(), "a tab that sent nothing is told nothing");
        }

        // An accepted answer is still everybody's business (#516).
        let taken = seat.take(1, Answer { seq: 1, action: serde_json::json!("PassPriority"), from: a });
        assert!(matches!(taken, Some(Action::PassPriority)));
        assert!(a_rx.try_recv().unwrap().contains("answered"));
        assert!(b_rx.try_recv().unwrap().contains("answered"));
    }

    /// #648: a message the seat cannot read is answered with a notice for
    /// the decision it names, so the page that sent it gets its prompt
    /// back; `seq: 0` matched no decision and left the page empty.
    #[test]
    fn an_unreadable_answer_is_refused_against_the_decision_it_names() {
        for (text, seq) in [
            (r#"{"type":"action","seq":3}"#, 3),
            (r#"{"type":"nonsense","seq":7}"#, 7),
            (r#"not json"#, 0),
        ] {
            let e = serde_json::from_str::<Inbound>(text).err().expect("unreadable");
            let notice: serde_json::Value = serde_json::from_str(&unreadable_notice(text, &e)).unwrap();
            assert_eq!(notice["type"], "notice");
            assert_eq!(notice["seq"], seq, "{text}");
            assert!(notice["text"].as_str().unwrap().starts_with("unreadable message: "));
        }
    }
}

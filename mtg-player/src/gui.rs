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

/// A seat played through a browser page.
pub struct GuiPlayer {
    name: String,
    shared: Arc<Shared>,
    answers: mpsc::Receiver<Answer>,
    seq: u64,
    /// The address the page is served at.
    pub url: String,
    said_waiting: bool,
}

impl GuiPlayer {
    /// Start serving the page. `port` of `None` takes the first free port
    /// from [`DEFAULT_PORT`] upward.
    ///
    /// # Errors
    /// The page directory is missing, or no port could be bound.
    pub fn new(name: &str, port: Option<u16>) -> Result<Self, String> {
        let web_dir = std::env::var(WEB_DIR_ENV)
            .map_or_else(|_| PathBuf::from(DEFAULT_WEB_DIR), PathBuf::from);
        if !web_dir.join("index.html").is_file() {
            return Err(format!(
                "the GUI page is not at '{}' (no index.html there); run from the repository \
                 root or set {WEB_DIR_ENV} to the mtg-gui directory",
                web_dir.display()));
        }
        let listener = match port {
            Some(p) => TcpListener::bind(("127.0.0.1", p))
                .map_err(|e| format!("cannot listen on 127.0.0.1:{p}: {e}"))?,
            // The next free port from the default upward, so two seats in
            // one game (`--p1 gui --p2 gui`) land on 8765 and 8766 and a
            // second runner on the next pair, rather than somewhere random.
            None => (DEFAULT_PORT..DEFAULT_PORT + 20)
                .find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok())
                .ok_or_else(|| format!("no free port in 127.0.0.1:{DEFAULT_PORT}-{}", DEFAULT_PORT + 19))?,
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
        });
        let accept_shared = Arc::clone(&shared);
        thread::Builder::new().name("gui-accept".into()).spawn(move || {
            for stream in listener.incoming().flatten() {
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
            url: format!("http://{bound}/"),
            said_waiting: false,
        })
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
        self.seq += 1;
        let seq = self.seq;
        let msg = serde_json::to_string(&Outbound::Decision {
            seq, seat: view.you, view, legal, combat,
        }).expect("a decision serializes");
        *self.shared.latest.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(msg.clone());
        self.shared.broadcast(&msg);
        if let Some(line) = browser_notice(&self.name, &self.url, self.shared.connected(), &mut self.said_waiting) {
            eprintln!("{line}");
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
            let answer = match self.answers.recv_timeout(BROWSER_CHECK) {
                Ok(answer) => answer,
                Err(_) => {
                    if let Some(line) = browser_notice(
                        &self.name, &self.url, self.shared.connected(), &mut self.said_waiting)
                    {
                        eprintln!("{line}");
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
}

// ------------------------------------------------------------ connections

/// One TCP connection: a page fetch, or a WebSocket for the game.
fn serve_connection(mut stream: TcpStream, shared: &Shared) {
    let mut head = [0u8; 2048];
    // Give the request head a moment to arrive, then decide what this is.
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let Ok(n) = stream.peek(&mut head) else { return };
    let head = String::from_utf8_lossy(&head[..n]).to_ascii_lowercase();
    if head.contains("upgrade: websocket") {
        serve_websocket(stream, shared);
    } else {
        serve_http(&mut stream, &shared.web_dir);
    }
}

/// Serve one static file and close. Only files under the page directory,
/// only by a path with no `..` in it.
fn serve_http(stream: &mut TcpStream, web_dir: &Path) {
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
    let request = String::from_utf8_lossy(&buf);
    let path = request.lines().next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");
    let path = path.split('?').next().unwrap_or("/");
    let rel = if path == "/" { "index.html" } else { path.trim_start_matches('/') };
    let safe = !rel.split('/').any(|seg| seg == ".." || seg.is_empty());
    let file = web_dir.join(rel);
    let body = if safe { std::fs::read(&file).ok() } else { None };
    let (status, ctype, body) = match body {
        Some(b) => ("200 OK", content_type(rel), b),
        None => ("404 Not Found", "text/plain", b"not found".to_vec()),
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n\
         Cache-Control: no-cache\r\nConnection: close\r\n\r\n", body.len());
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
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
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
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
            }),
            answers: answers_rx,
            seq: 1,
            url: String::new(),
            said_waiting: false,
        }
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

        for action in [serde_json::json!({"Nope": 1}), serde_json::json!("AbandonGame")] {
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

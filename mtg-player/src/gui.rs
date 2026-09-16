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
}

/// What a connection thread hands the seat: an answer to a decision.
struct Answer {
    seq: u64,
    action: serde_json::Value,
}

/// State shared between the seat and its connection threads.
struct Shared {
    /// What a page connecting now needs first: the pending decision, else
    /// the latest board.
    latest: Mutex<Option<String>>,
    /// One outbound channel per connected page.
    clients: Mutex<Vec<mpsc::Sender<String>>>,
    answers: mpsc::Sender<Answer>,
    web_dir: PathBuf,
}

impl Shared {
    fn broadcast(&self, msg: &str) {
        let mut clients = self.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clients.retain(|c| c.send(msg.to_string()).is_ok());
    }

    fn connected(&self) -> usize {
        self.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner).len()
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
    said_waiting: bool,
}

impl GuiPlayer {
    /// Start serving the page. `port` of `None` tries [`DEFAULT_PORT`] and
    /// then any free port.
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
            None => TcpListener::bind(("127.0.0.1", DEFAULT_PORT))
                .or_else(|_| TcpListener::bind(("127.0.0.1", 0)))
                .map_err(|e| format!("cannot listen on 127.0.0.1: {e}"))?,
        };
        let bound = listener.local_addr().map_err(|e| e.to_string())?;
        let (answer_tx, answer_rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            latest: Mutex::new(None),
            clients: Mutex::new(Vec::new()),
            answers: answer_tx,
            web_dir,
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
        if self.shared.connected() == 0 && !self.said_waiting {
            eprintln!("{}: waiting for a browser at {}", self.name, self.url);
            self.said_waiting = true;
        }
        loop {
            let Ok(answer) = self.answers.recv() else {
                // Every connection thread is gone and the listener with
                // them; nothing will ever answer. Leave the game the way
                // the harness does, not the way a player does.
                return Action::AbandonGame;
            };
            if answer.seq != seq {
                // An answer to an earlier decision (issue #71's rule: a
                // stale keystroke never lands on a new prompt).
                continue;
            }
            match serde_json::from_value::<Action>(answer.action) {
                Ok(Action::AbandonGame) => {
                    self.notice(seq, "AbandonGame is the harness's, not a player's");
                }
                Ok(action) => return action,
                Err(e) => self.notice(seq, &format!("not an action: {e}")),
            }
        }
    }

    fn notice(&self, seq: u64, text: &str) {
        let msg = serde_json::to_string(&Outbound::Notice { seq, text: text.to_string() })
            .expect("a notice serializes");
        self.shared.broadcast(&msg);
    }

    /// Declare attackers or blockers.
    pub fn choose_combat(&mut self, view: &GameView, legal: &LegalActions, prompt: &CombatPrompt) -> Action {
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
    shared.clients.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(tx);
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
                        let latest = shared.latest.lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner).clone();
                        if let Some(msg) = latest {
                            if ws.send(tungstenite::Message::Text(msg.into())).is_err() {
                                return;
                            }
                        }
                    }
                    Ok(Inbound::Action { seq, action }) => {
                        if shared.answers.send(Answer { seq, action }).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        let msg = serde_json::to_string(&Outbound::Notice {
                            seq: 0, text: format!("unreadable message: {e}"),
                        }).expect("a notice serializes");
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

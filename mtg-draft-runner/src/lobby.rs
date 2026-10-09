//! The hosted table: the state machine behind `mtg-draft-server`.
//!
//! Everything here is synchronous and owns no socket, thread or timer. The
//! server (`server.rs`) wraps one `Lobby` in a mutex, drives it from the
//! connection threads, the AI workers, the timer and the tournament
//! thread, and sends every seat its whole [`View`] after each change. That
//! split is what makes the phases testable in-process: a test walks a
//! `Lobby` from the lobby to the standings without a port.
//!
//! What a seat is shown is decided here and nowhere else: a view holds the
//! viewing seat's own pack, pool, picks and deck, and only public facts
//! about everybody else (who is here, what they are doing, how many picks
//! they have made, the pairings and the standings). Another seat's pack or
//! pool never enters a view, as in a real draft.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use mtg_draft::deckbuilding::{self, DraftDeck};
use mtg_draft::pack::BoosterPack;
use mtg_draft::set_data::SetData;
use mtg_draft::table::{Direction, Table, TablePick};
use mtg_draft::tournament::{GameOutcome, MatchResult, Tournament, TournamentConfig, BYE};
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::Color;

use crate::card_lines::CardLines;
use crate::deck::DeckBuildResult;
use crate::draft_log::DraftLogger;
use crate::standings::RowTags;
use crate::{log_bye, log_deck_building, log_draft_pick, log_draft_warning, log_game_log, log_header,
    log_match_result, log_pack_contents, log_pool_summary, log_section, log_standings, log_subsection};

// ───────────────────────────────────────────────────────────── seat specs

/// Who sits at a seat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatKind {
    /// A person, joining through the page or the terminal client.
    Human,
    /// An LLM seat, drafting and playing under this model spec.
    Ai(String),
    /// The host's own seat, at the server's terminal.
    Cli,
}

impl SeatKind {
    /// The word the view and the log use.
    #[must_use]
    pub fn word(&self) -> &'static str {
        match self {
            SeatKind::Human => "human",
            SeatKind::Ai(_) => "ai",
            SeatKind::Cli => "cli",
        }
    }
}

/// The smallest and largest pod `--seats` may name, as the runner's
/// `--players` has them.
pub const MIN_SEATS: usize = 2;
pub const MAX_SEATS: usize = 8;

/// Read `--seats`: a comma list of `human`, `ai`, `ai:<model spec>` and
/// `cli`, each optionally `Nx`-prefixed (`1xhuman,7xai`), in seat order.
///
/// # Errors
/// An empty list, an unknown word, a bad count, more than one `cli`, or a
/// pod outside `MIN_SEATS..=MAX_SEATS`.
pub fn parse_seats(spec: &str, default_ai: &str) -> Result<Vec<SeatKind>, String> {
    let mut seats = Vec::new();
    for entry in spec.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        let (count, word) = match entry.split_once('x') {
            Some((n, rest)) if n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty() => {
                let n: usize = n.parse().map_err(|_| format!("'{entry}': bad count"))?;
                if n == 0 {
                    return Err(format!("'{entry}': a count of 0 seats nobody"));
                }
                (n, rest)
            }
            _ => (1, entry),
        };
        let kind = match word {
            "human" => SeatKind::Human,
            "ai" => SeatKind::Ai(default_ai.to_string()),
            "cli" => SeatKind::Cli,
            other => match other.strip_prefix("ai:") {
                Some(model) if !model.is_empty() => SeatKind::Ai(model.to_string()),
                _ => return Err(format!(
                    "'{other}' is not a seat: expected human, ai, ai:<model spec> or cli")),
            },
        };
        for _ in 0..count {
            seats.push(kind.clone());
        }
    }
    if seats.is_empty() {
        return Err("--seats names nobody".to_string());
    }
    if seats.len() < MIN_SEATS || seats.len() > MAX_SEATS {
        return Err(format!(
            "--seats names {} seats; a pod is {MIN_SEATS} to {MAX_SEATS}", seats.len()));
    }
    if seats.iter().filter(|s| **s == SeatKind::Cli).count() > 1 {
        return Err("at most one cli seat: the terminal is one keyboard".to_string());
    }
    Ok(seats)
}

/// A seat's key: 128 random bits as hex, printed once by the host and
/// carried by every join.
#[must_use]
pub fn new_key() -> String {
    format!("{:032x}", rand::random::<u128>())
}

// ───────────────────────────────────────────────────────────── card facts

/// What the page is told about a card: enough to read it without a
/// registry of its own.
#[derive(Debug, Clone, Serialize)]
pub struct CardFacts {
    /// The one-line headline the runner's pack listing uses, rarity
    /// included.
    pub line: String,
    /// Rules text, both faces of a double-faced card.
    pub text: String,
    pub rarity: Option<String>,
    /// `W`, `U`, `B`, `R`, `G`.
    pub colors: Vec<String>,
}

/// Every card of the set as the page is told about it.
pub struct CardBook {
    facts: HashMap<String, CardFacts>,
}

impl CardBook {
    #[must_use]
    pub fn new(set_data: &SetData, registry: &CardRegistry, lines: &CardLines) -> Self {
        let rarities = set_data.rarities();
        let mut book = HashMap::new();
        let basics = deckbuilding::BASIC_LANDS.iter().map(|b| (*b).to_string());
        for name in set_data.all_card_names().into_iter().chain(basics) {
            let faces = mtg_player::llm::card_faces(&name, registry);
            let colors: Vec<String> = faces.first().map(|(_, data)| {
                let mut colors: Vec<Color> = data.color_indicator.clone();
                for symbol in data.cost.iter().flat_map(|c| c.symbols.iter()) {
                    if let mtg_engine::types::ManaSymbol::Colored(c) = symbol {
                        if !colors.contains(c) {
                            colors.push(*c);
                        }
                    }
                }
                colors.iter().map(|c| color_letter(*c).to_string()).collect()
            }).unwrap_or_default();
            let text = faces.iter().enumerate().map(|(i, (face, data))| {
                if i == 0 { data.oracle_text.clone() } else { format!("// {face}: {}", data.oracle_text) }
            }).collect::<Vec<_>>().join("\n");
            let fact = CardFacts {
                line: lines.pack_line(&name),
                text,
                rarity: rarities.get(&name).map(|r| r.label().to_string()),
                colors,
            };
            book.insert(mtg_draft::front_face(&name).to_string(), fact.clone());
            book.insert(name, fact);
        }
        Self { facts: book }
    }

    fn card(&self, name: &str, index: Option<usize>) -> CardView {
        let facts = self.facts.get(name).or_else(|| self.facts.get(mtg_draft::front_face(name)));
        CardView {
            index,
            name: mtg_draft::front_face(name).to_string(),
            line: facts.map_or_else(|| mtg_draft::front_face(name).to_string(), |f| f.line.clone()),
            text: facts.map(|f| f.text.clone()).unwrap_or_default(),
            rarity: facts.and_then(|f| f.rarity.clone()),
            colors: facts.map(|f| f.colors.clone()).unwrap_or_default(),
        }
    }
}

fn color_letter(c: Color) -> &'static str {
    match c {
        Color::White => "W",
        Color::Blue => "U",
        Color::Black => "B",
        Color::Red => "R",
        Color::Green => "G",
    }
}

// ───────────────────────────────────────────────────────────── the view

/// The whole of what one seat is shown, sent after every change.
#[derive(Debug, Clone, Serialize)]
pub struct View {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub phase: Phase,
    pub seat: usize,
    pub pod_size: usize,
    pub set: String,
    pub seats: Vec<SeatView>,
    pub pass_direction: Option<Direction>,
    pub pack: Option<PackView>,
    pub pool: Vec<CardView>,
    pub picks: Vec<PickView>,
    pub deck: Option<DeckView>,
    pub matches: Vec<MatchView>,
    /// Every pairing of the tournament so far, for everybody: who plays
    /// whom this round and how it went. A match's link is in `matches`,
    /// which holds the viewing seat's own.
    pub pairings: Vec<PairingView>,
    pub standings: Vec<StandingView>,
    pub notice: Option<String>,
    /// Seconds a human has for a pick and for the deck, when the host set
    /// timers; the page shows them beside the deadline.
    pub pick_seconds: Option<u64>,
    pub build_seconds: Option<u64>,
    /// Milliseconds left to build the deck, when a build timer is running
    /// for this seat.
    pub build_deadline_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Lobby,
    Drafting,
    Building,
    Playing,
    Done,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeatView {
    pub seat: usize,
    pub kind: &'static str,
    pub name: String,
    pub joined: bool,
    /// Live connections right now; a joined seat with none is away.
    pub connected: usize,
    pub status: &'static str,
    pub picks: usize,
    /// The table picks for this seat: it was kicked, or never joined and
    /// the host started without it.
    pub auto: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackView {
    pub id: usize,
    pub round: usize,
    pub pick: usize,
    pub size: usize,
    pub cards: Vec<CardView>,
    pub waiting: usize,
    pub deadline_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CardView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    pub name: String,
    pub line: String,
    pub text: String,
    pub rarity: Option<String>,
    pub colors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PickView {
    pub round: usize,
    pub pick: usize,
    pub card: String,
    pub auto: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeckView {
    pub main: Vec<String>,
    pub lands: HashMap<String, u32>,
    pub sideboard: Vec<String>,
    pub valid: bool,
    pub problem: Option<String>,
    /// The deck is final: `ready` was sent, or the table built it.
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchView {
    pub round: usize,
    pub opponent: usize,
    pub url: Option<String>,
    pub status: &'static str,
    pub games: Vec<GameView>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameView {
    pub winner: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairingView {
    pub round: usize,
    pub a: usize,
    /// `None` is a bye.
    pub b: Option<usize>,
    pub status: &'static str,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StandingView {
    pub seat: usize,
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
    pub points: usize,
    pub game_wins: usize,
    pub byes: usize,
}

/// A message to a seat that is not a view: its request was refused.
#[derive(Debug, Clone, Serialize)]
pub struct Refused {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub reason: String,
    pub echo: serde_json::Value,
}

/// What a client sends.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Hello,
    Pick { pack_id: usize, index: usize },
    Deck {
        main: Vec<String>,
        #[serde(default)]
        lands: HashMap<String, u32>,
        #[serde(default)]
        sideboard: Vec<String>,
    },
    Ready,
    Name { name: String },
}

// ───────────────────────────────────────────────────────────── the lobby

/// How the table is set up.
pub struct LobbyConfig {
    pub set_code: String,
    pub set_name: String,
    pub seats: Vec<SeatKind>,
    pub best_of: usize,
    pub seed: u64,
    pub guide_path: Option<String>,
    pub pick_seconds: Option<u64>,
    pub build_seconds: Option<u64>,
    /// Where `decks/` goes: the directory beside the log, or nowhere.
    pub out_dir: Option<PathBuf>,
}

struct SeatState {
    kind: SeatKind,
    name: String,
    key: Option<String>,
    joined: bool,
    connected: usize,
    /// The table picks and builds for this seat.
    auto: bool,
    /// The host handed the seat to the table for good; an absent seat
    /// that was only being picked for gets it back when it joins.
    kicked: bool,
    deck: Option<DraftDeck>,
    deck_problem: Option<String>,
    ready: bool,
    /// The deck is the table's, not the seat's.
    deck_fallback: bool,
    pick_deadline: Option<Instant>,
    build_deadline: Option<Instant>,
    notice: Option<String>,
}

impl SeatState {
    fn is_human(&self) -> bool {
        self.kind == SeatKind::Human
    }
}

/// One match of the tournament.
struct MatchState {
    round: usize,
    a: usize,
    b: usize,
    urls: [Option<String>; 2],
    status: MatchStatus,
    games: Vec<Option<usize>>,
    result: Option<MatchResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchStatus {
    Waiting,
    Playing,
    Done,
}

impl MatchStatus {
    fn word(self) -> &'static str {
        match self {
            MatchStatus::Waiting => "waiting",
            MatchStatus::Playing => "playing",
            MatchStatus::Done => "done",
        }
    }
}

/// What an AI worker is to ask its model next.
pub struct PickJob {
    pub pack_id: usize,
    pub round: usize,
    pub pick: usize,
    pub cards: Vec<String>,
    pub pool: Vec<String>,
}

/// The table.
pub struct Lobby {
    config: LobbyConfig,
    seats: Vec<SeatState>,
    table: Table,
    phase: Phase,
    book: CardBook,
    registry: Arc<CardRegistry>,
    log: DraftLogger,
    tournament: Option<Tournament>,
    matches: Vec<MatchState>,
    /// The round in progress: its pairings, byes included.
    round_pairings: Vec<(usize, usize)>,
    /// Lines for the host's terminal, drained by the server.
    events: Vec<String>,
    started_pool_log: bool,
}

impl Lobby {
    /// Seat the pod, deal the packs and write the log's header and packs.
    /// `log` is the opened logger (or one over no file).
    ///
    /// # Errors
    /// The packs do not make a table (see `Table::new`).
    pub fn new(
        config: LobbyConfig,
        packs: &[Vec<BoosterPack>],
        set_data: &SetData,
        registry: Arc<CardRegistry>,
        lines: &CardLines,
        log: DraftLogger,
    ) -> Result<Self, String> {
        if packs.len() != config.seats.len() {
            return Err(format!("{} seats but packs for {}", config.seats.len(), packs.len()));
        }
        let table = Table::new(packs)?;
        let book = CardBook::new(set_data, &registry, lines);
        let seats: Vec<SeatState> = config.seats.iter().enumerate().map(|(i, kind)| SeatState {
            kind: kind.clone(),
            name: format!("seat {i}"),
            key: if *kind == SeatKind::Human { Some(new_key()) } else { None },
            joined: !matches!(kind, SeatKind::Human),
            connected: 0,
            auto: false,
            kicked: false,
            deck: None,
            deck_problem: None,
            ready: false,
            deck_fallback: false,
            pick_deadline: None,
            build_deadline: None,
            notice: None,
        }).collect();

        let models: Vec<String> = config.seats.iter().map(|k| match k {
            SeatKind::Human => "human".to_string(),
            SeatKind::Ai(m) => m.clone(),
            SeatKind::Cli => "cli".to_string(),
        }).collect();
        let guides: Vec<Option<String>> = vec![config.guide_path.clone(); seats.len()];
        log_header!(log, &config.set_name, seats.len(), config.best_of, models.as_slice(),
            guides.as_slice(), config.seed, None, &[]);
        mtg_player::game_log::write(file!(), line!(),
            "NOTE this is a hosted table (mtg-draft-server): picks are asynchronous and the log is in \
the order they happened, not in seat order", "");
        log_section!(log, "BOOSTER PACKS");
        for (seat, seat_packs) in packs.iter().enumerate() {
            for (n, pack) in seat_packs.iter().enumerate() {
                log_pack_contents!(log, seat, n + 1, &pack.all_cards());
            }
        }

        Ok(Self {
            config,
            seats,
            table,
            phase: Phase::Lobby,
            book,
            registry,
            log,
            tournament: None,
            matches: Vec::new(),
            round_pairings: Vec::new(),
            events: Vec::new(),
            started_pool_log: false,
        })
    }

    // ── facts ──

    #[must_use]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    #[must_use]
    pub fn pod_size(&self) -> usize {
        self.seats.len()
    }

    #[must_use]
    pub fn pack_size(&self) -> usize {
        self.table.pack_size()
    }

    #[must_use]
    pub fn best_of(&self) -> usize {
        self.config.best_of
    }

    #[must_use]
    pub fn seed(&self) -> u64 {
        self.config.seed
    }

    #[must_use]
    pub fn kind(&self, seat: usize) -> Option<&SeatKind> {
        self.seats.get(seat).map(|s| &s.kind)
    }

    #[must_use]
    pub fn name(&self, seat: usize) -> &str {
        &self.seats[seat].name
    }

    /// The seats a person can sit at, with their keys.
    #[must_use]
    pub fn human_keys(&self) -> Vec<(usize, String)> {
        self.seats.iter().enumerate()
            .filter_map(|(i, s)| s.key.clone().map(|k| (i, k)))
            .collect()
    }

    /// Whether `key` opens `seat`.
    ///
    /// # Errors
    /// No such seat, not a human seat, or the wrong key — each said plainly.
    pub fn check_key(&self, seat: usize, key: &str) -> Result<(), String> {
        let Some(state) = self.seats.get(seat) else {
            return Err(format!("there is no seat {seat} at this {}-seat table", self.seats.len()));
        };
        match &state.key {
            None => Err(format!("seat {seat} is an {} seat; nobody sits at it", state.kind.word())),
            Some(k) if k == key => Ok(()),
            Some(_) => Err(format!("wrong key for seat {seat}")),
        }
    }

    /// The terminal lines since last asked.
    pub fn take_events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.events)
    }

    fn event(&mut self, line: String) {
        self.events.push(line);
    }

    /// A seat has read its one-off notice.
    pub fn clear_notice(&mut self, seat: usize) {
        self.seats[seat].notice = None;
    }

    /// Whether `seat` has a notice waiting.
    #[must_use]
    pub fn has_notice(&self, seat: usize) -> bool {
        self.seats[seat].notice.is_some()
    }

    // ── the lobby phase ──

    /// A connection for `seat` opened.
    pub fn connected(&mut self, seat: usize) {
        let first = !self.seats[seat].joined;
        self.seats[seat].joined = true;
        self.seats[seat].connected += 1;
        if first {
            self.event(format!("seat {seat} joined"));
        }
        if self.seats[seat].auto && !self.seats[seat].kicked {
            // Back: the table stops picking for it from here.
            self.seats[seat].auto = false;
            self.event(format!("seat {seat} is here; it picks for itself from now on"));
            self.arm_deadlines();
        }
        if self.phase == Phase::Lobby && self.seats.iter().all(|s| s.joined) {
            self.start();
        }
    }

    /// A connection for `seat` closed.
    pub fn disconnected(&mut self, seat: usize) {
        self.seats[seat].connected = self.seats[seat].connected.saturating_sub(1);
    }

    /// Whether every human seat has been seen.
    #[must_use]
    pub fn everyone_joined(&self) -> bool {
        self.seats.iter().all(|s| s.joined)
    }

    /// Start the draft: by the host, or by the last human joining. A seat
    /// that has not joined is auto-picked until it does.
    pub fn start(&mut self) {
        if self.phase != Phase::Lobby {
            return;
        }
        self.phase = Phase::Drafting;
        log_section!(self.log, "DRAFT");
        log_subsection!(self.log, "Pack 1");
        let absent: Vec<usize> = self.seats.iter().enumerate()
            .filter(|(_, s)| !s.joined).map(|(i, _)| i).collect();
        for seat in &absent {
            self.seats[*seat].auto = true;
            self.event(format!("seat {seat} has not joined; the table picks for it until it does"));
        }
        self.event(format!("the draft has started: {} seats, pack 1 passes left", self.seats.len()));
        self.arm_deadlines();
        self.run_auto_seats();
    }

    /// Whether the table picks for this seat at once rather than on the
    /// timer: it was kicked, or the host set no pick timer. An absent seat
    /// under a timer is picked for when the timer runs out, so a person
    /// who joins a minute late finds the table has taken a few picks, not
    /// the whole draft (the first playtest lost all 42 in four seconds).
    fn picks_at_once(&self, seat: usize) -> bool {
        self.seats[seat].auto && (self.seats[seat].kicked || self.config.pick_seconds.is_none())
    }

    /// The same for the deck: kicked, or no build timer.
    fn builds_at_once(&self, seat: usize) -> bool {
        self.seats[seat].auto && (self.seats[seat].kicked || self.config.build_seconds.is_none())
    }

    /// Give every human seat with a pack and no deadline one, when the host
    /// set a pick timer. An absent seat's deadline is the table's cue to
    /// pick for it.
    fn arm_deadlines(&mut self) {
        let Some(secs) = self.config.pick_seconds else { return };
        for seat in 0..self.seats.len() {
            if !self.seats[seat].is_human() || self.seats[seat].kicked {
                continue;
            }
            match self.table.in_front(seat) {
                Some(_) if self.seats[seat].pick_deadline.is_none() => {
                    self.seats[seat].pick_deadline = Some(Instant::now() + Duration::from_secs(secs));
                }
                Some(_) => {}
                None => self.seats[seat].pick_deadline = None,
            }
        }
    }

    // ── drafting ──

    /// Which seats have something to pick from right now.
    #[must_use]
    pub fn seats_with_a_pack(&self) -> Vec<usize> {
        (0..self.seats.len()).filter(|&s| self.table.in_front(s).is_some()).collect()
    }

    /// What an AI seat's worker should ask about, if anything.
    #[must_use]
    pub fn pick_job(&self, seat: usize) -> Option<PickJob> {
        if self.phase != Phase::Drafting || !matches!(self.seats[seat].kind, SeatKind::Ai(_))
            || self.seats[seat].auto
        {
            return None;
        }
        let front = self.table.in_front(seat)?;
        Some(PickJob {
            pack_id: front.id,
            round: front.round,
            pick: front.pick,
            cards: front.cards.to_vec(),
            pool: self.table.pool(seat).to_vec(),
        })
    }

    /// Whether an AI seat's worker should build its deck now.
    #[must_use]
    pub fn deck_job(&self, seat: usize) -> Option<Vec<String>> {
        let s = &self.seats[seat];
        if !matches!(s.kind, SeatKind::Ai(_)) || s.auto || s.ready || !self.table.seat_done(seat) {
            return None;
        }
        Some(self.table.pool(seat).to_vec())
    }

    /// A person's pick.
    ///
    /// # Errors
    /// Wrong phase, no pack, the wrong pack, or an index past the pack.
    pub fn pick(&mut self, seat: usize, pack_id: usize, index: usize) -> Result<(), String> {
        if self.phase != Phase::Drafting {
            return Err(match self.phase {
                Phase::Lobby => "the draft has not started".to_string(),
                _ => "the draft is over".to_string(),
            });
        }
        if self.seats[seat].auto {
            return Err("the table picks for this seat".to_string());
        }
        let front = self.table.in_front(seat).ok_or("no pack in front of you")?;
        if front.id != pack_id {
            return Err(format!("pack {pack_id} is not the pack in front of you (pack {} is)", front.id));
        }
        let card = front.cards.get(index)
            .ok_or_else(|| format!("card {index} is past the end of the pack ({} cards)", front.cards.len()))?
            .clone();
        let (round, pick) = (front.round, front.pick);
        self.apply_pick(seat, pack_id, index, false, "human", "human")?;
        let name = self.seats[seat].name.clone();
        self.event(format!("{name} picked {} ({round}.{pick})", mtg_draft::front_face(&card)));
        // The pack may have gone to a seat the table picks for.
        self.run_auto_seats();
        Ok(())
    }

    /// An AI seat's pick, with the prompt and the answer for the log.
    ///
    /// # Errors
    /// The pack is no longer in front of the seat (it cannot be: nothing
    /// but this worker moves an AI seat's packs), or the index is bad.
    pub fn ai_pick(
        &mut self, seat: usize, pack_id: usize, index: usize, prompt: &str, response: &str,
        substituted: bool,
    ) -> Result<(), String> {
        let (round, pick) = {
            let front = self.table.in_front(seat).ok_or("no pack in front of the seat")?;
            (front.round, front.pick)
        };
        if substituted {
            let card = self.table.in_front(seat)
                .and_then(|f| f.cards.get(index).cloned()).unwrap_or_default();
            log_draft_warning!(self.log, seat, round, pick, &card, response);
            self.event(format!("WARN seat {seat} pack {round} pick {pick}: unusable answer, \
substituted {} (the first card)", mtg_draft::front_face(&card)));
        }
        let card = self.table.in_front(seat).and_then(|f| f.cards.get(index).cloned())
            .ok_or("card index past the pack")?;
        self.apply_pick(seat, pack_id, index, false, prompt, response)?;
        let name = self.seats[seat].name.clone();
        self.event(format!("{name} picked {} ({round}.{pick})", mtg_draft::front_face(&card)));
        self.run_auto_seats();
        Ok(())
    }

    /// The table picks for a seat: the card the fallback deck would play,
    /// else the first.
    ///
    /// # Errors
    /// No pack in front of the seat.
    pub fn auto_pick(&mut self, seat: usize, why: &str) -> Result<(), String> {
        let front = self.table.in_front(seat).ok_or("no pack in front of the seat")?;
        let (pack_id, cards) = (front.id, front.cards.to_vec());
        let pool = self.table.pool(seat).to_vec();
        let index = auto_pick_index(&cards, &pool, &self.registry);
        let card = cards[index].clone();
        let note = format!("auto-pick: {why}");
        self.apply_pick(seat, pack_id, index, true, &note, &note)?;
        // A kicked seat was told once that it is the table's; a notice
        // per pick would only write over that line (the lobby test read
        // "The table picked Moonmist for you" where the kick should be).
        if !self.seats[seat].kicked {
            self.seats[seat].notice = Some(format!(
                "The table picked {} for you ({why}).", mtg_draft::front_face(&card)));
        }
        self.event(format!("seat {seat}: the table picked {} ({why})", mtg_draft::front_face(&card)));
        Ok(())
    }

    fn apply_pick(
        &mut self, seat: usize, pack_id: usize, index: usize, auto: bool, prompt: &str, response: &str,
    ) -> Result<(), String> {
        let available = self.table.in_front(seat).map(|f| f.cards.to_vec()).unwrap_or_default();
        let picked = self.table.pick(seat, pack_id, index, auto)?;
        log_draft_pick!(self.log, seat, picked.round, picked.pick, &available, &picked.card, prompt, response);
        self.seats[seat].pick_deadline = None;
        if picked.round_over && !picked.draft_over {
            let round = self.table.round().unwrap_or(0);
            log_subsection!(self.log, &format!("Pack {round}"));
            self.event(format!("pack {round} is open; it passes {}",
                match self.table.pass_direction() { Some(Direction::Right) => "right", _ => "left" }));
        }
        if self.table.seat_done(seat) {
            self.seat_finished_drafting(seat);
        }
        if picked.draft_over {
            self.draft_over();
        }
        self.arm_deadlines();
        Ok(())
    }

    fn seat_finished_drafting(&mut self, seat: usize) {
        if self.seats[seat].build_deadline.is_some() || self.seats[seat].ready {
            return;
        }
        // The section opens when the first seat is done, not when the
        // table is: a seat builds as soon as its own draft is over, and its
        // pool and deck would otherwise land inside the DRAFT section,
        // before the header a reader looks for (found in the first
        // playtest: three of four decks were above "DECK BUILDING").
        if !self.started_pool_log {
            self.started_pool_log = true;
            log_section!(self.log, "DECK BUILDING");
            mtg_player::game_log::write(file!(), line!(),
                "NOTE each seat's pool and deck are written as that seat finishes drafting, so they come in the order the seats finished, interleaved with the last picks of the others", "");
        }
        log_pool_summary!(self.log, seat, self.table.pool(seat));
        let name = self.seats[seat].name.clone();
        self.event(format!("{name} has drafted all {} cards and is building", self.table.pool(seat).len()));
        if self.seats[seat].is_human() {
            if self.builds_at_once(seat) {
                self.auto_build(seat, "the table builds for a seat it picks for");
            } else if let Some(secs) = self.config.build_seconds {
                self.seats[seat].build_deadline = Some(Instant::now() + Duration::from_secs(secs));
            }
        }
    }

    fn draft_over(&mut self) {
        self.phase = Phase::Building;
        self.event("the draft is over; every seat is building".to_string());
    }

    // ── building ──

    /// A person's deck, re-sendable until `ready`. A deck that is only
    /// short is work in progress: it is recorded with what it still needs
    /// (`deck.problem` in the view) and not refused, because the page and
    /// the client send the deck after every card moved, and a person
    /// building one card at a time was refused twenty-two times in a row
    /// (the first playtest). `ready` is what a short deck is refused at.
    ///
    /// # Errors
    /// The seat is not building yet, is already ready, or the deck names
    /// a card the seat did not draft (or too many copies of one, or a land
    /// that is not basic) — the reason is the one `validate_deck` gives.
    pub fn submit_deck(
        &mut self, seat: usize, main: &[String], lands: &HashMap<String, u32>, sideboard: &[String],
    ) -> Result<(), String> {
        if !self.table.seat_done(seat) {
            return Err("you are still drafting".to_string());
        }
        if self.seats[seat].ready {
            return Err("your deck is final".to_string());
        }
        let pool = self.table.pool(seat).to_vec();
        match deckbuilding::validate_deck(&pool, main, lands) {
            Ok(deck) => {
                self.seats[seat].deck = Some(deck);
                self.seats[seat].deck_problem = None;
                Ok(())
            }
            Err(problem) => {
                // Kept, so the page can show what was sent next to what is
                // wrong with it, whether or not it is refused.
                self.seats[seat].deck = Some(DraftDeck {
                    maindeck: main.to_vec(),
                    lands: lands.clone(),
                    sideboard: sideboard.to_vec(),
                });
                self.seats[seat].deck_problem = Some(problem.clone());
                if deck_in_progress(&problem) { Ok(()) } else { Err(problem) }
            }
        }
    }

    /// A person's deck is final.
    ///
    /// # Errors
    /// No valid deck has been sent.
    pub fn ready(&mut self, seat: usize) -> Result<(), String> {
        if self.seats[seat].ready {
            return Ok(());
        }
        if !self.table.seat_done(seat) {
            return Err("you are still drafting".to_string());
        }
        let deck = match (&self.seats[seat].deck, &self.seats[seat].deck_problem) {
            (Some(deck), None) => deck.clone(),
            (_, Some(problem)) => return Err(format!("your deck is not legal: {problem}")),
            (None, None) => return Err("send a deck first".to_string()),
        };
        self.finish_deck(seat, &deck, &[], 0, false);
        Ok(())
    }

    /// An AI seat's built deck.
    pub fn ai_deck(&mut self, seat: usize, result: &DeckBuildResult) {
        let attempts: Vec<(&str, &str, Option<&str>)> = result.attempts.iter()
            .map(|a| (a.prompt.as_str(), a.response.as_str(), a.error.as_deref()))
            .collect();
        if result.fallback {
            self.event(format!("WARN seat {seat}: no valid deck after {} attempts; the table built one",
                result.retries));
        }
        self.finish_deck(seat, &result.deck.clone(), &attempts, result.retries, result.fallback);
    }

    /// The table builds a seat's deck.
    pub fn auto_build(&mut self, seat: usize, why: &str) {
        if self.seats[seat].ready {
            return;
        }
        let deck = deckbuilding::fallback_deck(self.table.pool(seat), &self.registry);
        mtg_player::game_log::write(file!(), line!(),
            &format!("[Seat {seat}] WARN the table built this seat's deck: {why}"), "");
        self.seats[seat].notice = Some(format!("The table built your deck ({why})."));
        self.event(format!("seat {seat}: the table built its deck ({why})"));
        self.finish_deck(seat, &deck, &[], 0, true);
    }

    fn finish_deck(
        &mut self, seat: usize, deck: &DraftDeck, attempts: &[(&str, &str, Option<&str>)],
        retries: usize, fallback: bool,
    ) {
        log_deck_building!(self.log, seat, &deck.maindeck, &deck.lands, &deck.sideboard, attempts,
            retries, fallback);
        let s = &mut self.seats[seat];
        s.deck = Some(deck.clone());
        s.deck_problem = None;
        s.ready = true;
        s.deck_fallback = fallback;
        s.build_deadline = None;
        let name = s.name.clone();
        self.event(format!("{name} is ready: {} cards ({} spells, {} lands)",
            deck.total_cards(), deck.maindeck.len(), deck.lands.values().sum::<u32>()));
        if let Some(dir) = &self.config.out_dir {
            let path = dir.join("decks").join(format!("seat-{seat}.txt"));
            if let Err(e) = crate::deck::write_deck_file(&path, deck) {
                self.event(format!("WARN could not write {}: {e}", path.display()));
            }
        }
    }

    /// Games the tournament has played so far.
    #[must_use]
    pub fn games_played(&self) -> usize {
        self.matches.iter().map(|m| m.games.len()).sum()
    }

    /// Every deck is in.
    #[must_use]
    pub fn all_ready(&self) -> bool {
        self.seats.iter().all(|s| s.ready)
    }

    /// A built deck, as the engine takes it.
    #[must_use]
    pub fn decklist(&self, seat: usize) -> Option<mtg_engine::engine::Decklist> {
        self.seats[seat].deck.as_ref().map(|d| mtg_engine::engine::Decklist {
            entries: deckbuilding::to_decklist(d),
        })
    }

    // ── absent seats ──

    /// Turn a human seat into one the table picks and builds for, from now
    /// on. Its games are forfeit.
    ///
    /// # Errors
    /// Not a human seat.
    pub fn kick(&mut self, seat: usize) -> Result<(), String> {
        let Some(s) = self.seats.get_mut(seat) else {
            return Err(format!("there is no seat {seat}"));
        };
        if !s.is_human() {
            return Err(format!("seat {seat} is an {} seat", s.kind.word()));
        }
        s.auto = true;
        s.kicked = true;
        s.joined = true;
        s.notice = Some("The host has handed your seat to the table.".to_string());
        self.event(format!("seat {seat} kicked: the table picks and builds for it, and its games are forfeit"));
        if self.phase == Phase::Lobby && self.seats.iter().all(|s| s.joined) {
            self.start();
        }
        self.run_auto_seats();
        Ok(())
    }

    /// An AI seat's backend gave up (its retries are spent): the table
    /// picks and builds for it from here, and its games are forfeit, so
    /// the people at the table are not stopped by it.
    pub fn seat_gave_up(&mut self, seat: usize, why: &str) {
        self.seats[seat].auto = true;
        mtg_player::game_log::write(file!(), line!(),
            &format!("[Seat {seat}] WARN the seat stopped answering; the table picks and builds for it: {why}"), "");
        self.event(format!("WARN seat {seat} stopped answering ({why}); the table picks and builds for it, and its games are forfeit"));
        self.run_auto_seats();
    }

    /// Whether the table plays for this seat (its games are forfeit).
    #[must_use]
    pub fn is_auto(&self, seat: usize) -> bool {
        self.seats[seat].auto
    }

    /// Pick and build for every auto seat with something to do and no
    /// timer to wait for; the others get their deadlines armed.
    fn run_auto_seats(&mut self) {
        while let Some(seat) = (0..self.seats.len()).find(|&s| {
            self.picks_at_once(s) && self.phase == Phase::Drafting
                && self.table.in_front(s).is_some()
        }) {
            let _ = self.auto_pick(seat, "the seat is away");
        }
        for seat in 0..self.seats.len() {
            if self.builds_at_once(seat) && !self.seats[seat].ready
                && self.table.seat_done(seat)
            {
                self.auto_build(seat, "the seat is away");
            }
        }
        self.arm_deadlines();
    }

    /// The clock: expire pick and build deadlines. Returns whether anything
    /// changed.
    pub fn tick(&mut self, now: Instant) -> bool {
        let mut changed = false;
        for seat in 0..self.seats.len() {
            let why = if self.seats[seat].auto { "the seat is away" } else { "the pick timer ran out" };
            if self.seats[seat].pick_deadline.is_some_and(|d| d <= now) {
                self.seats[seat].pick_deadline = None;
                if self.auto_pick(seat, why).is_ok() {
                    changed = true;
                }
            }
            if self.seats[seat].build_deadline.is_some_and(|d| d <= now) {
                self.seats[seat].build_deadline = None;
                let why = if self.seats[seat].auto { "the seat is away" } else { "the build timer ran out" };
                self.auto_build(seat, why);
                changed = true;
            }
        }
        if changed {
            self.run_auto_seats();
        }
        changed
    }

    // ── playing ──

    /// Every deck is in: open the tournament and pair the first round.
    /// Returns the matches to play, `(round, a, b)`, byes excluded.
    #[must_use]
    pub fn begin_tournament(&mut self) -> Vec<(usize, usize, usize)> {
        if self.tournament.is_some() {
            return Vec::new();
        }
        let config = TournamentConfig { best_of: self.config.best_of };
        self.tournament = Some(Tournament::new(self.seats.len(), config));
        self.phase = Phase::Playing;
        log_section!(self.log, "TOURNAMENT");
        self.event(format!("every deck is in; the tournament starts: {} rounds, best-of-{}",
            self.tournament.as_ref().map_or(0, Tournament::total_rounds), self.config.best_of));
        self.next_round()
    }

    /// Pair the next round. Returns the matches to play.
    fn next_round(&mut self) -> Vec<(usize, usize, usize)> {
        let Some(t) = &self.tournament else { return Vec::new() };
        if t.is_complete() {
            self.finish();
            return Vec::new();
        }
        let round = t.rounds.len() + 1;
        let pairings = t.generate_pairings();
        self.round_pairings.clone_from(&pairings);
        let mut to_play = Vec::new();
        for &(a, b) in &pairings {
            if b == BYE {
                log_bye!(self.log, round, a);
                self.event(format!("round {round}: seat {a} has a bye"));
            } else {
                self.matches.push(MatchState {
                    round, a, b, urls: [None, None], status: MatchStatus::Waiting,
                    games: Vec::new(), result: None,
                });
                self.event(format!("round {round}: seat {a} vs seat {b}"));
                to_play.push((round, a, b));
            }
        }
        to_play
    }

    fn match_mut(&mut self, round: usize, a: usize, b: usize) -> Option<&mut MatchState> {
        self.matches.iter_mut().find(|m| m.round == round && m.a == a && m.b == b)
    }

    /// A match's game pages are up: `urls` are seat `a`'s and seat `b`'s.
    pub fn match_started(&mut self, round: usize, a: usize, b: usize, urls: [Option<String>; 2]) {
        if let Some(m) = self.match_mut(round, a, b) {
            m.urls.clone_from(&urls);
            m.status = MatchStatus::Playing;
        }
        for (seat, url) in [a, b].into_iter().zip(urls) {
            if let Some(url) = url {
                self.event(format!("round {round}: seat {seat} plays at {url}"));
            }
        }
    }

    /// One game of a match ended.
    pub fn game_finished(&mut self, round: usize, a: usize, b: usize, outcome: &GameOutcome) {
        let mut game_number = 0;
        if let Some(m) = self.match_mut(round, a, b) {
            m.games.push(outcome.winner);
            game_number = m.games.len();
        }
        self.event(match outcome.winner {
            Some(w) => format!("round {round}: seat {a} vs seat {b}, game {game_number}: seat {w} wins"),
            // A game stopped without a result is not a draw (#743): the page
            // says "Game abandoned" for it, and so does the table.
            None if outcome.abandoned => format!("round {round}: seat {a} vs seat {b}, game {game_number}: abandoned, no winner"),
            None => format!("round {round}: seat {a} vs seat {b}, game {game_number}: drawn"),
        });
    }

    /// A match ended. When the round is over it is recorded, the next one
    /// paired, and the matches to play next are returned.
    pub fn match_finished(&mut self, round: usize, a: usize, b: usize, result: MatchResult) -> Vec<(usize, usize, usize)> {
        log_match_result!(self.log, round, a, b, result.wins_a, result.wins_b, result.winner());
        for (n, game) in result.games.iter().enumerate() {
            log_game_log!(self.log, round, n + 1, a, b, &game.game_log);
        }
        self.event(format!("round {round}: {}", crate::standings::match_score_line(&result).trim_start()));
        if let Some(m) = self.match_mut(round, a, b) {
            m.games = result.games.iter().map(|g| g.winner).collect();
            m.result = Some(result);
            m.status = MatchStatus::Done;
        }
        let round_done = self.matches.iter().filter(|m| m.round == round)
            .all(|m| m.status == MatchStatus::Done);
        if !round_done {
            return Vec::new();
        }
        let results: Vec<MatchResult> = self.matches.iter()
            .filter(|m| m.round == round)
            .filter_map(|m| m.result.clone())
            .collect();
        let pairings = std::mem::take(&mut self.round_pairings);
        if let Some(t) = &mut self.tournament {
            t.record_round(pairings, results);
        }
        self.event(format!("round {round} is over"));
        self.next_round()
    }

    /// The match a seat the table plays for loses without playing: every
    /// game it would have played is a forfeit to its opponent, or a draw
    /// when both seats are away.
    #[must_use]
    pub fn forfeit_result(&self, a: usize, b: usize) -> MatchResult {
        let needed = mtg_draft::tournament::wins_needed(self.config.best_of);
        let (wins_a, wins_b, games) = match (self.is_auto(a), self.is_auto(b)) {
            (true, false) => (0, needed, vec![(Some(b), Some(a)); needed]),
            (false, true) => (needed, 0, vec![(Some(a), Some(b)); needed]),
            _ => (0, 0, Vec::new()),
        };
        MatchResult {
            player_a: a, player_b: b, wins_a, wins_b,
            games: games.into_iter().map(|(winner, stalled)| GameOutcome {
                winner, turns: 0, game_log: vec!["forfeit: the seat is away".to_string()],
                stalled_seat: stalled, abandoned: false,
            }).collect(),
        }
    }

    fn finish(&mut self) {
        if self.phase == Phase::Done {
            return;
        }
        self.phase = Phase::Done;
        let Some(t) = &self.tournament else { return };
        let sorted = t.sorted_standings();
        let tags: Vec<RowTags> = (0..self.seats.len()).map(|seat| RowTags {
            runner_built_deck: self.seats[seat].deck_fallback,
            games_forfeited: t.rounds.iter().flat_map(|r| r.results.iter())
                .flat_map(|m| m.games.iter()).filter(|g| g.stalled_seat == Some(seat)).count(),
            ..RowTags::default()
        }).collect();
        log_section!(self.log, "FINAL STANDINGS");
        log_standings!(self.log, &sorted, &tags);
        self.event("the tournament is over. Final standings:".to_string());
        for (rank, s) in sorted.iter().enumerate() {
            self.event(format!("  {}", crate::standings::standings_row(rank + 1, s, &tags[s.seat])));
        }
    }

    // ── names ──

    /// A seat's cosmetic name.
    pub fn set_name(&mut self, seat: usize, name: &str) {
        let clean: String = name.chars().filter(|c| !c.is_control()).take(24).collect();
        let clean = clean.trim().to_string();
        if clean.is_empty() {
            return;
        }
        self.event(format!("seat {seat} is now called {clean}"));
        self.seats[seat].name = clean;
    }

    // ── the view ──

    fn status_word(&self, seat: usize) -> &'static str {
        let s = &self.seats[seat];
        match self.phase {
            Phase::Lobby | Phase::Done => "idle",
            Phase::Drafting | Phase::Building if s.ready => "ready",
            Phase::Drafting | Phase::Building if self.table.seat_done(seat) => "building",
            Phase::Drafting | Phase::Building if self.table.in_front(seat).is_some() => "picking",
            Phase::Drafting | Phase::Building => "waiting",
            Phase::Playing => {
                let playing = self.matches.iter().any(|m| m.status == MatchStatus::Playing && (m.a == seat || m.b == seat));
                if playing { "playing" } else { "waiting" }
            }
        }
    }

    /// What `seat` is shown. The notice stays until `clear_notice`.
    #[must_use]
    pub fn view(&self, seat: usize) -> View {
        let notice = self.seats[seat].notice.clone();
        let now = Instant::now();
        let seats: Vec<SeatView> = (0..self.seats.len()).map(|i| {
            let s = &self.seats[i];
            SeatView {
                seat: i,
                kind: s.kind.word(),
                name: s.name.clone(),
                joined: s.joined,
                connected: s.connected,
                status: self.status_word(i),
                picks: self.table.picks(i).len(),
                auto: s.auto,
            }
        }).collect();
        let pack = if self.phase == Phase::Drafting {
            self.table.in_front(seat).map(|f| PackView {
                id: f.id,
                round: f.round,
                pick: f.pick,
                size: f.size,
                cards: f.cards.iter().enumerate().map(|(i, c)| self.book.card(c, Some(i))).collect(),
                waiting: f.waiting,
                deadline_ms: self.seats[seat].pick_deadline
                    .map(|d| millis_left(d, now)),
            })
        } else {
            None
        };
        let pool: Vec<CardView> = self.table.pool(seat).iter().map(|c| self.book.card(c, None)).collect();
        let picks: Vec<PickView> = self.table.picks(seat).iter().map(|p: &TablePick| PickView {
            round: p.round, pick: p.pick, card: mtg_draft::front_face(&p.card).to_string(), auto: p.auto,
        }).collect();
        let s = &self.seats[seat];
        let deck = s.deck.as_ref().map(|d| DeckView {
            main: d.maindeck.clone(),
            lands: d.lands.clone(),
            sideboard: d.sideboard.clone(),
            valid: s.deck_problem.is_none(),
            problem: s.deck_problem.clone(),
            ready: s.ready,
        });
        let matches: Vec<MatchView> = self.matches.iter()
            .filter(|m| m.a == seat || m.b == seat)
            .map(|m| {
                let (me, opponent) = if m.a == seat { (0, m.b) } else { (1, m.a) };
                MatchView {
                    round: m.round,
                    opponent,
                    url: m.urls[me].clone(),
                    status: m.status.word(),
                    games: m.games.iter().map(|w| GameView { winner: *w }).collect(),
                    result: m.result.as_ref().map(|r| score_for(r, seat)),
                }
            })
            .collect();
        let mut pairings: Vec<PairingView> = self.matches.iter().map(|m| PairingView {
            round: m.round, a: m.a, b: Some(m.b), status: m.status.word(),
            result: m.result.as_ref().map(|r| format!("{}-{}", r.wins_a, r.wins_b)),
        }).collect();
        if let Some(t) = &self.tournament {
            for r in &t.rounds {
                for &(a, _) in r.pairings.iter().filter(|(_, b)| *b == BYE) {
                    pairings.push(PairingView { round: r.round_number, a, b: None, status: "done", result: Some("bye".into()) });
                }
            }
            for &(a, _) in self.round_pairings.iter().filter(|(_, b)| *b == BYE) {
                pairings.push(PairingView { round: t.rounds.len() + 1, a, b: None, status: "done", result: Some("bye".into()) });
            }
        }
        pairings.sort_by_key(|p| (p.round, p.a));
        let standings: Vec<StandingView> = self.tournament.as_ref().map(|t| {
            t.sorted_standings().iter().map(|s| StandingView {
                seat: s.seat, wins: s.match_wins, losses: s.match_losses, draws: s.match_draws,
                points: s.match_points(), game_wins: s.game_wins, byes: s.byes,
            }).collect()
        }).unwrap_or_default();
        View {
            kind: "view",
            phase: self.phase,
            seat,
            pod_size: self.seats.len(),
            set: self.config.set_code.clone(),
            seats,
            pass_direction: self.table.pass_direction().filter(|_| self.phase == Phase::Drafting),
            pack,
            pool,
            picks,
            deck,
            matches,
            pairings,
            standings,
            notice,
            pick_seconds: self.config.pick_seconds,
            build_seconds: self.config.build_seconds,
            build_deadline_ms: self.seats[seat].build_deadline
                .map(|d| millis_left(d, now)),
        }
    }
}

/// Whether a deck's problem is only that it is short: the one state a
/// deck passes through on its way to legal, so not a refusal. Every other
/// problem `validate_deck` reports names a card or a land that cannot be
/// in the deck at all.
#[must_use]
pub fn deck_in_progress(problem: &str) -> bool {
    problem.starts_with("Deck has ") && problem.contains("need at least")
}

/// Milliseconds from `now` to `deadline`, 0 once it has passed.
fn millis_left(deadline: Instant, now: Instant) -> u64 {
    u64::try_from(deadline.saturating_duration_since(now).as_millis()).unwrap_or(u64::MAX)
}

/// A match's score from one seat's side.
fn score_for(r: &MatchResult, seat: usize) -> String {
    if r.player_a == seat {
        format!("{}-{}", r.wins_a, r.wins_b)
    } else {
        format!("{}-{}", r.wins_b, r.wins_a)
    }
}

/// The card the table takes for an absent seat: the first card of the
/// pack the fallback deck would play alongside the pool, else the first
/// card of the pack.
#[must_use]
pub fn auto_pick_index(cards: &[String], pool: &[String], registry: &CardRegistry) -> usize {
    for (i, card) in cards.iter().enumerate() {
        let mut with = pool.to_vec();
        with.push(card.clone());
        let deck = deckbuilding::fallback_deck(&with, registry);
        let in_pool = pool.iter().filter(|c| mtg_draft::front_face(c) == mtg_draft::front_face(card)).count();
        let played = deck.maindeck.iter().filter(|c| mtg_draft::front_face(c) == mtg_draft::front_face(card)).count();
        if played > in_pool {
            return i;
        }
    }
    0
}

/// Where the log's sibling files go: the directory the log is in.
#[must_use]
pub fn out_dir_for(log_path: &Path) -> PathBuf {
    log_path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seats_parse_in_order_with_counts_and_models() {
        let seats = parse_seats("human,ai,ai:cc:claude-sonnet-4-6,2xhuman", "cc").unwrap();
        assert_eq!(seats, vec![
            SeatKind::Human, SeatKind::Ai("cc".into()), SeatKind::Ai("cc:claude-sonnet-4-6".into()),
            SeatKind::Human, SeatKind::Human,
        ]);
        assert_eq!(parse_seats("1xhuman,7xai", "cc").unwrap().len(), 8);
        assert_eq!(parse_seats("cli,ai", "cc").unwrap()[0], SeatKind::Cli);
    }

    #[test]
    fn bad_seat_lists_are_refused_by_name() {
        assert!(parse_seats("", "cc").unwrap_err().contains("nobody"));
        assert!(parse_seats("human", "cc").unwrap_err().contains("2 to 8"));
        assert!(parse_seats("9xai", "cc").unwrap_err().contains("2 to 8"));
        assert!(parse_seats("human,robot", "cc").unwrap_err().contains("'robot' is not a seat"));
        assert!(parse_seats("cli,cli", "cc").unwrap_err().contains("at most one cli"));
        assert!(parse_seats("0xhuman,ai", "cc").unwrap_err().contains("nobody"));
        assert!(parse_seats("ai:,human", "cc").unwrap_err().contains("not a seat"));
    }

    #[test]
    fn a_short_deck_is_in_progress_and_a_wrong_card_is_not() {
        let pool: Vec<String> = ["Abbey Griffin", "Chapel Geist"].map(String::from).to_vec();
        let short = deckbuilding::validate_deck(&pool, &pool[..1], &HashMap::new()).unwrap_err();
        assert!(deck_in_progress(&short), "{short}");
        let wrong = deckbuilding::validate_deck(&pool, &["Griselbrand".to_string()], &HashMap::new()).unwrap_err();
        assert!(!deck_in_progress(&wrong), "{wrong}");
        let copies = deckbuilding::validate_deck(&pool, &[pool[0].clone(), pool[0].clone()], &HashMap::new()).unwrap_err();
        assert!(!deck_in_progress(&copies), "{copies}");
        let land = deckbuilding::validate_deck(&pool, &pool, &HashMap::from([("Shimmering Grotto".to_string(), 1)])).unwrap_err();
        assert!(!deck_in_progress(&land), "{land}");
    }

    #[test]
    fn a_key_is_128_bits_of_hex_and_never_the_same_twice() {
        let a = new_key();
        let b = new_key();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}

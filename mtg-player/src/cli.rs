use std::collections::HashMap;
use std::io::{self, Write, stdout};

use crossterm::{
    cursor, execute,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Color, SetForegroundColor, SetBackgroundColor, SetAttribute, Attribute, ResetColor, Print},
    terminal::{self, Clear, ClearType},
};

use mtg_engine::actions::{Action, CombatPrompt, Target};
use mtg_engine::types::Step;
use mtg_engine::ids::ObjectId;
use mtg_engine::types::CardType;
use mtg_engine::view::{GameView, PermanentView};

use crate::Player;

/// Global flag: set to true when the user requests a hot reload (rr).
pub static HOT_RELOAD_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// The (seat, prompt-kind) of the last decision that actually read input,
/// shared across both hotseat CliPlayer instances — the terminal's event
/// queue is process-global, so seat-crossing has to be tracked globally too.
static LAST_DECISION_IDENTITY: std::sync::Mutex<Option<(String, String)>> =
    std::sync::Mutex::new(None);

/// The terminal settings from before the TUI ever touched them, captured
/// when the signal handlers are installed, for the handler to restore.
static SANE_TERMIOS: std::sync::OnceLock<libc::termios> = std::sync::OnceLock::new();

/// Signal handler: put the terminal back, then die with the conventional
/// 128+sig code. Restricted to async-signal-safe calls — `tcsetattr` with
/// a pre-captured termios, a `write(2)` of the bracketed-paste-off /
/// show-cursor sequences, `_exit` — so no crossterm, no locks, no
/// allocation (`OnceLock::get` after initialization is a plain atomic
/// load).
extern "C" fn restore_terminal_and_exit(sig: libc::c_int) {
    unsafe {
        if let Some(t) = SANE_TERMIOS.get() {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, t);
        }
        // Bracketed paste off, cursor back on, clear the screen, home the
        // cursor — one write of literal escape bytes, because a signal
        // handler may only call async-signal-safe functions and `write` is
        // the one that qualifies. Without the clear, a signal death left the
        // TUI frame on screen for the shell to paint into (#235), the same
        // defect the Ctrl-C key path had.
        let seq = b"\x1b[?2004l\x1b[?25h\x1b[2J\x1b[H";
        libc::write(libc::STDOUT_FILENO, seq.as_ptr().cast(), seq.len());
        unlink_scratch_file();
        libc::_exit(128 + sig);
    }
}

/// A scratch file this process must take with it when it dies.
///
/// The runner writes a hot-reload snapshot of the whole game state before
/// every decision, whether or not `--save` was given, and used to unlink it
/// only on the normal-completion path — so Ctrl-C, a signal, or a closed
/// window stranded a ~100 KB file holding both players' hands and libraries
/// in a world-readable /tmp, forever (issues #234, #239).
static SCRATCH_FILE: std::sync::OnceLock<std::ffi::CString> = std::sync::OnceLock::new();

/// Name the file to remove on the way out. Called once, before the game
/// starts; a second call is ignored.
pub fn unlink_on_exit(path: &str) {
    if let Ok(c) = std::ffi::CString::new(path) {
        let _ = SCRATCH_FILE.set(c);
    }
}

/// Remove the registered scratch file.
///
/// Async-signal-safe, so the signal handler above can call it: `unlink(2)`
/// is on the list, and `OnceLock::get` after initialization is a plain
/// atomic load.
pub fn unlink_scratch_file() {
    if let Some(path) = SCRATCH_FILE.get() {
        unsafe { libc::unlink(path.as_ptr()) };
    }
}

/// Ctrl-C at a prompt: put the terminal back, take the scratch file with
/// us, and exit as an interrupted program does.
fn quit_at_prompt() -> ! {
    // Clear the frame on the way out, the same as the game-over path does
    // (#47). Restoring the terminal MODES without clearing left the whole
    // TUI on screen, and the shell prompt and every command after it painted
    // inside the abandoned game board (#235). This runs in normal context —
    // it is a key handler, not a signal handler — so it can do the full
    // reset rather than the async-signal-safe subset.
    reset_terminal_for_exit();
    unlink_scratch_file();
    std::process::exit(0);
}

/// True while a TUI prompt holds the terminal in raw mode, and the raw
/// termios itself — what the SIGCONT handler re-arms after a job-control
/// stop/resume put the tty back into cooked mode behind the app's back
/// (issue #104).
static RAW_MODE_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
static RAW_TERMIOS: std::sync::OnceLock<libc::termios> = std::sync::OnceLock::new();

/// Enter raw mode for a prompt, capturing the raw termios once and
/// flagging the state for the SIGCONT handler (issue #104).
fn tui_raw_on() {
    let _ = terminal::enable_raw_mode();
    if RAW_TERMIOS.get().is_none() {
        unsafe {
            let mut t: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut t) == 0 {
                let _ = RAW_TERMIOS.set(t);
            }
        }
    }
    RAW_MODE_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Leave raw mode, clearing the SIGCONT re-arm flag first so a signal
/// racing the switch can't re-raw a terminal we just released.
fn tui_raw_off() {
    RAW_MODE_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = terminal::disable_raw_mode();
}

/// SIGCONT handler: when the process was stopped (SIGTSTP/SIGSTOP) inside
/// a raw-mode prompt and resumed under a job-control shell, the shell
/// restores its own cooked termios across the stop. crossterm still
/// believes raw mode is on, so the cooked line discipline's `\n` never
/// parses as Enter and every prompt is permanently deaf (issue #104).
/// Re-arm the raw termios on resume. Async-signal-safe: an atomic load,
/// an initialized `OnceLock::get`, and `tcsetattr`.
extern "C" fn rearm_raw_mode_on_cont(_sig: libc::c_int) {
    if RAW_MODE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
        if let Some(t) = RAW_TERMIOS.get() {
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, t);
            }
        }
    }
}

/// Whether a CLI seat has any usable terminal: stdin is a tty, or a
/// controlling terminal exists to fall back to (crossterm reads
/// `/dev/tty` when stdin is redirected). With neither, every
/// `event::read` fails instantly and the prompt loop is a silent
/// 100%-CPU spin (issue #103) — the runner uses this to refuse the
/// seat up front instead.
#[must_use]
pub fn terminal_available() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() || std::fs::File::open("/dev/tty").is_ok()
}

thread_local! {
    static READ_ERRORS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// `event::read` with the issue-#103 guard. A read error used to mean
/// "retry immediately", which with no terminal at all turned every prompt
/// into a silent hot loop. Failures now back off, and an unbroken run of
/// them long enough to rule out a transient (EINTR from a signal, a
/// momentary EIO) restores the terminal and exits with an explanation.
fn read_event_guarded() -> Option<Event> {
    match event::read() {
        Ok(ev) => {
            READ_ERRORS.with(|c| c.set(0));
            Some(ev)
        }
        Err(_) => {
            let n = READ_ERRORS.with(|c| {
                let n = c.get().saturating_add(1);
                c.set(n);
                n
            });
            if n >= 200 {
                reset_terminal_for_exit();
                eprintln!("Error: cannot read terminal input (terminal gone?); exiting");
                std::process::exit(1);
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
            None
        }
    }
}

/// Install SIGHUP/SIGTERM/SIGINT handlers that restore the terminal
/// before exiting (issue #78), plus the SIGCONT re-arm handler for
/// stop/resume under a job-control shell (issue #104). A signal landing
/// while a prompt held the terminal in raw mode used to leave the pty raw
/// for the inheriting shell — no echo, no line editing, no Ctrl-C,
/// staircased output — on the ordinary close-the-window (SIGHUP) and
/// `kill`/`timeout` (SIGTERM) paths, forcing a blind `stty sane`. The
/// runner calls this once, before the first prompt, when a human CLI seat
/// exists.
pub fn install_terminal_restore_signal_handlers() {
    unsafe {
        let mut t: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(libc::STDIN_FILENO, &mut t) == 0 {
            let _ = SANE_TERMIOS.set(t);
        }
        let handler = restore_terminal_and_exit as extern "C" fn(libc::c_int);
        for sig in [libc::SIGHUP, libc::SIGTERM, libc::SIGINT] {
            libc::signal(sig, handler as libc::sighandler_t);
        }
        let cont_handler = rearm_raw_mode_on_cont as extern "C" fn(libc::c_int);
        libc::signal(libc::SIGCONT, cont_handler as libc::sighandler_t);
    }
}

/// Restore the terminal for normal line-oriented output after the TUI:
/// leave raw mode, clear the last rendered frame, and home the cursor.
/// The runner calls this before printing the end-of-game summary so the
/// summary doesn't land on top of a stale frame and visually merge with
/// leftover rows (issue #47).
pub fn reset_terminal_for_exit() {
    tui_raw_off();
    let mut out = stdout();
    // Defensive: bracketed paste must never survive into the user's shell.
    let _ = execute!(out, event::DisableBracketedPaste);
    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
    let _ = out.flush();
}

/// Handle for the background spinner thread. Drop to stop.
pub struct SpinnerHandle {
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for SpinnerHandle {
    fn drop(&mut self) {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        // Don't join — let the thread die on its own. The next render
        // will overwrite whatever it last printed.
    }
}

/// A player that interacts via a terminal UI.
/// What step/turn the player wants to auto-pass until.
#[derive(Clone, Debug)]
enum PassMode {
    /// Pass until our next Main Phase 1.
    UntilNextTurn {
        activated_turn: u32,
        /// True when 'f' was pressed on our own turn before our precombat
        /// main — our "next Main Phase 1" is then still THIS turn's, so the
        /// break clauses must not wait for a later turn number (issue #45).
        before_our_main: bool,
    },
}

/// Which trailing rows a target chooser offers below the targets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ChooserRows {
    /// One target to pick, or abandon the cast.
    CancelOnly,
    /// One of an "up to N" batch: pick, stop here and cast, or abandon.
    DoneThenCancel,
}

/// What a line typed at a target chooser means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TargetInput {
    Pick(usize),
    Done,
    Cancel,
    Panel(char),
    /// The `m` the frame advertises whenever the menu is longer than the
    /// pane. It used to be answered with "Invalid input 'm'" on the same
    /// frame that offered it (issue #261).
    NextPage,
    /// `p`, the way back (issue #255).
    PrevPage,
    Invalid,
}


/// Why auto-pass stops at a prompt.
///
/// It used to be a bare `bool`, and the refusal message was reconstructed
/// afterwards by looking for the first actionable-looking thing on the menu —
/// so a prompt blocked by the postcombat-main stop was reported as having "a
/// castable spell it would skip" (issue #294). The reason travels with the
/// decision now, so what the player is told is what actually happened.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BreakReason {
    /// A land drop is once per turn and free, so it is never passed (#39).
    LandPlay,
    /// The Main Phase 1 auto-pass was passing towards.
    TargetMainPhase,
    /// A sorcery-speed action on your own turn.
    MeaningfulAction,
    /// Something on the stack you have a real answer to.
    StackResponse,
    /// The opponent is attacking.
    Attackers,
    /// Your own postcombat main, for removal on damaged creatures.
    YourPostcombatMain,
}

impl BreakReason {
    /// The clause, in the second half of "Auto-pass not engaged: this prompt
    /// has ...".
    fn describe(self) -> &'static str {
        match self {
            BreakReason::LandPlay =>
                "a land play it would skip. Pass with 0 first to decline it",
            BreakReason::TargetMainPhase =>
                "the Main Phase 1 auto-pass passes towards — there is nothing left to skip",
            BreakReason::MeaningfulAction =>
                "an action of yours it would skip. Pass with 0 first to decline it",
            BreakReason::StackResponse =>
                "something on the stack to respond to. Pass with 0 to decline",
            BreakReason::Attackers =>
                "an attack to respond to. Pass with 0 to decline",
            BreakReason::YourPostcombatMain =>
                "your postcombat main phase, a stop auto-pass always honours. \
                 Pass with 0 to move on",
        }
    }
}

/// One round of an "up to N targets" prompt (see `prompt_target_up_to`).
///
/// It serves both "up to N" slots — a bare `UpToTargets` spell and the wide
/// second slot of a `TwoTargets` spell, which used to have a chooser of its
/// own with no Cancel row (issue #288).
enum UpToPick {
    Pick(mtg_engine::actions::Target),
    Done,
    Cancel,
}

/// Display width of one character in terminal columns (CJK and other wide
/// characters take two cells). Clipping by `char` count let 20 wide chars
/// overflow a 40-column region and wrap over neighbouring panels (#109).
/// The printable form of typed or pasted text.
///
/// Control and escape bytes become a visible placeholder instead of being
/// written to the terminal, where they are *executed*: a pasted `ESC[1;1H`
/// moved the cursor home and painted over the frame, and the error notice
/// then replayed it on every re-render (issue #282). A paste of ordinary
/// terminal output — a log line with colour codes — contains them by
/// accident.
///
/// Public because the terminal is not the only place untrusted text is
/// printed: a save file's deck names reach the runner's banner, its
/// game-over summary and the `--log` (issue #315), and the same rule has to
/// hold there. One definition, so a second sink cannot be fixed differently.
pub fn sanitize_for_display(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { '\u{00b7}' } else { c }).collect()
}

/// The user's input, quoted for an error notice: sanitised so a pasted
/// escape sequence is not re-executed on every render (#282), and clipped so
/// one long entry cannot wrap over the whole frame and splice the prompt row
/// into itself (#283). The echo is capped, so the player never sees how long
/// the line is until they press Enter — the notice must not be the place
/// they find out.
fn quote_input(input: &str) -> String {
    const MAX: usize = 40;
    let shown = sanitize_for_display(input);
    if str_cols(&shown) <= MAX {
        return shown;
    }
    format!("{}\u{2026}", clip_cols(&shown, MAX))
}

/// Repaint an input line from the buffer, clipped to `cap` display columns.
///
/// The readers used to keep a parallel model of what was on screen and paint
/// deltas into it. It desynchronised from the buffer three ways — a grapheme
/// cluster erased more cells than it owned, a zero-width mark erased none,
/// and a wide character that did not fit was skipped while a narrower one
/// after it was not — and once it did, the line rendered EMPTY while the
/// buffer still held a character, so Enter attacked with everything or cast
/// for max X (issue #281). Painting the whole line from the buffer cannot
/// drift from it.
fn repaint_input_line(out: &mut io::Stdout, col: u16, row: u16, buf: &str, cap: usize) {
    let shown = clip_cols(&sanitize_for_display(buf), cap);
    let _ = execute!(out, cursor::MoveTo(col, row), Clear(ClearType::UntilNewLine), Print(shown));
    let _ = out.flush();
}

/// What one key does to the line being typed at a prompt. Shared by both
/// line readers, so the two cannot disagree about it.
///
/// Three kinds of key reach here (Enter, Ctrl-C and the readers' own
/// shortcuts are taken before it):
///
/// - a plain character is typed;
/// - Backspace and Ctrl-U edit (#79);
/// - everything else — Tab, the arrows, Home/End, Delete, the function
///   keys, and any Ctrl/Alt chord — is a SEPARATOR: it is never typed as a
///   character (#51: Ctrl-L must not become the `l` shortcut, and crossterm
///   reports Ctrl-\ as the digit `4` with CONTROL set), but it is not
///   dropped either. Dropping it silently concatenated the digits typed on
///   either side of it: `0 <Tab> 1` became the buffer `01`, which the reader
///   accepted as option 1 — at a mulligan-bottoming prompt, an irreversible
///   choice the player never typed (issue #322). As a separator the same
///   keystrokes read `0 1`, which every numeric prompt refuses out loud, and
///   which a multi-select prompt reads as the two indices they are.
///
/// Returns whether the buffer changed, so the caller knows to repaint.
fn edit_line(buf: &mut String, code: KeyCode, modifiers: KeyModifiers) -> bool {
    match code {
        KeyCode::Backspace => buf.pop().is_some(),
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            let had = !buf.is_empty();
            buf.clear();
            had
        }
        KeyCode::Char(c) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            buf.push(c);
            true
        }
        _ => {
            // One separator is enough, and one at the start would only be
            // trimmed: the buffer is read with its ends trimmed, so a Tab
            // pressed before or after a number changes nothing.
            if buf.is_empty() || buf.ends_with(' ') {
                false
            } else {
                buf.push(' ');
                true
            }
        }
    }
}


fn col_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Truncate `s` to at most `max` display COLUMNS (not chars).
fn clip_cols(s: &str, max: usize) -> String {
    let mut cols = 0;
    let mut out = String::new();
    for c in s.chars() {
        let w = col_width(c);
        if cols + w > max {
            break;
        }
        cols += w;
        out.push(c);
    }
    out
}

/// The page of a menu `render_paged` drew: which rows, and what it had to
/// fit them in — enough for the caller to page backwards exactly when the
/// rows are of uneven height (issue #318).
/// What a pager's marker says the keys are. One string per pager, so the
/// row can be measured before the page it describes is chosen.
const MENU_PAGE_KEYS: &str = "m/p = next/prev page (any number works)";
const ATTACKERS_PAGE_KEYS: &str = "m = next page";
const BLOCKERS_PAGE_KEYS: &str = "b = next page";

/// The pane keys the combat prompts advertise.
const ATTACK_HINTS: &str = "  [d=deck] [l=log] [g=gy] [e=exile] [i=inspect] [s=stack] [m/p=page]";
/// One line of input at a card-set prompt.
#[derive(Debug)]
enum SetInput {
    Toggle(Vec<usize>),
    All,
    None,
    Confirm,
    Cancel,
    Pane(char),
    NextPage,
    PrevPage,
    Invalid(String),
}

/// How to answer a set-picking screen, and the panes it can step into.
const SET_HOW_TO: &str = " Type a number to mark or unmark it, several at once if you like. [a=all] [n=none] [enter = done] [s=stack] [i=board] [g=gy] [e=exile] [l=log] [d=deck] [m/p=page]";

/// What the screen says when the idle key would commit an empty answer
/// nobody chose (issue #262).
const SET_NOTHING_MARKED: &str =
    "nothing marked — mark the cards you want, or press n for none and then enter";

/// One "choose some of these" screen: what it is called, what it asks, the
/// rows to mark, how many may be marked, and whether it can be abandoned.
struct SetPick {
    title: String,
    question: String,
    rows: Vec<String>,
    min: usize,
    max: usize,
    /// `Some(label)` when `c` abandons the choice — an additional cost can
    /// be backed out of, a turn-based action cannot.
    cancel: Option<&'static str>,
}

/// How to answer an ordering screen, and the panes it can step into.
const ORDER_HOW_TO: &str = " Type the numbers in order, e.g. \"2 0 1\". [enter = keep the order shown] [s=stack] [i=board] [g=gy] [e=exile] [l=log] [d=deck] [m/p=page]";

const BLOCK_HINTS: &str = "  [d=deck] [l=log] [g=gy] [e=exile] [i=inspect] [s=stack] [m/b=page]";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct MenuPage {
    /// The first row shown.
    offset: usize,
    /// How many rows were shown from it.
    shown: usize,
    /// Lines the menu had available.
    avail: usize,
    /// Lines the paging marker takes in the pane it was drawn in.
    marker_h: usize,
    /// Every row's height in lines, wrapped to the pane it was drawn in.
    heights: Vec<usize>,
}

/// Which ordering a `prompt_ordering` screen is for (issue #325).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OrderingKind {
    /// CR 603.3b: a player's simultaneous triggers, onto the stack.
    Triggers,
    /// CR 509.2: an attacker's blockers, for damage assignment.
    Blockers,
}

/// What the engine handed the ordering screen.
struct OrderingPrompt<'a> {
    kind: OrderingKind,
    description: &'a str,
    /// One line of text per option, as the engine names them.
    options: &'a [String],
    /// The parts of each trigger, parallel to `options`; empty for a
    /// blocker list or a prompt from an older save.
    details: &'a [mtg_engine::state::TriggerOrderOption],
}

/// One line of input at the ordering prompt, read.
#[derive(Clone, Debug, PartialEq, Eq)]
enum OrderInput {
    /// A complete ordering: every index exactly once.
    Order(Vec<usize>),
    /// One of the info panes.
    Pane(char),
    NextPage,
    PrevPage,
    /// Refused, with the reason to show.
    Invalid(String),
}

/// How a line of the ordering screen is painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style { Title, Bold, Dim, Row, Plain }

/// The slice of a screen's body lines on show, for paging a body taller
/// than the terminal with `m`/`p`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BodyPage {
    start: usize,
    end: usize,
    len: usize,
    avail: usize,
    paged: bool,
}

impl BodyPage {
    fn new(len: usize, avail: usize, offset: usize) -> Self {
        let paged = len > avail;
        // One line goes to the "… showing" marker when paging.
        let per_page = if paged { avail.saturating_sub(1).max(1) } else { avail };
        let start = if paged { offset.min(len.saturating_sub(1)) / per_page * per_page } else { 0 };
        let end = (start + per_page).min(len);
        BodyPage { start, end, len, avail, paged }
    }

    fn per_page(&self) -> usize {
        if self.paged { self.avail.saturating_sub(1).max(1) } else { self.avail.max(1) }
    }

    /// `m`: the next page, wrapping to the top.
    fn next_offset(&self) -> usize {
        if self.end >= self.len { 0 } else { self.end }
    }

    /// `p`: the previous page, wrapping to the last.
    fn prev_offset(&self) -> usize {
        if self.start == 0 {
            self.len.saturating_sub(1) / self.per_page() * self.per_page()
        } else {
            self.start.saturating_sub(self.per_page())
        }
    }
}

/// One combat-list row as it will be drawn: the entry's lines, and the

/// caller's coloured note — kept whole, on the last line or on lines of its
/// own (issues #328, #318).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CombatRowLayout {
    /// The entry, wrapped to the pane.
    lines: Vec<String>,
    /// The note as the caller passed it (leading space included).
    note: String,
    /// The note wrapped onto lines of its own, when it did not fit after
    /// the entry's last line. Empty when it did, or when there is no note.
    note_lines: Vec<String>,
}

impl CombatRowLayout {
    /// Lines the row takes on screen.
    fn height(&self) -> usize {
        self.lines.len() + self.note_lines.len()
    }

    /// Everything the row says, line breaks taken back out.
    #[cfg(test)]
    fn text(&self) -> String {
        let mut s = self.lines.join(" ");
        if self.note_lines.is_empty() {
            s.push_str(&self.note);
        } else {
            s.push(' ');
            s.push_str(&self.note_lines.join(" "));
        }
        s
    }
}

/// What a menu row stands for: an action to submit, or a spell to walk
/// through the casting flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayEntry {


    /// Index into `LegalActions::actions`.
    Direct(usize),
    /// Index into `LegalActions::castable_spells`.
    Cast(usize),
}

/// One row of a menu: its text, and the objects that make it the choice it
/// is.
///
/// The text is shown whole. It used to be clipped to the pane in three
/// regions (a head and tail that named objects, prose in between that could
/// be eaten), and every clip strategy lost something a real game turned out
/// to need: the target (#36, #80), the source (#257), the description (#258),
/// and finally the third card of a "Bottom A, B, C" row, which is what told
/// 14 of 25 bottoming options apart (issue #318). A row that does not fit
/// its line now wraps onto the next, under a hanging indent, and nothing on
/// it is ever cut.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct MenuLabel {
    text: String,
    /// The objects that give this row its identity, in a fixed order:
    /// source, then each target, then the sacrifice. Two rows with the same
    /// ids are interchangeable; two rows with different ids are not, however
    /// alike they read (issue #257).
    ids: Vec<u64>,
}

impl MenuLabel {
    /// A row with nothing to tell apart.
    fn plain(s: impl Into<String>) -> Self {
        MenuLabel { text: s.into(), ..MenuLabel::default() }
    }

    fn full(&self) -> String {
        self.text.clone()
    }
}


/// Display width of `s` in terminal columns.
fn str_cols(s: &str) -> usize {
    s.chars().map(col_width).sum()
}

/// One line of a full-screen info view (`l`/`g`/`e`), carrying just enough
/// styling for the shared pager to render it (issues #101/#102).
enum InfoLine {
    Plain(String),
    Bold(String),
    Dim(String),
    /// Indented card line rendered through `print_with_mana`.
    Mana(String),
}

/// A card the reference panel shows: the data of the face it is showing —
/// the back face for a transformed permanent (issue #238) — and whether the
/// card prints `*`/`*`, so the panel never renders the engine's `Some(0)`
/// creature sentinel as a printed P/T (issue #267).
struct CardRef {
    data: mtg_engine::cards::CardData,
    star_pt: bool,
}

pub struct CliPlayer {
    name: String,
    /// When set, auto-pass priority until the specified condition.
    pass_mode: Option<PassMode>,
    /// Filter string for the card reference panel.
    card_filter: String,
    /// A message for the NEXT prompt shown. Pressing 'f' answers the current
    /// one immediately, so its confirmation has nowhere to go but forward —
    /// and without it engaging auto-pass produced no feedback at all
    /// (issue #296).
    pending_notice: Option<String>,
}

impl CliPlayer {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pass_mode: None,
            card_filter: String::new(),
            pending_notice: None,
        }
    }

    /// Drop pending type-ahead when the decision being prompted changes
    /// identity — a different seat, or a different kind of prompt (the
    /// action menu vs. a mandatory discard/sacrifice/bottoming menu, which
    /// all share the same raw-mode reader). A keystroke must never answer a
    /// prompt the player has not been shown: ordinary type-ahead against
    /// one seat's main-phase menu survived the seat change and answered the
    /// other player's mandatory cleanup discard — and picked which creature
    /// an opponent sacrificed to Tribute to Hunger (issue #71).
    ///
    /// Repeats of the SAME identity keep their type-ahead: spamming Enter
    /// through your own priority prompts still works. The surviving bytes
    /// live in crossterm's parsed event queue, not the kernel tty buffer,
    /// so the drain reads events, and raw mode must be on for `poll` to
    /// see them.
    fn drain_stale_input(&self, kind: &str) {
        let id = (self.name.clone(), kind.to_string());
        let mut last = match LAST_DECISION_IDENTITY.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if last.as_ref() == Some(&id) {
            return;
        }
        *last = Some(id);
        let was_raw = terminal::is_raw_mode_enabled().unwrap_or(false);
        tui_raw_on();
        while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            let _ = event::read();
        }
        if !was_raw {
            tui_raw_off();
        }
    }

    // ── Pass mode logic ─────────────────────────────────────────────

    /// The pass mode 'f' would engage at the current prompt.
    fn new_pass_mode(view: &GameView) -> PassMode {
        PassMode::UntilNextTurn {
            activated_turn: view.turn_number,
            before_our_main: view.active_player == view.you
                && matches!(view.step, Step::Untap | Step::Upkeep | Step::Draw),
        }
    }

    /// Decide what pressing 'f' does at the current prompt: the pass mode to
    /// engage, or the reason the current prompt already breaks — engaging
    /// there would pass over a decision auto-pass promises to stop for
    /// (issue #48). One predicate for both questions, so the refusal names
    /// the clause that actually blocked engagement rather than the first
    /// actionable-looking row on the menu (issue #294).
    fn try_engage_auto_pass(
        view: &GameView,
        legal: &mtg_engine::engine::LegalActions,
    ) -> Result<PassMode, BreakReason> {
        let mode = Self::new_pass_mode(view);
        match Self::should_break_pass(view, legal, &mode) {
            Some(reason) => Err(reason),
            None => Ok(mode),
        }
    }

    /// Why the current pass mode should stop and hand control back, or
    /// `None` to keep passing.
    ///
    /// The set of stops is the one `.claude/commands/play-cli.md` documents.
    /// Two of them used to be wider than their own rationale: the
    /// declare-attackers stop tested whether the opponent merely CONTROLLED
    /// a creature rather than whether one was attacking, and the
    /// postcombat-main stop ("so the player can use removal on damaged
    /// creatures" — a your-turn rationale) had no player test at all, so
    /// auto-pass engaged on the opponent's turn died at THEIR main phase 2,
    /// a phase and a turn short of where it was going (issue #295).
    fn should_break_pass(
        view: &GameView,
        legal: &mtg_engine::engine::LegalActions,
        mode: &PassMode,
    ) -> Option<BreakReason> {
        match mode {
            PassMode::UntilNextTurn { activated_turn, before_our_main } => {
                // "Our next Main Phase 1" is this turn's when 'f' was pressed
                // before it, and a later turn's otherwise (issue #45 — the
                // spell/ability clauses below used to require a strictly
                // later turn, silently skipping a same-turn castable spell).
                let reached_target_turn = view.turn_number > *activated_turn
                    || (*before_our_main && view.turn_number == *activated_turn);
                let our_turn = view.active_player == view.you;

                // A land drop is never auto-passed, whatever the turn: once
                // per turn and free, a land play is always worth stopping
                // for (issue #39).
                if legal.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })) {
                    return Some(BreakReason::LandPlay);
                }

                // Break at our precombat main once we reach the target turn.
                if our_turn && reached_target_turn && view.step == Step::PrecombatMain {
                    return Some(BreakReason::TargetMainPhase);
                }

                // Break on our turn if we have meaningful actions (cast
                // spells, activate non-mana abilities) — even outside a main
                // phase.
                if our_turn && reached_target_turn {
                    let has_meaningful = legal.actions.iter().any(|a| matches!(a,
                        Action::PlayLand { .. }
                        | Action::CastSpell { .. }
                        | Action::ActivateAbility { .. }
                    ));
                    if has_meaningful {
                        return Some(BreakReason::MeaningfulAction);
                    }
                }

                // Break if something is on the stack AND we have a meaningful
                // response (not just pass/concede/mana abilities).
                if !view.stack.is_empty() {
                    let has_response = legal.actions.iter().any(|a| !matches!(a,
                        Action::PassPriority | Action::Concede | Action::ActivateManaAbility { .. }
                    ));
                    if has_response {
                        return Some(BreakReason::StackResponse);
                    }
                }

                // Break at the opponent's DeclareAttackers only if they are
                // actually attacking. Testing "controls a creature" stopped
                // auto-pass at a combat where the opponent had declared no
                // attackers at all, on a menu offering pass, a mana ability
                // and concede (issue #295).
                if !our_turn && view.step == Step::DeclareAttackers {
                    let under_attack = view.battlefield.iter().any(|p| {
                        p.controller != view.you && p.attacking.is_some()
                    });
                    if under_attack {
                        return Some(BreakReason::Attackers);
                    }
                }

                // Break after combat ends so the player can use removal or
                // burn on damaged creatures — which is a reason about their
                // OWN postcombat main, not the opponent's.
                if our_turn && view.step == Step::PostcombatMain {
                    return Some(BreakReason::YourPostcombatMain);
                }

                None
            }
        }
    }

    // ── Text wrapping ──────────────────────────────────────────────

    /// Word-wrap a string to fit within `width` characters.
    /// Returns a Vec of lines. Breaks at spaces when possible,
    /// falls back to hard break if a single word exceeds the width.
    fn word_wrap(text: &str, width: usize) -> Vec<String> {
        if width == 0 { return vec![text.to_string()]; }
        let mut lines = Vec::new();
        let mut remaining = text;
        while !remaining.is_empty() {
            let char_count = remaining.chars().count();
            if char_count <= width {
                lines.push(remaining.to_string());
                break;
            }
            // Find the byte index at `width` chars.
            let hard_end = remaining.char_indices()
                .nth(width)
                .map_or(remaining.len(), |(i, _)| i);
            // Look for the last space within the width.
            let break_at = remaining[..hard_end].rfind(' ')
                .unwrap_or(hard_end); // no space — hard break
            if break_at == 0 {
                // Edge case: space at position 0 or single huge word.
                let (line, rest) = remaining.split_at(hard_end);
                lines.push(line.to_string());
                remaining = rest;
            } else {
                let (line, rest) = remaining.split_at(break_at);
                lines.push(line.to_string());
                // Skip the space at the break point.
                remaining = rest.strip_prefix(' ').unwrap_or(rest);
            }
        }
        lines
    }

    // ── Mana coloring ─────────────────────────────────────────────

    /// Map a mana symbol character to its background color.
    fn mana_bg_color(ch: char) -> Option<Color> {
        match ch {
            'W' => Some(Color::AnsiValue(255)), // white
            'U' => Some(Color::AnsiValue(75)),  // blue
            'B' => Some(Color::AnsiValue(244)), // grey
            'R' => Some(Color::AnsiValue(203)), // salmon
            'G' => Some(Color::AnsiValue(71)),  // forest green
            _ => None,
        }
    }

    /// Map a basic land name to its mana background color.
    fn basic_land_bg(name: &str) -> Option<Color> {
        match name {
            "Plains" => Self::mana_bg_color('W'),
            "Island" => Self::mana_bg_color('U'),
            "Swamp" => Self::mana_bg_color('B'),
            "Mountain" => Self::mana_bg_color('R'),
            "Forest" => Self::mana_bg_color('G'),
            _ => None,
        }
    }

    /// Print a string to `out`, coloring mana symbols like {R}, {W}, etc.
    /// with colored backgrounds and black text.
    /// Non-mana text is printed with `default_color` (or reset if None).
    fn print_with_mana(out: &mut io::Stdout, text: &str, default_color: Option<Color>) {
        let mut chars = text.chars().peekable();
        let mut buf = String::new();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                let sym = chars.peek().copied();
                if let Some(s) = sym {
                    let mut lookahead = chars.clone();
                    lookahead.next();
                    if lookahead.peek() == Some(&'}') {
                        if let Some(bg) = Self::mana_bg_color(s) {
                            // Flush buffered text first
                            if !buf.is_empty() {
                                if let Some(c) = default_color {
                                    let _ = execute!(out, SetForegroundColor(c), Print(buf.as_str()), ResetColor);
                                } else {
                                    let _ = execute!(out, Print(buf.as_str()));
                                }
                                buf.clear();
                            }
                            // Reset all attributes before/after to avoid bold/dim bleeding into background
                            let _ = execute!(out, SetAttribute(Attribute::Reset),
                                SetBackgroundColor(bg), SetForegroundColor(Color::Black),
                                Print(format!("{{{s}}}")),
                                SetAttribute(Attribute::Reset));
                            chars.next(); // skip symbol
                            chars.next(); // skip '}'
                            continue;
                        }
                    }
                }
            }
            buf.push(ch);
        }
        // Flush remaining
        if !buf.is_empty() {
            if let Some(c) = default_color {
                let _ = execute!(out, SetForegroundColor(c), Print(buf.as_str()), ResetColor);
            } else {
                let _ = execute!(out, Print(buf.as_str()));
            }
        }
    }

    /// Print an action label, coloring mana symbols and basic land names.
    fn print_action_label(out: &mut io::Stdout, label: &str) {
        const BASIC_LANDS: &[(&str, char)] = &[
            ("Plains", 'W'), ("Island", 'U'), ("Swamp", 'B'),
            ("Mountain", 'R'), ("Forest", 'G'),
        ];
        // Check if any basic land name appears in the label.
        let mut colored = false;
        for &(land_name, mana_ch) in BASIC_LANDS {
            if let Some(pos) = label.find(land_name) {
                // Print prefix with mana coloring
                let prefix = &label[..pos];
                Self::print_with_mana(out, prefix, None);
                // Print land name with background
                if let Some(bg) = Self::mana_bg_color(mana_ch) {
                    let _ = execute!(out, SetBackgroundColor(bg), SetForegroundColor(Color::Black),
                        Print(land_name), SetAttribute(Attribute::Reset));
                }
                // Print suffix with mana coloring
                let suffix = &label[pos + land_name.len()..];
                Self::print_with_mana(out, suffix, None);
                colored = true;
                break;
            }
        }
        if !colored {
            Self::print_with_mana(out, label, None);
        }
    }

    // ── Rendering ──────────────────────────────────────────────────

    /// Draw a frame with no action menu.
    ///
    /// Deliberately menu-less: a caller that hands a menu to a renderer it
    /// cannot page from draws a live "m = next page" marker over a control
    /// it does not implement, which is how three target choosers came to
    /// advertise a key that answered "Invalid input 'm'" (issue #261). A
    /// menu goes through `render_paged`, which hands back how many rows it
    /// drew.
    fn render(view: &GameView, message: Option<&str>, log: &[String], card_filter: &str, pass_mode_label: Option<&str>) {
        let _ = Self::render_paged_noticed(view, None, message, None, log, card_filter, pass_mode_label, 0);
    }

    /// Blank the middle panel's part of one row, keeping the frame.
    ///
    /// On a pane too short for the whole frame the prompt block is anchored
    /// to the bottom and drawn OVER the board, so its rows can still hold
    /// what the board wrote there — a 20-row pane showed "0: Keep opening
    /// hand} 2/2" (issue #260).
    fn clear_mid_row(out: &mut io::Stdout, mid_col: u16, right_sep_col: u16,
                     has_right: bool, row: u16) {
        let _ = execute!(out, cursor::MoveTo(mid_col, row), Clear(ClearType::UntilNewLine));
        if has_right {
            let _ = execute!(out, cursor::MoveTo(right_sep_col, row),
                SetAttribute(Attribute::Dim), Print("│"), SetAttribute(Attribute::Reset));
        }
    }

    /// Which slice of a menu fits: `(offset, shown, paged)`.
    ///
    /// Pulled out of the pager so the arithmetic is testable without a
    /// terminal, and so the one prompt that could page and the ones that
    /// could not stop disagreeing about it (#96, #261).
    #[cfg(test)]
    fn menu_page(len: usize, avail: usize, offset: usize) -> (usize, usize, bool) {
        Self::menu_page_lines(&vec![1; len], avail, offset, 1)
    }


    /// `menu_page` for rows of uneven height: `heights[i]` is the number of
    /// lines row `i` takes once wrapped (issue #318). The page is as many
    /// whole rows from `offset` as fit in `avail` lines — always at least
    /// one, so a row taller than the pane still shows what it can rather
    /// than nothing — with `marker_h` lines kept for the "… showing" marker
    /// whenever the menu does not fit whole. The marker wraps like any other
    /// row, so how many lines it needs is the caller's to measure.
    fn menu_page_lines(heights: &[usize], avail: usize, offset: usize, marker_h: usize) -> (usize, usize, bool) {
        let len = heights.len();
        let offset = offset.min(len.saturating_sub(1));
        let remaining: usize = heights[offset..].iter().sum();
        let paged = offset > 0 || remaining > avail;
        let budget = if paged { avail.saturating_sub(marker_h).max(1) } else { avail };
        let mut shown = 0;
        let mut used = 0;
        for &h in &heights[offset..] {
            if shown > 0 && used + h > budget {
                break;
            }
            used += h;
            shown += 1;
        }
        (offset, shown, paged)
    }

    /// A prompt's heading, split into the label its rule carries and the
    /// question that goes under it.
    ///
    /// A horizontal rule is one line by construction: a rule wrapped over
    /// three lines put "Delver of Secrets: reveal Delver of Secrets from
    /// the top of your library? (not an instant or sorcery -- no
    /// transform)" through the middle of the frame, and the question a
    /// player has to read was spliced between two box rules. Nothing that
    /// has to be READ goes inside a rule now: the rule names the prompt and
    /// the words go below it, where they wrap like any other pane row.
    ///
    /// The label is the part before the first ": " when that is short
    /// enough to be a name — which is the convention the card prompts
    /// follow ("Civilized Scholar: choose a card to discard") — and the
    /// engine's own upper-case context lines ("MAIN PHASE 1", "DECLARE
    /// ATTACKERS") pass through whole. Anything else is labelled by kind,
    /// and the whole heading goes below.
    fn rule_title(message: &str, width: usize) -> (String, Option<String>) {
        let msg = message.trim();
        if str_cols(msg) <= width {
            return (msg.to_string(), None);
        }
        if let Some((head, rest)) = msg.split_once(": ") {
            let rest = rest.trim();
            if !head.is_empty() && str_cols(head) <= width && !rest.is_empty() {
                return (head.to_string(), Some(rest.to_string()));
            }
        }
        ("CHOICE".to_string(), Some(msg.to_string()))
    }

    /// A pane row too wide for the panel, laid out as several: the first
    /// line keeps the row's own leading indent and every continuation lines
    /// up under it.
    ///
    /// The chrome under a menu — the paging marker, the hint line — used to
    /// be cut at the panel edge (issue #53) or, in the combat panes, printed
    /// at full length straight over the border and into the card panel. A
    /// row of a menu has wrapped since issue #318; the rows that describe
    /// the menu wrap the same way.
    fn wrap_indented(text: &str, width: usize) -> Vec<String> {
        let trimmed = text.trim_start_matches(' ');
        let indent_n = text.chars().count() - trimmed.chars().count();
        let indent = " ".repeat(indent_n);
        Self::word_wrap(trimmed, width.saturating_sub(indent_n))
            .into_iter().map(|l| format!("{indent}{l}")).collect()
    }

    /// The "… showing a-b of 0-n" row, in one place: every pane that pages
    /// says the same thing, and `marker_lines` can measure it before the
    /// page that will be shown is known.
    fn page_marker(offset: usize, shown: usize, last: usize, keys: &str) -> String {
        format!("  \u{2026} showing {}-{} of 0-{} \u{2014} {keys}",
            offset, offset + shown.saturating_sub(1), last)
    }

    /// How many lines that row takes at its widest, so a page can reserve
    /// them before it knows which page it is.
    fn marker_lines(last: usize, keys: &str, width: usize) -> usize {
        Self::wrap_indented(&Self::page_marker(last, 1, last, keys), width).len()
    }

    /// What `m` does: the next page, wrapping to the top at the end. One
    /// definition, so every menu that draws the marker means the same thing
    /// by it.
    fn next_menu_offset(offset: usize, shown: usize, len: usize) -> usize {
        if offset + shown >= len { 0 } else { offset + shown }
    }

    /// What `p` does: the page that ends just above `offset` — as many
    /// whole rows as fit in the marker-less budget, walking back — or, from
    /// the top, the last page.
    ///
    /// Paging used to be forward-only, so overshooting a 253-name list meant
    /// pressing `m` eleven more times to come back around (issue #255). It
    /// then stepped back by a fixed page size, which rows of uneven height
    /// made a guess; measured in lines it is exact (issue #318).
    fn prev_menu_offset_lines(heights: &[usize], avail: usize, offset: usize, marker_h: usize) -> usize {

        let len = heights.len();
        if len == 0 {
            return 0;
        }
        let budget = avail.saturating_sub(marker_h).max(1);
        let end = if offset == 0 || offset > len { len } else { offset };
        let mut start = end;
        let mut used = 0;
        while start > 0 {
            let h = heights[start - 1];
            if start < end && used + h > budget {
                break;
            }
            used += h;
            start -= 1;
        }
        start
    }


    /// `render`, starting the action menu at `menu_offset` (issue #96 — a
    /// menu longer than the pane is paged with 'm', not guessed at).
    /// Returns the page it drew, so the caller can page from it exactly.
    fn render_paged(view: &GameView, actions: Option<&[MenuLabel]>, message: Option<&str>, log: &[String], card_filter: &str, pass_mode_label: Option<&str>, menu_offset: usize) -> MenuPage {
        Self::render_paged_noticed(view, actions, message, None, log, card_filter, pass_mode_label, menu_offset)
    }

    /// `render_paged` with a notice: one line of feedback about the last
    /// keystroke, drawn under the prompt's own heading rather than in place
    /// of it.
    ///
    /// The notice used to REPLACE the heading (`notice.or(context)`), so at
    /// a mandatory prompt — the one kind with no "pass" to fall back on —
    /// a refused keystroke took the question off the screen and left the
    /// player looking at "0: Yes / 1: No" with nothing saying what was
    /// being asked.
    #[allow(clippy::too_many_arguments)]
    fn render_paged_noticed(view: &GameView, actions: Option<&[MenuLabel]>, message: Option<&str>, notice: Option<&str>, log: &[String], card_filter: &str, pass_mode_label: Option<&str>, menu_offset: usize) -> MenuPage {

        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

        let (term_w, term_h) = terminal::size().unwrap_or((100, 30));
        let w = term_w as usize;
        let h = term_h as usize;

        // 3-column layout: left (stack+log), middle (game), right (card reference)
        let has_right = w >= 100;
        let gutter_w: usize = w / 5; // 20% each gutter
        let left_w: usize = gutter_w;
        let right_w: usize = if has_right { gutter_w } else { 0 };
        let mid_w = w.saturating_sub(left_w + right_w + if has_right { 2 } else { 1 });
        let mid_col = u16::try_from(left_w + 1).unwrap_or(u16::MAX);
        let right_sep_col = u16::try_from(left_w + 1 + mid_w).unwrap_or(u16::MAX);
        let right_col = if has_right { right_sep_col + 1 } else { 0 };

        // ── Draw vertical separators ──
        for r in 0..h {
            let _ = execute!(out, cursor::MoveTo(u16::try_from(left_w).unwrap_or(u16::MAX), u16::try_from(r).unwrap_or(u16::MAX)),
                SetAttribute(Attribute::Dim), Print("│"), SetAttribute(Attribute::Reset));
        }
        if has_right {
            for r in 0..h {
                let _ = execute!(out, cursor::MoveTo(right_sep_col, u16::try_from(r).unwrap_or(u16::MAX)),
                    SetAttribute(Attribute::Dim), Print("│"), SetAttribute(Attribute::Reset));
            }
        }

        // ── Left panel: STACK (top 1/3) + LOG (bottom 2/3) ──
        let stack_h = h / 3;
        let log_start = stack_h;

        // Stack
        let stack_label = "─── STACK ";
        let stack_line = format!("{}{}", stack_label, "─".repeat(left_w.saturating_sub(stack_label.chars().count())));
        let _ = execute!(out, cursor::MoveTo(0, 0),
            SetAttribute(Attribute::Dim), Print(&stack_line), SetAttribute(Attribute::Reset));
        let _ = execute!(out, cursor::MoveTo(u16::try_from(left_w).unwrap_or(u16::MAX), 0),
            SetAttribute(Attribute::Dim), Print("┤"), SetAttribute(Attribute::Reset));
        if view.stack.is_empty() {
            let _ = execute!(out, cursor::MoveTo(1, 1),
                SetAttribute(Attribute::Dim), Print("(empty)"), SetAttribute(Attribute::Reset));
        } else {
            // One row is held back for the "N more" marker, so a stack the
            // panel cannot hold says so rather than looking short: it used to
            // render until it ran out of rows and stop, cutting the last
            // entry mid-entry, with 44 objects off screen and nothing saying
            // there were any (issue #247).
            let max_w = left_w.saturating_sub(1);
            let body_h = stack_h.saturating_sub(1);
            let mut srow: u16 = 1;
            let mut shown = 0usize;
            for item in &view.stack {
                if srow as usize >= body_h { break; }
                let text = Self::stack_entry_headline(view, item);
                let mut fitted = true;
                for line in Self::word_wrap(&text, max_w) {
                    if srow as usize >= body_h { fitted = false; break; }
                    let _ = execute!(out, cursor::MoveTo(1, srow), Print(&line));
                    srow += 1;
                }
                for target in &item.targets {
                    if srow as usize >= body_h { fitted = false; break; }
                    let line = clip_cols(&Self::stack_target_line(view, target), max_w);
                    let _ = execute!(out, cursor::MoveTo(1, srow),
                        SetAttribute(Attribute::Dim), Print(&line), SetAttribute(Attribute::Reset));
                    srow += 1;
                }
                if !fitted { break; }
                shown += 1;
            }
            if shown < view.stack.len() {
                let marker = clip_cols(
                    &format!(" … {} more — s to see them all", view.stack.len() - shown), max_w);
                let _ = execute!(out,
                    cursor::MoveTo(1, u16::try_from(body_h).unwrap_or(u16::MAX)),
                    SetAttribute(Attribute::Dim), Print(marker), SetAttribute(Attribute::Reset));
            }
        }

        // Log separator with label
        let log_label = "─── LOG ";
        let log_line = format!("{}{}", log_label, "─".repeat(left_w.saturating_sub(log_label.chars().count())));
        let _ = execute!(out, cursor::MoveTo(0, u16::try_from(log_start).unwrap_or(u16::MAX)),
            SetAttribute(Attribute::Dim), Print(&log_line), SetAttribute(Attribute::Reset));
        let _ = execute!(out, cursor::MoveTo(u16::try_from(left_w).unwrap_or(u16::MAX), u16::try_from(log_start).unwrap_or(u16::MAX)),
            SetAttribute(Attribute::Dim), Print("┤"), SetAttribute(Attribute::Reset));
        if !log.is_empty() {
            let log_visible = h.saturating_sub(log_start + 2);
            let max_chars = left_w.saturating_sub(1);
            // Wrap log entries that are too long for the panel.
            // Continuation lines get a 2-space indent.
            let mut wrapped: Vec<String> = Vec::new();
            let indent = "  ";
            let cont_max = max_chars.saturating_sub(indent.len());
            for entry in log {
                if entry.chars().count() <= max_chars {
                    wrapped.push(entry.clone());
                } else {
                    let lines = Self::word_wrap(entry, max_chars);
                    for (i, line) in lines.into_iter().enumerate() {
                        if i == 0 {
                            wrapped.push(line);
                        } else {
                            // Re-wrap the continuation line at the narrower indent width.
                            let sub_lines = Self::word_wrap(&line, cont_max);
                            for sub in sub_lines {
                                wrapped.push(format!("{indent}{sub}"));
                            }
                        }
                    }
                }
            }
            let start = if wrapped.len() > log_visible { wrapped.len() - log_visible } else { 0 };
            for (i, line) in wrapped[start..].iter().enumerate() {
                let r = u16::try_from(log_start + 1 + i).unwrap_or(u16::MAX);
                if r >= term_h - 1 { break; }
                let _ = execute!(out, cursor::MoveTo(1, r),
                    SetAttribute(Attribute::Dim), Print(line), SetAttribute(Attribute::Reset));
            }
        }

        // ── Middle panel: main game ──
        let mut row: u16 = 0;
        // Readable step name
        let step_name = match view.step {
            Step::Untap => "Untap",
            Step::Upkeep => "Upkeep",
            Step::Draw => "Draw",
            Step::PrecombatMain => "Main Phase 1",
            Step::BeginCombat => "Begin Combat",
            Step::DeclareAttackers => "Declare Attackers",
            Step::DeclareBlockers => "Declare Blockers",
            // Two damage steps rendered identically with first strikers in
            // combat (issue #140, CR 510.4).
            Step::CombatDamage if view.first_strike_damage_step => "First-Strike Combat Damage",
            Step::CombatDamage => "Combat Damage",
            Step::EndCombat => "End Combat",
            Step::PostcombatMain => "Main Phase 2",
            Step::EndStep => "End Step",
            Step::Cleanup => "Cleanup (Discard to 7)",
        };

        // Turn/phase bar. The seat is named in the log's p0/p1 scheme —
        // nothing else told a hotseat player which seat was being prompted,
        // or (before the first keep/mulligan) who is on the play (#115).
        let whose_turn = if view.active_player == view.you { "Your turn" } else { "Opponent's turn" };
        let on_play = if view.turn_number == 1 {
            if view.active_player == view.you { ", on the play" } else { ", on the draw" }
        } else {
            ""
        };
        let pass_label = pass_mode_label.map(|l| format!(" [{l}]")).unwrap_or_default();
        let status = format!(" Turn {} - {} | {} (you are p{}{})",
            view.turn_number, step_name, whose_turn, view.you.0, on_play);
        let _ = execute!(out, cursor::MoveTo(mid_col, row),
            SetAttribute(Attribute::Bold), Print(&status), SetAttribute(Attribute::Reset));
        if !pass_label.is_empty() {
            let _ = execute!(out, SetForegroundColor(Color::Yellow), Print(&pass_label), ResetColor);
        }
        row += 1;

        // Compute stats
        let your_gy: usize = view.graveyards.iter()
            .filter(|(pid, _)| *pid == view.you)
            .map(|(_, cards)| cards.len()).sum();
        let your_exile: usize = view.exile.iter()
            .filter(|c| c.owner == view.you).count();
        let opp_gy: usize = view.graveyards.iter()
            .filter(|(pid, _)| *pid != view.you)
            .map(|(_, cards)| cards.len()).sum();
        let opp_exile: usize = view.exile.iter()
            .filter(|c| c.owner != view.you).count();

        let is_your_turn = view.active_player == view.you;
        let your_caret = if is_your_turn { "▸ " } else { "  " };
        let opp_caret = if is_your_turn { "  " } else { "▸ " };

        let your_stats = format!("{}You: {}hp  {}lib  {}gy  {}ex  {}hand",
            your_caret, view.your_life, view.your_library_size, your_gy, your_exile, view.your_hand.len());
        let opp_stats = view.opponents.first().map(|opp|
            format!("{}Opp: {}hp  {}lib  {}gy  {}ex  {}hand",
                opp_caret, opp.life, opp.library_size, opp_gy, opp_exile, opp.hand_size)
        ).unwrap_or_default();

        // ── BATTLEFIELD section (combined) ──
        let bf_label = "─── BATTLEFIELD ";
        let bf_line = format!("{}{}", bf_label, "─".repeat(mid_w.saturating_sub(bf_label.chars().count())));
        let _ = execute!(out, cursor::MoveTo(mid_col, row),
            SetAttribute(Attribute::Dim), Print(&bf_line), SetAttribute(Attribute::Reset));
        let _ = execute!(out, cursor::MoveTo(u16::try_from(left_w).unwrap_or(u16::MAX), row),
            SetAttribute(Attribute::Dim), Print("├"), SetAttribute(Attribute::Reset));
        row += 1;

        // Opponent status line
        let _ = execute!(out, cursor::MoveTo(mid_col, row));
        if is_your_turn {
            let _ = execute!(out, SetForegroundColor(Color::Red));
        } else {
            let _ = execute!(out, SetForegroundColor(Color::Red), SetAttribute(Attribute::Bold));
        }
        let _ = execute!(out, Print(&opp_stats));
        let _ = execute!(out, SetAttribute(Attribute::Reset), ResetColor);
        row += 1;

        // Opponent board
        let bf_content_start = row;
        let opp_perms: Vec<&PermanentView> = view.battlefield.iter()
            .filter(|p| p.controller != view.you).collect();
        row = Self::render_battlefield_at(&mut out, &opp_perms, Color::Red, mid_col, row, mid_w, &view.battlefield, false, view.you);
        let opp_rows = row - bf_content_start;

        // Your board (measure first to calculate padding)
        let your_perms: Vec<&PermanentView> = view.battlefield.iter()
            .filter(|p| p.controller == view.you).collect();
        // Count how many rows your side will take (lands + creatures + artifacts + enchantments)
        let your_row_count = {
            let has_type = |p: &&PermanentView, t: CardType| p.card_types.contains(&t);
            let lands: Vec<_> = your_perms.iter().filter(|p| has_type(p, CardType::Land)).collect();
            let creatures: Vec<_> = your_perms.iter().filter(|p| has_type(p, CardType::Creature)).collect();
            let other: usize = your_perms.iter().filter(|p|
                !has_type(p, CardType::Land) && !has_type(p, CardType::Creature)).count();
            usize::from(!lands.is_empty()) + creatures.len() + other
        };

        // Pad the divider area so the total battlefield is at least 9 lines
        // (opp_status + opp_board + divider + your_board + your_status = content)
        let min_bf_height: u16 = 9;
        let content_rows = 2 + opp_rows + u16::try_from(your_row_count).unwrap_or(u16::MAX); // 2 for status lines
        let padding = if content_rows + 1 < min_bf_height {
            min_bf_height - content_rows
        } else {
            1 // at least 1 line for the divider
        };

        // Draw divider with padding
        let divider_mid = row + padding / 2;
        let dots = "· · ·";
        let dots_pad = mid_w.saturating_sub(dots.chars().count()) / 2;
        let _ = execute!(out, cursor::MoveTo(mid_col + u16::try_from(dots_pad).unwrap_or(u16::MAX), divider_mid),
            SetAttribute(Attribute::Dim), Print(dots), SetAttribute(Attribute::Reset));
        row += padding;

        // Your board
        row = Self::render_battlefield_at(&mut out, &your_perms, Color::Green, mid_col, row, mid_w, &view.battlefield, true, view.you);

        // Your status line
        let _ = execute!(out, cursor::MoveTo(mid_col, row));
        if is_your_turn {
            let _ = execute!(out, SetForegroundColor(Color::Green), SetAttribute(Attribute::Bold));
        } else {
            let _ = execute!(out, SetForegroundColor(Color::Green));
        }
        let _ = execute!(out, Print(&your_stats));
        let _ = execute!(out, SetAttribute(Attribute::Reset), ResetColor);
        row += 1;

        // Hand separator — starts at mid_col, spans to right edge
        let hand_label = "─── HAND ";
        let hand_line = format!("{}{}", hand_label, "─".repeat(mid_w.saturating_sub(hand_label.chars().count())));
        let _ = execute!(out, cursor::MoveTo(mid_col, row),
            SetAttribute(Attribute::Dim), Print(&hand_line), SetAttribute(Attribute::Reset));
        let _ = execute!(out, cursor::MoveTo(u16::try_from(left_w).unwrap_or(u16::MAX), row),
            SetAttribute(Attribute::Dim), Print("├"), SetAttribute(Attribute::Reset));
        row += 1;

        // Hand
        if view.your_hand.is_empty() {
            Self::mid_print(&mut out, mid_col, &mut row, mid_w, "  (empty)", None, false);
        } else {
            for card in &view.your_hand {
                let cost = card.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
                let pt = match (card.power, card.toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                if let Some(bg) = Self::basic_land_bg(&card.name) {
                    let _ = execute!(out, cursor::MoveTo(mid_col, row), Print("  "),
                        SetBackgroundColor(bg), SetForegroundColor(Color::Black),
                        Print(&card.name), ResetColor);
                    row += 1;
                } else {
                    Self::mid_print(&mut out, mid_col, &mut row, mid_w,
                        &format!("  {}{}{}", card.name, cost, pt), None, false);
                }
            }
        }

        // Mana pool at bottom of hand area
        if !view.your_mana_pool.is_empty() {
            let mana_str: Vec<String> = view.your_mana_pool.mana.iter()
                .filter(|(_, &v)| v > 0)
                .map(|(t, v)| format!("{t:?}:{v}"))
                .collect();
            Self::mid_print(&mut out, mid_col, &mut row, mid_w,
                &format!("  Mana: {}", mana_str.join(" ")), Some(Color::Yellow), false);
        }

        // The prompt block — the separator that carries the notice, the menu,
        // its truncation marker, the hint line and the input row — is the
        // part of the frame the player is typing INTO, so it is the last
        // thing a short pane sacrifices, not the first. It used to be drawn
        // wherever the board happened to end, so below ~24 rows the options,
        // the "… showing" marker and every error notice were pushed off the
        // bottom and the input prompt was painted over the hint line: a
        // prompt with no visible options, no error feedback and no statement
        // that anything was hidden, which is the "indistinguishable from a
        // hung game" symptom #76 exists to prevent (issue #260). Anchor it
        // to the bottom and let the board scroll off above instead.
        {
            // One row for the rule, whatever the heading says, plus the
            // rows the question under it takes.
            let title_rows = message.map_or(1, |msg| {
                let (_, detail) = Self::rule_title(msg, mid_w.saturating_sub(6));
                1 + detail.map_or(0, |d| Self::wrap_indented(&format!("  {d}"), mid_w).len())
            }) + notice.map_or(0, |n| Self::wrap_indented(&format!("  {n}"), mid_w).len());
            // hint row + input row, and for a menu one option and its marker.
            let furniture = if actions.is_some() { 2 } else { 1 };
            let menu_floor = if actions.is_some() { 2 } else { 0 };
            let min_block = title_rows + menu_floor + furniture;
            if h.saturating_sub(row as usize) < min_block {
                row = u16::try_from(h.saturating_sub(min_block)).unwrap_or(0);
            }
        }

        // The actions separator. A rule is ONE line: it carries the
        // prompt's label and nothing that has to be read, and the question
        // itself goes on its own rows underneath, where it wraps like any
        // other pane row. A rule that wrapped put the box borders through
        // the middle of the sentence a player was trying to read (#121 is
        // the version of this that only moved the tee).
        if let Some(msg) = message {
            let prefix = "─── ";
            let prefix_len = prefix.chars().count(); // 4
            // Leave room for the prefix + trailing space + at least 1 dash
            let text_w = mid_w.saturating_sub(prefix_len + 2);
            let (title, detail) = Self::rule_title(msg, text_w);
            let label = format!("{prefix}{title} ");
            let full = format!("{}{}", label, "─".repeat(mid_w.saturating_sub(str_cols(&label))));
            let _ = execute!(out, cursor::MoveTo(mid_col, row),
                SetAttribute(Attribute::Dim), Print(&full), SetAttribute(Attribute::Reset));
            let _ = execute!(out, cursor::MoveTo(u16::try_from(left_w).unwrap_or(u16::MAX), row),
                SetAttribute(Attribute::Dim), Print("├"), SetAttribute(Attribute::Reset));
            if has_right {
                let _ = execute!(out, cursor::MoveTo(right_sep_col, row),
                    SetAttribute(Attribute::Dim), Print("┤"), SetAttribute(Attribute::Reset));
            }
            row += 1;
            if let Some(detail) = detail {
                for line in Self::wrap_indented(&format!("  {detail}"), mid_w) {
                    if row as usize >= h { break; }
                    Self::clear_mid_row(&mut out, mid_col, right_sep_col, has_right, row);
                    let _ = execute!(out, cursor::MoveTo(mid_col, row), Print(&line));
                    row += 1;
                }
            }
            for line in notice.map(|n| Self::wrap_indented(&format!("  {n}"), mid_w)).unwrap_or_default() {
                if row as usize >= h { break; }
                Self::clear_mid_row(&mut out, mid_col, right_sep_col, has_right, row);
                let _ = execute!(out, cursor::MoveTo(mid_col, row),
                    SetForegroundColor(Color::Red), Print(&line), ResetColor);
                row += 1;
            }
        } else {
            let action_line = "─".repeat(mid_w);
            let _ = execute!(out, cursor::MoveTo(mid_col, row),
                SetAttribute(Attribute::Dim), Print(&action_line), SetAttribute(Attribute::Reset));
            let _ = execute!(out, cursor::MoveTo(u16::try_from(left_w).unwrap_or(u16::MAX), row),
                SetAttribute(Attribute::Dim), Print("├"), SetAttribute(Attribute::Reset));
            if has_right {
                let _ = execute!(out, cursor::MoveTo(right_sep_col, row),
                    SetAttribute(Attribute::Dim), Print("┤"), SetAttribute(Attribute::Reset));
            }
            row += 1;
        }

        // Action list (only when actions are provided)
        let mut page = MenuPage::default();
        if let Some(labels) = actions {
            // Lines left for the menu once the hint and prompt rows below it
            // are reserved. A menu longer than the pane used to keep printing
            // past the bottom — 11 of 35 mulligan-bottom options were simply
            // invisible (#60) — and the marker #60 added still left hidden
            // entries reachable only by typing a number the player could not
            // see, which mis-cast a spell in a real game (#96). The menu now
            // renders a page starting at `menu_offset`, advanced with 'm';
            // indices are absolute, so any number works from any page.
            // Two rows below the menu are the hint line and the input row.
            // Two kinds of row sit under the menu, and both wrap rather
            // than being cut: the hint line, whose height is known now, and
            // the paging marker, whose height is the same whichever page it
            // ends up describing. Under them is the input row.
            let hints = Self::menu_hints(labels, has_right);
            let hint_lines = Self::wrap_indented(hints, mid_w);
            let marker_h = Self::marker_lines(
                labels.len().saturating_sub(1), MENU_PAGE_KEYS, mid_w);
            let avail = h.saturating_sub(row as usize + hint_lines.len() + 1);
            // A row that does not fit the pane wraps under a hanging indent;
            // nothing on it is cut. Every way of clipping a row lost the
            // part of it a real game needed — most recently the third card
            // of a "Bottom A, B, C" row, so 14 of 25 bottoming options
            // printed as 5 identical lines at 100 columns (issue #318). The
            // index column is as wide as the widest index, so the text of
            // every row, and every continuation line, starts in the same
            // column.
            let idx_w = labels.len().saturating_sub(1).to_string().chars().count();
            let plen = 4 + idx_w;
            let rows = Self::wrap_menu_rows(labels, mid_w.saturating_sub(plen));
            let heights: Vec<usize> = rows.iter().map(|r| r.len().max(1)).collect();
            let (offset, shown, paged) = Self::menu_page_lines(&heights, avail, menu_offset, marker_h);
            page = MenuPage { offset, shown, avail, marker_h, heights };
            let indent = " ".repeat(plen);
            'rows: for (i, lines) in rows.iter().enumerate().skip(offset).take(shown) {
                for (k, line) in lines.iter().enumerate() {
                    if row as usize >= h { break 'rows; }
                    Self::clear_mid_row(&mut out, mid_col, right_sep_col, has_right, row);
                    let _ = execute!(out, cursor::MoveTo(mid_col, row));
                    if k == 0 {
                        let _ = execute!(out,
                            SetAttribute(Attribute::Bold), Print(format!("  {i:>idx_w$}")),
                            SetAttribute(Attribute::Reset), Print(": "));
                    } else {
                        let _ = execute!(out, Print(&indent));
                    }
                    Self::print_action_label(&mut out, line);
                    row += 1;
                }
            }
            // The marker is the LAST row sacrificed, not the first: a menu
            // that does not fit has to say so, or the pane reads as a game
            // that has stopped asking (issue #260).
            if paged {
                let marker = Self::page_marker(offset, shown, labels.len() - 1, MENU_PAGE_KEYS);
                for line in Self::wrap_indented(&marker, mid_w) {
                    if row as usize >= h { break; }
                    Self::clear_mid_row(&mut out, mid_col, right_sep_col, has_right, row);
                    let _ = execute!(out, cursor::MoveTo(mid_col, row),
                        SetAttribute(Attribute::Dim), Print(&line),
                        SetAttribute(Attribute::Reset));
                    row += 1;
                }
            }

            // Kept inside the panel like every other row — at full length
            // this ate the right border and the card panel behind it (#53) —
            // but wrapped rather than cut, so the last pane key is still
            // legible at 100 columns (issue #318).
            for line in &hint_lines {
                if row as usize >= h { break; }
                Self::clear_mid_row(&mut out, mid_col, right_sep_col, has_right, row);
                let _ = execute!(out, cursor::MoveTo(mid_col, row),
                    SetAttribute(Attribute::Dim), Print(line),
                    SetAttribute(Attribute::Reset));
                row += 1;
            }
        }

        // ── Right panel: card reference ──
        if has_right {
            let registry = mtg_engine::cards::CardRegistry::with_all_cards();
            let card_refs = Self::build_card_refs(view, &registry, card_filter);
            Self::render_right_panel(&mut out, &card_refs, right_col, right_w, h, card_filter);
        }

        // Print prompt and move cursor to input area in middle panel. The row
        // is cleared first: on a pane too short for the whole frame the menu
        // block is anchored to the bottom and drawn OVER the board, so the
        // prompt row can still hold whatever the board wrote there — the
        // reported 16-row pane showed "  > om Blade {1}{B}res {W}2"
        // (issue #260).
        Self::clear_mid_row(&mut out, mid_col, right_sep_col, has_right, row);
        let _ = execute!(out, cursor::MoveTo(mid_col, row), Print("  > "));
        let _ = out.flush();
        page
    }


    /// Display name for a counter kind, as it reads on a battlefield line.
    fn counter_display_name(ct: mtg_engine::types::CounterType) -> &'static str {
        ct.label()
    }

    /// " {2 hatchling, 3 +1/+1}" for a permanent's non-loyalty counters, or
    /// "" when it has none. Counters are public information (CR 122.3) and
    /// were invisible everywhere in the CLI except the log line that added
    /// them (issue #82); loyalty stays with the planeswalker line's own
    /// `[N loyalty]` rendering (#58).
    /// Join a battlefield row's three parts, shortening the elastic middle
    /// rather than the tail when the row does not fit.
    ///
    /// `head` (name, P/T, counters) and `tail` (tap / sickness / damage
    /// flags) are what the row is read for; the attachment and keyword list
    /// in between is the part that grows without bound. Truncating the whole
    /// string dropped the tail first.
    fn elide_middle(head: &str, elastic: &str, tail: &str, max_w: usize) -> String {
        // `max_w` is the panel's budget; leave room for the "Nx " prefix a
        // collapsed row adds.
        let budget = max_w.saturating_sub(4);
        let fixed = head.chars().count() + tail.chars().count();
        if fixed + elastic.chars().count() <= budget {
            return format!("{head}{elastic}{tail}");
        }
        let room = budget.saturating_sub(fixed);
        if room <= 1 {
            return format!("{head}{tail}");
        }
        let kept: String = elastic.chars().take(room - 1).collect();
        format!("{head}{kept}…{tail}")
    }

    /// Whether `[S]` means anything for this permanent.
    ///
    /// Summoning sickness restricts a *creature*'s attacks and `{T}`
    /// abilities (CR 302.6) and nothing else, so the raw object flag is
    /// meaningless on a planeswalker or an enchantment that has just
    /// resolved (issue #221) and misleading on a creature with haste (#139).
    /// One helper, so every pane answers it the same way — the battlefield
    /// row learned the haste half and the inspector never did.
    fn is_summoning_sick(perm: &PermanentView) -> bool {
        perm.summoning_sick
            && perm.card_types.contains(&CardType::Creature)
            && !perm.keywords.contains(&mtg_engine::types::Keyword::Haste)
    }

    fn counters_suffix(counters: &HashMap<mtg_engine::types::CounterType, u32>) -> String {
        let mut parts: Vec<String> = counters.iter()
            .filter(|&(ct, n)| *ct != mtg_engine::types::CounterType::Loyalty && *n > 0)
            .map(|(ct, n)| format!("{n} {}", Self::counter_display_name(*ct)))
            .collect();
        if parts.is_empty() {
            return String::new();
        }
        parts.sort();
        format!(" {{{}}}", parts.join(", "))
    }

    /// Render battlefield permanents at a specific column/row, return next row.
    fn render_battlefield_at(out: &mut io::Stdout, perms: &[&PermanentView], color: Color,
                              col: u16, mut row: u16, max_w: usize,
                              all_perms: &[PermanentView], lands_last: bool,
                              view_you: mtg_engine::ids::PlayerId) -> u16 {
        let has_type = |p: &&PermanentView, t: CardType| p.card_types.contains(&t);
        let lands: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Land)).collect();
        let creatures: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Creature)).collect();
        let enchantments: Vec<_> = perms.iter().filter(|p|
            has_type(p, CardType::Enchantment) && !has_type(p, CardType::Creature)).collect();
        let artifacts: Vec<_> = perms.iter().filter(|p|
            has_type(p, CardType::Artifact) && !has_type(p, CardType::Creature) && !has_type(p, CardType::Land)).collect();
        // Planeswalkers get their own bucket — any permanent type outside the
        // buckets above simply vanished from the panel, and loyalty (their
        // defining public state, CR 306.5b) was shown nowhere (#58).
        let planeswalkers: Vec<_> = perms.iter().filter(|p|
            has_type(p, CardType::Planeswalker) && !has_type(p, CardType::Creature)).collect();

        // Build the attachment map from ALL permanents (auras can be
        // controlled by a different player than the creature they're
        // attached to, e.g. Pacifism). Equipment counts too: a creature's
        // line shows everything on it, aura or Pike — filtering to
        // enchantments left Equipment invisible outside the inspector
        // (issue #83, CR 301.5c).
        let mut aura_map: HashMap<ObjectId, Vec<String>> = HashMap::new();
        for p in all_perms {
            if let Some(target_id) = p.attached_to {
                aura_map.entry(target_id).or_default().push(p.name.clone());
            }
        }

        // Helper: render the lands summary line
        let render_lands = |out: &mut io::Stdout, row: &mut u16| {
            if !lands.is_empty() {
                let mut summary: Vec<(String, usize, usize)> = Vec::new();
                for land in &lands {
                    if let Some(entry) = summary.iter_mut().find(|(n, _, _)| *n == land.name) {
                        if land.tapped { entry.2 += 1; } else { entry.1 += 1; }
                    } else {
                        let (u, t) = if land.tapped { (0, 1) } else { (1, 0) };
                        summary.push((land.name.clone(), u, t));
                    }
                }
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(color), Print("  Lands: "), ResetColor);
                // Every other row in this panel is clipped to `max_w`; this
                // one was printed at full length and wrote straight over the
                // CARDS column, wrapping and displacing the whole frame once
                // a real nonbasic mana base was on the battlefield (issue
                // #244). The entries are coloured individually, so the budget
                // is spent entry by entry rather than clipping one string.
                let mut spent = "  Lands: ".chars().count();
                for (i, (name, untapped, tapped)) in summary.iter().enumerate() {
                    let total = untapped + tapped;
                    let suffix = if *tapped > 0 && *untapped > 0 {
                        format!(" ({tapped} tapped)")
                    } else if *tapped > 0 {
                        " (tapped)".to_string()
                    } else {
                        String::new()
                    };
                    let sep = if i > 0 { ", " } else { "" };
                    let entry_len = sep.chars().count()
                        + format!("{total}x ").chars().count()
                        + name.chars().count()
                        + suffix.chars().count();
                    if spent + entry_len > max_w {
                        let left = summary.len() - i;
                        let more = format!("{sep}+{left} more");
                        if spent + more.chars().count() <= max_w {
                            let _ = execute!(out, SetForegroundColor(color), Print(more), ResetColor);
                        }
                        break;
                    }
                    spent += entry_len;
                    if i > 0 {
                        let _ = execute!(out, SetForegroundColor(color), Print(", "), ResetColor);
                    }
                    let _ = execute!(out, SetForegroundColor(color), Print(format!("{total}x ")), ResetColor);
                    if let Some(bg) = CliPlayer::basic_land_bg(name) {
                        let _ = execute!(out, SetBackgroundColor(bg), SetForegroundColor(Color::Black),
                            Print(name), ResetColor);
                    } else {
                        let _ = execute!(out, SetForegroundColor(color), Print(name), ResetColor);
                    }
                    if !suffix.is_empty() {
                        let _ = execute!(out, SetForegroundColor(color), Print(suffix), ResetColor);
                    }
                }
                *row += 1;
            }
        };

        // Rows whose every visible detail matches are one line with a count
        // (`63x Zombie Token 2/2`), exactly as the lands summary already
        // does: an Endless Ranks of the Dead board grew one row per token
        // and pushed the hand, the action list, and the prompt clean off
        // the pane (issue #74). Order is first-appearance, and any visible
        // difference — P/T, an aura, damage, tapped/sick flags — keeps its
        // own row.
        let collapse = |labels: Vec<String>| -> Vec<(usize, String)> {
            let mut counted: Vec<(usize, String)> = Vec::new();
            for label in labels {
                match counted.iter_mut().find(|(_, l)| *l == label) {
                    Some((n, _)) => *n += 1,
                    None => counted.push((1, label)),
                }
            }
            counted
        };
        let counted_line = |n: usize, label: &str| -> String {
            if n > 1 { format!("  {n}x {label}") } else { format!("  {label}") }
        };

        // Helper: render creatures, enchantments, artifacts
        let render_nonlands = |out: &mut io::Stdout, row: &mut u16| {
            let creature_labels = creatures.iter()
                .map(|c| Self::creature_row_parts(c, aura_map.get(&c.object_id)))
                .map(|(head, elastic, flags)| Self::elide_middle(&head, &elastic, &flags, max_w))
                .collect();

            for (n, label) in collapse(creature_labels) {
                let truncated: String = counted_line(n, &label).chars().take(max_w).collect();
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(color), Print(&truncated), ResetColor);
                *row += 1;
            }
            let enchantment_labels = enchantments.iter()
                .filter(|e| e.attached_to.is_none())
                .map(|e| {
                    // A Curse's entire identity is whom it enchants
                    // (CR 702.5c) — without this, two curses on opposite
                    // players rendered identically (issue #81).
                    let host = match e.attached_to_player {
                        Some(p) if p == view_you => " [enchanting you]".to_string(),
                        Some(_) => " [enchanting opponent]".to_string(),
                        None => String::new(),
                    };
                    // The chosen name is the permanent's whole identity
                    // (Nevermore) and is public information (issue #130).
                    let named = e.named_card.as_ref()
                        .map(|n| format!(" [names: {n}]"))
                        .unwrap_or_default();
                    format!("{}{}{}{}{}", e.name, Self::legend_mark(e), host, named,
                        CliPlayer::counters_suffix(&e.counters))
                })
                .collect();

            for (n, label) in collapse(enchantment_labels) {
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(Color::Magenta), Print(counted_line(n, &label)), ResetColor);
                *row += 1;
            }
            // Attached Equipment rides on its creature's line (above), like
            // attached auras — not in the standalone artifact list.
            let artifact_labels = artifacts.iter()
                .filter(|a| a.attached_to.is_none())
                .map(|a| format!("{}{}{}{}", a.name, Self::legend_mark(a),
                    CliPlayer::counters_suffix(&a.counters),
                    if a.tapped { " [T]" } else { "" }))
                .collect();

            for (n, label) in collapse(artifact_labels) {
                let _ = execute!(out, cursor::MoveTo(col, *row), Print(counted_line(n, &label)));
                *row += 1;
            }
            for pw in &planeswalkers {
                let loyalty = pw.counters.get(&mtg_engine::types::CounterType::Loyalty)
                    .copied().unwrap_or(0);
                let dmg = if pw.damage_marked > 0 { format!(" ({}d)", pw.damage_marked) } else { String::new() };
                let text = format!("  {}{} [{loyalty} loyalty]{dmg}", pw.name, Self::legend_mark(pw));

                let truncated: String = text.chars().take(max_w).collect();
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(Color::Cyan), Print(&truncated), ResetColor);
                *row += 1;
            }
        };

        if lands_last {
            render_nonlands(out, &mut row);
            render_lands(out, &mut row);
        } else {
            render_lands(out, &mut row);
            render_nonlands(out, &mut row);
        }

        row
    }

    // (Old render_battlefield removed — replaced by render_battlefield_at)

    /// One creature's battlefield row, in the three regions `elide_middle`
    /// treats differently: `(head, elastic, flags)`. Pure, so the row is
    /// testable without a terminal.
    ///
    /// `auras` are the names of everything attached to it.
    fn creature_row_parts(c: &PermanentView, auras: Option<&Vec<String>>) -> (String, String, String) {
        let pt = match (c.effective_power, c.effective_toughness) {
            (Some(p), Some(t)) => format!(" {p}/{t}"),
            _ => match (c.power, c.toughness) {
                (Some(p), Some(t)) => format!(" {p}/{t}"),
                _ => String::new(),
            },
        };
        let auras = auras
            .map(|names| format!(" [{}]", names.join(",")))
            .unwrap_or_default();
        let dmg = if c.damage_marked > 0 { format!(" ({}d)", c.damage_marked) } else { String::new() };
        // A hasty creature isn't slowed by summoning sickness —
        // '[S]' read as "cannot attack" on a creature whose attack
        // was perfectly legal (issue #139).
        let sick = Self::is_summoning_sick(c);
        // CR 506.3a/509.1a: attacking and blocking are public state,
        // and `[T]` — the same mark a creature gets for tapping for
        // mana — was the only thing the pane said about either
        // (issue #245).
        let combat = if c.attacking.is_some() {
            " [ATK]"
        } else if !c.blocking.is_empty() {
            " [BLK]"
        } else {
            ""
        };
        // CR 111.4 leaves the word "Token" out of a token's name, so
        // the pane says it here instead — otherwise a Spirit token and
        // a card named Spirit render identically, and the CARDS pane
        // (which excludes tokens) is the only thing that tells them
        // apart (issues #331, #334).
        let flags = format!("{}{}{}{}{}",
            if c.is_token { " [tok]" } else { "" },
            if c.tapped { " [T]" } else { "" },
            if sick { " [S]" } else { "" },
            combat,
            dmg);
        // The permanent's live keywords and protections. A flying
        // token rendered exactly like a ground creature, and a
        // creature that had lost defender still read "Defender" from
        // its printed card — the block decision is made off this line
        // (issue #243). "legendary" leads the list: it is the supertype
        // that arms the legend rule (CR 704.5j), and nothing on the board
        // said it until the prompt that took one of two legends away
        // (issue #333).
        let mut abilities: Vec<String> = Vec::new();
        if Self::is_legendary(c) {
            abilities.push("legendary".into());
        }
        abilities.extend(c.keywords.iter().map(|k| format!("{k:?}").to_lowercase()));
        abilities.extend(c.protections.iter().cloned());
        let kw = if abilities.is_empty() {
            String::new()
        } else {
            format!(" ({})", abilities.join(", "))
        };
        // head is what must survive, elastic is what may be elided:
        // the flags used to be last and were the first thing a long
        // attachment list pushed off the end, so a tapped, damaged,
        // summoning-sick voltron creature read as a clean untapped
        // one (issue #270).
        (format!("{}{}{}", c.name, pt, CliPlayer::counters_suffix(&c.counters)),
         format!("{auras}{kw}"),
         flags)
    }

    fn is_legendary(p: &PermanentView) -> bool {
        p.supertypes.contains(&mtg_engine::types::Supertype::Legendary)
    }

    /// " (legendary)" on a non-creature permanent's row, or nothing. A
    /// creature's row carries the word among its abilities instead.
    fn legend_mark(p: &PermanentView) -> &'static str {
        if Self::is_legendary(p) { " (legendary)" } else { "" }
    }

    fn mid_print(out: &mut io::Stdout, col: u16, row: &mut u16, max_w: usize,

                  text: &str, color: Option<Color>, bold: bool) {
        let _ = execute!(out, cursor::MoveTo(col, *row));
        if bold { let _ = execute!(out, SetAttribute(Attribute::Bold)); }
        let truncated: String = text.chars().take(max_w).collect();
        Self::print_with_mana(out, &truncated, color);
        if bold { let _ = execute!(out, SetAttribute(Attribute::Reset)); }
        let _ = execute!(out, ResetColor);
        *row += 1;
    }

    fn print_colored(out: &mut impl Write, color: Color, text: &str) {
        let _ = execute!(out, SetForegroundColor(color), SetAttribute(Attribute::Bold),
            Print(format!("{text}\n")), SetAttribute(Attribute::Reset), ResetColor);
    }

    // ── Card reference panel ──────────────────────────────────────

    /// Build a prioritized, deduplicated list of card data for the reference panel.
    fn build_card_refs(view: &GameView, registry: &mtg_engine::cards::CardRegistry, filter: &str) -> Vec<CardRef> {
        let mut seen: Vec<String> = Vec::new();
        // Card id plus whether the entry is a permanent showing its back
        // face. The panel used to keep ids alone and resolve them with
        // `card_data`, which is always the FRONT face: a transformed
        // Cloistered Youth on the battlefield was described as a Cloistered
        // Youth, `/unholy` found nothing, and the panel listed the same card
        // twice — once for the hand copy and once for the transformed
        // permanent, deduped by the back-face name but rendered from the
        // front (issue #238).
        let mut entries: Vec<(mtg_engine::ids::CardId, bool)> = Vec::new();

        fn add(
            seen: &mut Vec<String>,
            entries: &mut Vec<(mtg_engine::ids::CardId, bool)>,
            name: &str,
            card_id: mtg_engine::ids::CardId,
        ) {
            if !seen.iter().any(|n| n == name) {
                seen.push(name.to_string());
                entries.push((card_id, false));
            }
        }

        // Priority 1: cards in your hand
        for c in &view.your_hand {
            add(&mut seen, &mut entries, &c.name, c.card_id);
        }
        // Priority 2: cards on the stack
        for s in &view.stack {
            add(&mut seen, &mut entries, &s.name, s.card_id);
        }
        // Priority 3 and 4: the battlefield, opponent's first (skip basic
        // lands). A permanent showing its back face is described by that
        // face, which is also the name it is listed under.
        fn add_permanent(
            p: &mtg_engine::view::PermanentView,
            registry: &mtg_engine::cards::CardRegistry,
            seen: &mut Vec<String>,
            entries: &mut Vec<(mtg_engine::ids::CardId, bool)>,
        ) {
            let printed = registry.card_data(p.card_id);
            if printed.as_ref()
                .is_some_and(|d| d.supertypes.contains(&mtg_engine::types::Supertype::Basic))
            {
                return;
            }
            if seen.contains(&p.name) { return; }
            seen.push(p.name.clone());
            // The view already resolved the active face's name, so a name
            // that differs from the printed one is a permanent showing its
            // back face.
            let showing_back = printed.is_some_and(|d| d.name != p.name);
            entries.push((p.card_id, showing_back));
        }
        for p in view.battlefield.iter().filter(|p| p.controller != view.you) {
            add_permanent(p, registry, &mut seen, &mut entries);
        }
        for p in view.battlefield.iter().filter(|p| p.controller == view.you) {
            add_permanent(p, registry, &mut seen, &mut entries);
        }
        // Priority 5: graveyard flashback cards (yours)
        for (pid, cards) in &view.graveyards {
            if *pid == view.you {
                for c in cards {
                    if c.flashback_cost.is_some() {
                        add(&mut seen, &mut entries, &c.name, c.card_id);
                    }
                }
            }
        }
        // Priority 6: recently seen cards in graveyards (both players,
        // most recent first — instants/sorceries that just resolved,
        // creatures that just died). The graveyard is an ordered pile
        // (CR 404.2) and the engine keeps it in arrival order, so the last
        // entry really is the card that just got there — this used to be an
        // assertion the engine did not honour, and `.rev()` showed whichever
        // card happened to have the highest object id (issue #222).
        for (_, cards) in &view.graveyards {
            for c in cards.iter().rev() {
                add(&mut seen, &mut entries, &c.name, c.card_id);
            }
        }
        // Priority 7: exile (cards that were exiled)
        for c in &view.exile {
            add(&mut seen, &mut entries, &c.name, c.card_id);
        }

        // Look up the face's CardData, filter out basic lands, apply text
        // filter. `/unholy` has to find the Unholy Fiend the battlefield is
        // showing, and must not find it under the front face's name.
        let filter_lower = filter.to_lowercase();
        entries.iter()
            .filter_map(|(id, showing_back)| {
                let data = if *showing_back {
                    registry.get(*id).and_then(mtg_engine::cards::CardBehavior::back_face_data)
                } else {
                    registry.card_data(*id)
                }?;
                let star_pt = registry.get(*id)
                    .is_some_and(mtg_engine::cards::CardBehavior::prints_star_pt);
                Some(CardRef { data, star_pt })
            })
            .filter(|c| !c.data.supertypes.contains(&mtg_engine::types::Supertype::Basic))
            .filter(|c| filter.is_empty() || c.data.name.to_lowercase().contains(&filter_lower))
            .collect()
    }

    /// Render the card reference panel in the right column.
    fn render_right_panel(out: &mut io::Stdout, cards: &[CardRef],
                           right_col: u16, right_w: usize, h: usize, filter: &str) {
        if right_w < 10 { return; }

        // Header — full gutter width
        let label = "─── CARDS ";
        let header = format!("{}{}", label, "─".repeat(right_w.saturating_sub(label.chars().count())));
        let _ = execute!(out, cursor::MoveTo(right_col, 0),
            SetAttribute(Attribute::Dim), Print(&header), SetAttribute(Attribute::Reset));

        // Search box below title
        let search_display = if filter.is_empty() {
            format!(" /search{}", " ".repeat(right_w.saturating_sub(8)))
        } else {
            // Clipped to the gutter by display columns: an overlong or
            // wide-character filter used to wrap onto the next terminal row
            // and paint over the STACK panel (#109). Show the TAIL — what
            // the player just typed is the useful part.
            let text = format!(" /{filter}");
            let text = if str_cols(&text) > right_w {
                let tail: String = text.chars().rev().collect::<String>();
                let mut kept = clip_cols(&tail, right_w.saturating_sub(1));
                kept = kept.chars().rev().collect();
                format!("\u{2026}{kept}")
            } else {
                text
            };
            let pad = right_w.saturating_sub(str_cols(&text));
            format!("{}{}", text, " ".repeat(pad))
        };
        let _ = execute!(out, cursor::MoveTo(right_col, 1),
            SetAttribute(Attribute::Dim), Print(&search_display), SetAttribute(Attribute::Reset));

        let content_w = right_w.saturating_sub(1); // text margin

        let mut row: u16 = 2; // cards start below search box
        let max_row = u16::try_from(h).unwrap_or(u16::MAX).saturating_sub(1);

        for card in cards {
            if row >= max_row { break; }

            // Name + cost
            let cost_str = card.data.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
            let name_line = format!("{}{}", card.data.name, cost_str);
            let truncated: String = name_line.chars().take(content_w).collect();
            let _ = execute!(out, cursor::MoveTo(right_col, row), SetAttribute(Attribute::Bold));
            Self::print_with_mana(out, &truncated, None);
            let _ = execute!(out, SetAttribute(Attribute::Reset));
            row += 1;
            if row >= max_row { break; }

            // Type line + P/T, supertypes first (CR 205.4a, issue #333).
            let pt = if card.star_pt {
                " */*".to_string()
            } else {
                match (card.data.power, card.data.toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                }
            };
            let type_line = format!("{}{}", mtg_engine::types::type_line(
                &card.data.supertypes, &card.data.card_types, &card.data.subtypes), pt);

            let truncated: String = type_line.chars().take(content_w).collect();
            let _ = execute!(out, cursor::MoveTo(right_col, row),
                SetAttribute(Attribute::Dim), Print(&truncated), SetAttribute(Attribute::Reset));
            row += 1;
            if row >= max_row { break; }

            // Keywords
            if !card.data.keywords.is_empty() {
                let kw_str: Vec<&str> = card.data.keywords.iter().map(|k| match k {
                    mtg_engine::types::Keyword::Flying => "Flying",
                    mtg_engine::types::Keyword::FirstStrike => "First strike",
                    mtg_engine::types::Keyword::DoubleStrike => "Double strike",
                    mtg_engine::types::Keyword::Trample => "Trample",
                    mtg_engine::types::Keyword::Deathtouch => "Deathtouch",
                    mtg_engine::types::Keyword::Lifelink => "Lifelink",
                    mtg_engine::types::Keyword::Vigilance => "Vigilance",
                    mtg_engine::types::Keyword::Flash => "Flash",
                    mtg_engine::types::Keyword::Reach => "Reach",
                    mtg_engine::types::Keyword::Haste => "Haste",
                    mtg_engine::types::Keyword::Defender => "Defender",
                    mtg_engine::types::Keyword::Hexproof => "Hexproof",
                    mtg_engine::types::Keyword::Intimidate => "Intimidate",
                    mtg_engine::types::Keyword::Menace => "Menace",
                    mtg_engine::types::Keyword::Indestructible => "Indestructible",
                }).collect();
                let kw_line = kw_str.join(", ");
                let truncated: String = kw_line.chars().take(content_w).collect();
                let _ = execute!(out, cursor::MoveTo(right_col, row),
                    SetForegroundColor(Color::Blue), Print(&truncated), ResetColor);
                row += 1;
                if row >= max_row { break; }
            }

            // Oracle text (word-wrapped), minus the lines the panel already
            // prints for itself above and below.
            if !card.data.oracle_text.is_empty() {
                let kept = Self::card_panel_oracle_lines(
                    &card.data.oracle_text, card.data.flashback_cost.is_some());
                let text = kept.join("\n");
                if !text.trim().is_empty() {
                    let wrapped = Self::wrap_text(text.trim(), content_w);
                    for line in wrapped {
                        if row >= max_row { break; }
                        let _ = execute!(out, cursor::MoveTo(right_col, row));
                        Self::print_with_mana(out, &line, None);
                        row += 1;
                    }
                }
            }

            // Flashback cost
            if let Some(fb) = &card.data.flashback_cost {
                if row < max_row {
                    let fb_line = format!("Flashback {fb}");
                    let truncated: String = fb_line.chars().take(content_w).collect();
                    let _ = execute!(out, cursor::MoveTo(right_col, row));
                    Self::print_with_mana(out, &truncated, Some(Color::Cyan));
                    row += 1;
                }
            }

            // Subtle dot separator between cards
            if row < max_row {
                let sep: String = "·".repeat(right_w);
                let _ = execute!(out, cursor::MoveTo(right_col, row),
                    SetAttribute(Attribute::Dim), Print(&sep), SetAttribute(Attribute::Reset));
                row += 1;
            }
        }

        // (search box is at top, no footer needed)
    }

    /// Simple word-wrap for oracle text.
    fn wrap_text(text: &str, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                if line.is_empty() {
                    line = word.to_string();
                } else if line.len() + 1 + word.len() <= width {
                    line.push(' ');
                    line.push_str(word);
                } else {
                    lines.push(line);
                    line = word.to_string();
                }
            }
            if !line.is_empty() {
                lines.push(line);
            }
        }
        lines
    }

    /// Interactive card search: enters raw mode, reads key-by-key,
    /// re-renders the right panel live, exits on Escape or `/`.
    /// `message` is the prompt's own header — the pending notice, or
    /// `legal.context`, or a chooser's title. It used to be dropped while the
    /// search box was open: the rule above the option list went blank, which
    /// at an ordinary priority menu costs the step name and at a mandatory
    /// prompt costs the question itself, since the description is the only
    /// thing on screen saying what the numbered rows are for (issue #327).
    /// The search overlay changes what is in the CARDS gutter, not what the
    /// game is asking.
    fn run_card_search(view: &GameView, actions: &[MenuLabel], message: Option<&str>,
                       menu_offset: usize) {
        // The search box is part of the right panel, which is only drawn at
        // >= 100 columns. Entering search mode on a narrower terminal showed
        // nothing at all and silently swallowed every keystroke until an
        // undiscoverable Esc/Enter (issue #107) — refuse to enter it instead
        // (the hint line stops advertising it at this width too).
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        if (term_w as usize) < 100 {
            return;
        }

        tui_raw_on();
        // A paste must arrive as one Paste event, never as keystrokes whose
        // embedded newlines exit the search and SUBMIT at the underlying
        // prompt (issue #106; same hardening as read_line, #50).
        let _ = execute!(stdout(), event::EnableBracketedPaste);

        // The filter is scratch state for this one search (it was cleared on
        // entry and exit as a field too) — local, so the search is callable
        // from static prompts like the target choosers (issue #122).
        let mut card_filter = String::new();

        loop {
            // Re-render with current filter
            let _ = Self::render_paged(view, Some(actions), message, &view.display_log, &card_filter, None, menu_offset);

            // Move actual cursor to the search box in the right gutter
            let (term_w, _) = terminal::size().unwrap_or((100, 30));
            let w = term_w as usize;
            let gutter_w = w / 5;
            let mid_w = w.saturating_sub(gutter_w * 2 + 2);
            let right_col = u16::try_from(gutter_w + 1 + mid_w + 1).unwrap_or(u16::MAX);
            let cursor_x = right_col + 2 + u16::try_from(card_filter.chars().count()).unwrap_or(u16::MAX);
            let _ = execute!(stdout(), cursor::MoveTo(cursor_x, 1));
            let _ = stdout().flush();

            // Read one event
            let Some(ev) = read_event_guarded() else { continue };
            // A paste lands in the filter as one event — only its first line,
            // and its newlines never act as Enter (issue #106).
            if let Event::Paste(pasted) = &ev {
                let first = pasted.split(['\r', '\n']).next().unwrap_or("");
                card_filter.push_str(first);
                continue;
            }
            if let Event::Key(KeyEvent { code, modifiers, .. }) = ev {
                match code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('/') => {
                        card_filter.clear();
                        break;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        quit_at_prompt();
                    }
                    KeyCode::Backspace => {
                        card_filter.pop();
                    }
                    // A chord the UI doesn't bind (Ctrl-A, Alt-x, ...) is
                    // ignored, never inserted as its bare character (#51).
                    KeyCode::Char(c) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        card_filter.push(c);
                    }
                    _ => {}
                }
            }
        }

        let _ = execute!(stdout(), event::DisableBracketedPaste);
        tui_raw_off();
    }

    /// Interactive target selection for a castable spell.
    ///
    /// `None` abandons the cast with nothing spent. Cancelling is an empty
    /// line, `c`/`cancel`, or the Cancel row, at any chooser — Escape is not
    /// bound anywhere in this CLI, and the doc used to promise it (#288).
    fn choose_targets(view: &GameView, spell: &mtg_engine::actions::CastableSpell) -> Option<Action> {
        use mtg_engine::actions::CastTargetSpec;

        let chosen_targets = match &spell.target_spec {
            CastTargetSpec::NoTargets => vec![],
            CastTargetSpec::SingleTarget(options) => {
                // CR 601.2c: with exactly one legal target the choice is
                // forced, so the row NAMES it (see `cast_row_label`) rather
                // than offering a one-option menu. The silent version cast
                // Brimstone Volley at its own caster off one keypress, with
                // nothing on screen saying so (issue #254).
                let forced = Self::forced_cast_targets(&spell.target_spec);
                if forced.is_empty() {
                    vec![Self::prompt_target(view, options,
                        &format!("{}: select a target", spell.name))?]
                } else {
                    forced
                }
            }
            CastTargetSpec::TwoTargets { first, second, second_min, second_max } => {
                let t1 = Self::prompt_target(view, first, &format!("{}: select first of two targets", spell.name))?;
                let idx = first.iter().position(|t| *t == t1)?;
                // The engine pre-narrowed each first choice's legal second-slot
                // options (e.g. "cards from THEIR graveyard" — only the chosen
                // player's cards).
                let remaining = second[idx].clone();
                if *second_max <= 1 {
                    if remaining.is_empty() {
                        return None;
                    }
                    let t2 = Self::prompt_target(view, &remaining, &format!("{}: select second of two targets", spell.name))?;
                    vec![t1, t2]
                } else {
                    // A wide second slot is chosen on the marking screen the
                    // cast raises, not one question per pick: this asked
                    // "select target 1 of up to 3", then again, and again.
                    let _ = (&remaining, second_min);
                    vec![t1]
                }
            }
            CastTargetSpec::UpToTargets { .. } => {
                // Likewise: the cast is submitted with the slot empty and
                // the engine asks for the whole set at once (CR 601.2c).
                // Choosing zero is a real cast and is said on that screen by
                // marking none — it is not this branch returning early,
                // which is how it became a silent no-op in issue #49.
                Vec::new()
            }
        };

        // Prompt for sacrifice if the spell has a sacrifice additional cost.
        // CR 601.2h, same rule as the target above: one eligible creature is
        // no choice, and the row names it instead of prompting (#254).
        let chosen_sacrifice = match Self::forced_sacrifice(&spell.sacrifice_options) {
            Some(id) => Some(id),
            None if spell.sacrifice_options.is_empty() => None,
            None => {
                let target = Self::prompt_target(view,
                    &spell.sacrifice_options.iter().map(|&id| mtg_engine::actions::Target::Object(id)).collect::<Vec<_>>(),
                    &format!("{}: choose a creature to sacrifice", spell.name))?;
                match target {
                    mtg_engine::actions::Target::Object(id) => Some(id),
                    mtg_engine::actions::Target::Player(_) => None,
                    Target::Illegal => None,
                }
            }
        };

        Some(Action::CastSpell {
            object_id: spell.object_id,
            targets: chosen_targets,
            sacrifice: chosen_sacrifice,
            exile_count: None,
            exile_ids: vec![],
            // The cost this entry was offered for: dropping it here charged
            // the normal cost for an alternative-cost entry (issue #128).
            alternative_cost: spell.alternative_cost.clone(),
            tap_plan: spell.tap_plan.clone(),
        })
    }

    /// The picker rows for a list of targets: the name is the row, the object
    /// is its identity.
    ///
    /// Two rows that read alike but name different objects have to be
    /// tellable apart — two identical tokens in a picker decided whether an
    /// Aura on the stack would fizzle, with nothing on screen saying so
    /// (issue #136). That id used to be appended here, before the renderer
    /// clipped the row, so it was the first thing a clip removed; carrying
    /// the object instead lets the id be added at the width the row is
    /// actually drawn (issue #258).
    fn target_menu_labels(view: &GameView, options: &[mtg_engine::actions::Target]) -> Vec<MenuLabel> {
        options.iter().map(|t| match t {
            mtg_engine::actions::Target::Object(id) => MenuLabel {
                text: Self::perm_name(view, *id),
                ids: vec![id.0],
            },

            mtg_engine::actions::Target::Player(pid) => MenuLabel::plain(
                if *pid == view.you { "You" } else { "Opponent" }),
            mtg_engine::actions::Target::Illegal =>
                unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
        }).collect()
    }

    /// One reading of a target chooser's input, for all of them.
    ///
    /// The three choosers each had their own rule and a bare Enter meant
    /// three different things: `prompt_target` cancelled, and the two "up to
    /// N" prompts CAST — two lands tapped, the card to the graveyard, or
    /// with flashback exiled forever, from the key a player presses to back
    /// out (issue #288). Enter is the reversible key everywhere in this CLI
    /// (#123), so it is `Cancel` here too, and stopping early keeps its own
    /// row.
    fn parse_target_input(input: &str, n_options: usize, rows: ChooserRows) -> TargetInput {
        let t = input.trim();
        if t.is_empty() {
            return TargetInput::Cancel;
        }
        // Panel keys before the numeric parse, as before.
        if let Some(c) = t.chars().next() {
            if t.chars().count() == 1 && "lgedis/".contains(c) {
                return TargetInput::Panel(c);
            }
        }
        if t == "m" {
            return TargetInput::NextPage;
        }
        if t == "p" {
            return TargetInput::PrevPage;
        }
        if t.eq_ignore_ascii_case("c") || t.eq_ignore_ascii_case("cancel") {
            return TargetInput::Cancel;
        }
        if rows == ChooserRows::DoneThenCancel && t.eq_ignore_ascii_case("done") {
            return TargetInput::Done;
        }
        if let Ok(idx) = t.parse::<usize>() {
            if idx < n_options {
                return TargetInput::Pick(idx);
            }
            return match (rows, idx - n_options) {
                (ChooserRows::CancelOnly, 0) => TargetInput::Cancel,
                (ChooserRows::DoneThenCancel, 0) => TargetInput::Done,
                (ChooserRows::DoneThenCancel, 1) => TargetInput::Cancel,
                _ => TargetInput::Invalid,
            };
        }
        TargetInput::Invalid
    }

    /// The rows a target chooser shows: the targets, then its trailing rows.
    ///
    /// Every chooser ends in a Cancel row. Making that unconditional here is
    /// what stops the next one being written without an exit — the "up to N"
    /// second slot of a two-target spell had none at all (issue #288).
    fn chooser_labels(view: &GameView, options: &[mtg_engine::actions::Target], rows: ChooserRows)
        -> Vec<MenuLabel>
    {
        let mut labels = Self::target_menu_labels(view, options);
        match rows {
            ChooserRows::CancelOnly => labels.push(MenuLabel::plain("Cancel the cast")),
            ChooserRows::DoneThenCancel => {
                labels.push(MenuLabel::plain("Done (cast with targets chosen so far)"));
                labels.push(MenuLabel::plain("Cancel the cast"));
            }
        }
        labels
    }

    /// Run one target chooser to an answer.
    fn run_target_chooser(
        view: &GameView,
        options: &[mtg_engine::actions::Target],
        label: &str,
        rows: ChooserRows,
    ) -> UpToPick {
        let labels = Self::chooser_labels(view, options, rows);
        let mut notice: Option<String> = None;
        // A chooser longer than the pane pages like the priority menu does.
        // It drew the "m = next page" marker and had no offset to advance,
        // so every option past the first page — the Cancel row included —
        // was reachable only by typing a number that was not on the screen
        // (issue #261).
        let mut menu_offset = 0usize;
        loop {
            let title = notice.take().map_or_else(|| label.to_string(),
                |n| format!("{n} — {label}"));
            let page = Self::render_paged(
                view, Some(&labels), Some(&title), &view.display_log, "", None, menu_offset);
            let input = Self::read_line("");
            match Self::parse_target_input(&input, options.len(), rows) {
                TargetInput::Pick(idx) => return UpToPick::Pick(options[idx].clone()),
                TargetInput::Done => return UpToPick::Done,
                TargetInput::Cancel => return UpToPick::Cancel,
                TargetInput::NextPage => {
                    menu_offset = Self::next_menu_offset(menu_offset, page.shown, labels.len());
                }
                TargetInput::PrevPage => {
                    menu_offset = Self::prev_menu_offset_lines(&page.heights, page.avail, menu_offset, page.marker_h);
                }

                // Info panes + card search: a player wants their graveyard
                // exactly when choosing a target (issue #122).
                TargetInput::Panel(c) => {
                    match c {
                        'l' => Self::show_log(&view.display_log),
                        'g' => Self::show_graveyards(view),
                        'e' => Self::show_exile(view),
                        'd' => Self::show_deck_browser(view),
                        'i' => Self::show_battlefield_inspector(view),
                        's' => Self::show_stack(view),
                        _ => Self::run_card_search(view, &labels, Some(&title), menu_offset),
                    }
                }
                // A silent re-render is indistinguishable from a hung game —
                // same rule as the main menu (#76, issue #122).
                TargetInput::Invalid => {
                    notice = Some(format!(
                        "Invalid input '{}' — enter a number 0-{}, or c to cancel the cast",
                        quote_input(&input), labels.len() - 1));
                }
            }
        }
    }

    /// Prompt the user to pick one target from a list. `None` abandons the
    /// cast with nothing spent.
    fn prompt_target(view: &GameView, options: &[mtg_engine::actions::Target], label: &str)
        -> Option<mtg_engine::actions::Target>
    {
        match Self::run_target_chooser(view, options, label, ChooserRows::CancelOnly) {
            UpToPick::Pick(t) => Some(t),
            // `Done` is not offered by this chooser.
            UpToPick::Done | UpToPick::Cancel => None,
        }
    }


    // ── Action formatting ──────────────────────────────────────────

    /// Format a tap plan as a compact string like "2x Plains, Hinterland Harbor".
    fn format_tap_plan(view: &GameView, tap_plan: &[(ObjectId, usize)]) -> String {
        if tap_plan.is_empty() { return String::new(); }
        let mut name_counts: Vec<(String, usize)> = Vec::new();
        for &(source_id, _) in tap_plan {
            let name = Self::perm_name(view, source_id);
            if let Some(entry) = name_counts.iter_mut().find(|(n, _)| *n == name) {
                entry.1 += 1;
            } else {
                name_counts.push((name, 1));
            }
        }
        name_counts.iter()
            .map(|(name, count)| {
                if *count > 1 { format!("{count}x {name}") } else { name.clone() }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// A combat-list entry for `id`, disambiguated against `others`.
    ///
    /// The three combat lists rendered `<name> <P/T> (your|opp)` and nothing
    /// else: two same-named attackers were byte-identical even when one of
    /// them had damage marked and would die to the block (issue #268), an
    /// attacker aimed at a planeswalker looked exactly like one aimed at the
    /// player (issue #219), and the keywords the block turns on were not
    /// there either (issue #243). All of it is public information at the
    /// moment the defender has to use it.
    /// The entry as it would read with no width limit. The screen goes
    /// through [`combat_row`](Self::combat_row), which clamps it; this is
    /// what the tests about *what an entry says* read, so they can state
    /// that without stating a width too.
    /// A combat-list entry, whole: the creature, its live abilities, its
    /// damage, what it is attacking, and the id when a twin needs it.
    fn combat_entry(view: &GameView, id: ObjectId, others: &[ObjectId]) -> String {

        let (head, elastic, tail) = Self::combat_entry_parts(view, id, others);
        format!("{head}{elastic}{tail}")
    }

    /// The same entry, split the way [`elide_middle`](Self::elide_middle)
    /// wants it: what must survive, what may be cut, and what must survive at
    /// the end.
    ///
    /// The combat lists were emitted at their natural length while the
    /// battlefield rows beside them were already clamped (#244). An Elite
    /// Inquisitor's abilities are 100 columns of suffix on their own, so its
    /// row ran through the pane's right border and overwrote the CARDS pane
    /// on exactly the line the defender was reading (issue #328).
    ///
    /// What may be cut is the ability list — long, and the only part with a
    /// natural middle. What may not is the name and P/T (which say what the
    /// creature is), the attack target (CR 508.1a: it decides how the
    /// creature should be blocked) and the `(#id)` disambiguator, which is
    /// the whole reason two identical rows can be told apart (#136).
    fn combat_entry_parts(view: &GameView, id: ObjectId, others: &[ObjectId])
        -> (String, String, String)
    {
        let perm = view.battlefield.iter().find(|p| p.object_id == id);
        let head = Self::perm_name(view, id);
        let mut elastic = String::new();
        let mut tail = String::new();
        if let Some(p) = perm {
            let mut abilities: Vec<String> = p.keywords.iter()
                .map(|k| format!("{k:?}").to_lowercase())
                .collect();
            abilities.extend(p.protections.iter().cloned());
            if !abilities.is_empty() {
                elastic.push_str(&format!(" ({})", abilities.join(", ")));
            }
            if p.damage_marked > 0 {
                tail.push_str(&format!(" ({}d)", p.damage_marked));
            }
            // CR 508.1a: each attacker attacks a player or a planeswalker of
            // its own, and which one decides how it should be blocked.
            if let Some(mtg_engine::view::AttackTarget::Planeswalker(walker)) = &p.attacking {
                let name = view.battlefield.iter()
                    .find(|w| w.object_id == *walker)
                    .map_or_else(|| "a planeswalker".to_string(), |w| {
                        let loyalty = w.counters.get(&mtg_engine::types::CounterType::Loyalty)
                            .copied().unwrap_or(0);
                        format!("{} [{loyalty} loyalty]", w.name)
                    });
                tail.push_str(&format!(" -> {name}"));
            }
        }
        // Two entries that still read the same are told apart by object id,
        // the way the target pickers already do it (#136).
        let collides = others.iter().any(|&other| {
            other != id && Self::combat_entry_base(view, other) == Self::combat_entry_base(view, id)
        });
        if collides {
            tail.push_str(&format!(" (#{})", id.0));
        }
        (head, elastic, tail)
    }

    /// A combat-list row laid out for the pane it is drawn in: the entry
    /// wrapped under a hanging indent, and the caller's coloured note —
    /// `[MUST ATTACK]`, `[needs N+ blockers]`, `(can block: …)` — on the
    /// end of the last line when it fits there, on a line of its own when
    /// it does not. Nothing is cut (issues #328, #318).
    ///
    /// `prefix_w` is the width of the row's own `"  N: "`; `panel_w` the
    /// pane's content width, passed in rather than measured so the widths
    /// that matter can be stated in a test without a terminal.
    fn combat_row_layout(view: &GameView, id: ObjectId, others: &[ObjectId],
                         prefix_w: usize, note: &str, panel_w: usize) -> CombatRowLayout {
        let budget = panel_w.saturating_sub(prefix_w).max(1);
        let lines = Self::wrap_row(&Self::combat_entry(view, id, others), budget);
        let last = lines.last().map_or(0, |l| str_cols(l));
        let note_lines = if note.is_empty() {
            Vec::new()
        } else if last + str_cols(note) <= budget {
            Vec::new()
        } else {
            Self::wrap_row(note.trim_start(), budget)
        };
        CombatRowLayout { lines, note: note.to_string(), note_lines }
    }

    /// Paint one laid-out combat row at `(col, r)`, the index in bold and
    /// the note in `note_color`, advancing `r` past every line it used.
    fn draw_combat_row(out: &mut io::Stdout, col: u16, r: &mut u16, index: usize,
                       prefix_w: usize, layout: &CombatRowLayout, note_color: Color) {
        let indent = " ".repeat(prefix_w);
        for (k, line) in layout.lines.iter().enumerate() {
            let _ = execute!(out, cursor::MoveTo(col, *r));
            if k == 0 {
                let _ = execute!(out, SetAttribute(Attribute::Bold), Print(format!("  {index}")),
                    SetAttribute(Attribute::Reset), Print(": "));
            } else {
                let _ = execute!(out, Print(&indent));
            }
            let _ = execute!(out, Print(line));
            if k == layout.lines.len() - 1 && layout.note_lines.is_empty() && !layout.note.is_empty() {
                let _ = execute!(out, SetForegroundColor(note_color), Print(&layout.note), ResetColor);
            }
            *r += 1;
        }
        for line in &layout.note_lines {
            let _ = execute!(out, cursor::MoveTo(col, *r), Print(&indent),
                SetForegroundColor(note_color), Print(line), ResetColor);
            *r += 1;
        }
    }


    /// The part of a combat entry that decides whether two rows collide.
    fn combat_entry_base(view: &GameView, id: ObjectId) -> String {
        let perm = view.battlefield.iter().find(|p| p.object_id == id);
        let damage = perm.map_or(0, |p| p.damage_marked);
        let attacking = perm.and_then(|p| p.attacking.clone());
        format!("{}|{damage}|{attacking:?}", Self::perm_name(view, id))
    }

    fn perm_name(view: &GameView, id: ObjectId) -> String {
        view.battlefield.iter()
            .find(|p| p.object_id == id)
            .map(|p| {
                let pt = match (p.effective_power, p.effective_toughness) {
                    (Some(pw), Some(t)) => format!(" {pw}/{t}"),
                    _ => String::new(),
                };
                // Lands carry the marker too: "Destroy target land" offered
                // your own and the opponent's Islands as byte-identical menu
                // lines (issue #100).
                let owner = if p.controller == view.you { "your" } else { "opp" };
                format!("{}{} ({})", p.name, pt, owner)
            })
            .or_else(|| view.your_hand.iter()
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.stack.iter()
                .find(|s| s.object_id == id)
                .map(|s| s.name.clone()))
            // Cards a choice can name live beyond the battlefield/hand/stack:
            // the library (search effects), graveyards (flashback, reanimation),
            // and the revealed-names map the view builds for pending choices.
            // Falling through to the raw obj#NN id made the look-at-top-N
            // picker unreadable (issue #38).
            .or_else(|| view.your_library_cards.iter()
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.graveyards.iter()
                .flat_map(|(_, cards)| cards.iter())
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            // Exile is a public zone and belongs on this list beside the
            // graveyards — it was the one zone missing, and Runic Repetition
            // is the only card in the pool whose targets live there, so the
            // hole showed up as an unplayable card: every place the CLI named
            // its target printed `obj#NN`, and no pane anywhere shows object
            // ids (issue #332, the same hole #38 closed for the library).
            .or_else(|| view.exile.iter()
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.revealed_names.get(&id).cloned())
            .unwrap_or_else(|| format!("{id}"))
    }

    /// The key hints under a menu.
    ///
    /// A target chooser printed no `enter=` hint of any kind, so the one key
    /// that abandons a cast was advertised nowhere on the screen a player
    /// was staring at (issue #288). The chooser is recognised by its last
    /// row, the way the priority menu is recognised by its first — matched
    /// exactly, so the resolution menu's own "Cancel cast" row is not
    /// mistaken for one.
    fn menu_hints(labels: &[MenuLabel], has_right: bool) -> &'static str {
        let has_pass = labels.first().is_some_and(|l| l.full() == "Pass priority");
        let is_chooser = labels.last().is_some_and(|l| l.full() == "Cancel the cast");
        // The `/` search lives in the right panel, which only exists at
        // >= 100 columns — advertising it below that put users into an
        // invisible modal mode that swallowed keystrokes (issue #107).
        match (has_pass, is_chooser, has_right) {
            (true, _, true) =>
                "  [enter=pass] [f=auto-pass] [/=search] [d=deck] [l=log] [g=gy] [e=exile] [s=stack] [m/p=page]",
            (true, _, false) =>
                "  [enter=pass] [f=auto-pass] [d=deck] [l=log] [g=gy] [e=exile] [s=stack] [m/p=page]",
            (false, true, true) =>
                "  [enter=cancel] [/=search] [d=deck] [l=log] [g=gy] [e=exile] [s=stack] [m/p=page]",
            (false, true, false) =>
                "  [enter=cancel] [d=deck] [l=log] [g=gy] [e=exile] [s=stack] [m/p=page]",
            (false, false, true) => "  [/=search] [d=deck] [l=log] [g=gy] [e=exile] [s=stack] [m/p=page]",
            (false, false, false) => "  [d=deck] [l=log] [g=gy] [e=exile] [s=stack] [m/p=page]",
        }
    }

    /// The rows of a menu as they read, with any two that read the same
    /// told apart.
    ///
    /// Rows whose identity objects are the same are genuinely
    /// interchangeable and are left alike (that is #54's collapse); rows
    /// that name different objects get those objects' ids, the #136/#100
    /// convention — two Wooden Stakes, or one Stake offered against two
    /// identical tokens, read the same and are not the same (issue #257).
    fn menu_row_texts(labels: &[MenuLabel]) -> Vec<String> {
        let mut out: Vec<String> = labels.iter().map(MenuLabel::full).collect();
        let mut handled = vec![false; out.len()];
        for k in 0..out.len() {
            if handled[k] { continue; }
            let group: Vec<usize> = (k..out.len()).filter(|&j| out[j] == out[k]).collect();
            for &j in &group { handled[j] = true; }
            if group.len() < 2 { continue; }
            // The id positions on which this group is not unanimous are
            // exactly what distinguishes its members.
            let width = group.iter().map(|&j| labels[j].ids.len()).max().unwrap_or(0);
            let differing: Vec<usize> = (0..width)
                .filter(|&p| group.iter().any(|&j| labels[j].ids.get(p) != labels[group[0]].ids.get(p)))
                .collect();
            if differing.is_empty() { continue; }
            for &j in &group {
                let named: Vec<String> = differing.iter()
                    .filter_map(|&p| labels[j].ids.get(p))
                    .map(|id| format!("#{id}"))
                    .collect();
                if named.is_empty() { continue; }
                out[j] = format!("{} ({})", out[j], named.join(" "));
            }
        }
        out
    }

    /// One menu row broken into lines of at most `width` display columns,
    /// losing nothing (issue #318).
    ///
    /// A row breaks after a comma before it breaks at a space: the rows
    /// that overflow in practice are lists — "Bottom A, B, C", "tap
    /// Mountain, Mountain, Forest" — and a list reads as a list when each
    /// line ends on an item. The longest comma-terminated prefix that fits
    /// is taken; failing a comma, the longest space-terminated one; failing
    /// any space, the word is cut at the column. Measured in display
    /// columns, not chars, so a wide-character name wraps where it should
    /// (#109, #53).
    fn wrap_row(text: &str, width: usize) -> Vec<String> {
        if width == 0 || text.is_empty() {
            return vec![text.to_string()];
        }
        let mut lines = Vec::new();
        let mut rest = text;
        loop {
            if str_cols(rest) <= width {
                lines.push(rest.to_string());
                return lines;
            }
            // Byte offsets, with the columns used up to each char, so the
            // break candidates are found in one pass. A comma counts as a
            // break as soon as the comma itself fits — the space after it
            // is swallowed by the break, so it need not.
            let mut cols = 0;
            let mut last_comma: Option<usize> = None; // byte index just after ','
            let mut last_space: Option<usize> = None; // byte index of ' '
            let mut hard_end = rest.len();
            let mut it = rest.char_indices().peekable();
            while let Some((i, c)) = it.next() {
                let w = col_width(c);
                if cols + w > width {
                    hard_end = i;
                    break;
                }
                cols += w;
                match c {
                    ' ' if i > 0 => last_space = Some(i),
                    // A comma that separates items, not one inside a number.
                    ',' if i > 0 && it.peek().is_none_or(|&(_, next)| next == ' ') => {
                        last_comma = Some(i + 1);
                    }
                    _ => {}
                }
            }
            let cut = last_comma.or(last_space).unwrap_or(hard_end);

            let (line, tail) = rest.split_at(cut);
            lines.push(line.trim_end().to_string());
            rest = tail.trim_start();
            if rest.is_empty() {
                return lines;
            }
        }
    }

    /// Every menu row wrapped to `width` columns: one `Vec` of lines per row,
    /// rows that read the same already told apart by id.
    fn wrap_menu_rows(labels: &[MenuLabel], width: usize) -> Vec<Vec<String>> {
        Self::menu_row_texts(labels).iter()
            .map(|text| Self::wrap_row(text, width))
            .collect()
    }


    /// " targeting X" for an action's chosen targets, or "" when untargeted.
    /// The legal-action list pre-expands one entry per target, so a label
    /// that omits the target renders identical menu lines whose choice
    /// silently decides who gets hit — twice a self-hit in real games
    /// (issue #36).
    fn targets_suffix(view: &GameView, targets: &[Target]) -> String {
        if targets.is_empty() {
            return String::new();
        }
        let names: Vec<String> = targets.iter().map(|t| match t {
            Target::Object(id) => Self::perm_name(view, *id),
            Target::Player(pid) =>
                if *pid == view.you { "you".into() } else { "opponent".into() },
            Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
        }).collect();
        format!(" targeting {}", names.join(", "))
    }

    /// The targets a cast row will hit WITHOUT asking, or empty when a
    /// chooser will run.
    ///
    /// With exactly one legal target the choice is forced (CR 601.2c), so
    /// the CLI takes it rather than offering a one-option menu — but the row
    /// then has to SAY which one, or a single keypress commits a target the
    /// player was never shown. Brimstone Volley whose only legal target was
    /// its own caster read exactly like one aimed at the opponent (issue
    /// #254). `TwoTargets` and `UpToTargets` always prompt.
    fn forced_cast_targets(spec: &mtg_engine::actions::CastTargetSpec) -> Vec<Target> {
        match spec {
            mtg_engine::actions::CastTargetSpec::SingleTarget(o) if o.len() == 1 =>
                vec![o[0].clone()],
            _ => Vec::new(),
        }
    }

    /// The creature a cast will sacrifice WITHOUT asking (CR 601.2h) — one
    /// eligible creature is no choice at all, and the row names it.
    fn forced_sacrifice(options: &[ObjectId]) -> Option<ObjectId> {
        match options {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// ", sacrificing X" for a cast or ability row, or "" when nothing is
    /// sacrificed.
    fn sacrifice_suffix(view: &GameView, sacrifice: Option<ObjectId>) -> String {
        sacrifice.map_or_else(String::new,
            |id| format!(", sacrificing {}", Self::perm_name(view, id)))
    }

    /// The menu row for one way to cast one spell.
    ///
    /// It carries what the cast will do without asking again: the forced
    /// target, the forced sacrifice, and — when the cost still has a choice
    /// in it — that there is one to make.
    fn cast_row_label(view: &GameView, cs: &mtg_engine::actions::CastableSpell) -> MenuLabel {
        let verb = if cs.is_flashback { "Flashback" } else { "Cast" };
        let zone_note = if cs.from_graveyard { " from graveyard" } else { "" };
        let mut notes: Vec<String> = Vec::new();
        match &cs.alternative_cost {
            Some(alt) if !cs.is_flashback && alt.symbols.is_empty() =>
                notes.push("without paying its mana cost".to_string()),
            Some(alt) if !cs.is_flashback =>
                notes.push(format!("alternative cost {alt}")),
            _ => {}
        }
        // The additional cost is the whole reason two ways to cast the same
        // card are not interchangeable — but once the cost is forced, the
        // tail names the creature instead, which says strictly more.
        let forced_sac = Self::forced_sacrifice(&cs.sacrifice_options);
        if forced_sac.is_none() {
            if let Some(extra) = &cs.additional_cost_label {
                notes.push(extra.clone());
            }
        }
        let tap_str = Self::format_tap_plan(view, &cs.tap_plan);
        if !tap_str.is_empty() {
            notes.push(format!("tap {tap_str}"));
        }
        let forced_targets = Self::forced_cast_targets(&cs.target_spec);
        let mut ids = vec![cs.object_id.0];
        ids.extend(forced_targets.iter().filter_map(|t| match t {
            Target::Object(id) => Some(id.0),
            _ => None,
        }));
        if let Some(sac) = forced_sac { ids.push(sac.0); }
        let notes = if notes.is_empty() {
            String::new()
        } else {
            format!(" ({})", notes.join(", "))
        };
        MenuLabel {
            text: format!("{verb} {}{zone_note}{notes}{}{}", cs.name,
                Self::targets_suffix(view, &forced_targets),
                Self::sacrifice_suffix(view, forced_sac)),
            ids,
        }
    }


    fn format_action(view: &GameView, action: &Action) -> String {
        match action {
            Action::PassPriority => "Pass priority".into(),
            // Never offered: the harness stopping is not a menu row (#233).
            Action::AbandonGame => "Abandon game".into(),
            Action::PlayLand { object_id } =>
                format!("Play land {}", Self::perm_name(view, *object_id)),
            Action::CastSpell { object_id, targets, tap_plan, sacrifice, .. } => {
                // The same shape as the ability rows and the collapsed cast
                // rows: what it casts, what it costs, what it hits, what it
                // kills. It used to spell the targets its own way and never
                // mention the sacrifice at all (#254).
                let name = Self::perm_name(view, *object_id);
                let tap_str = Self::format_tap_plan(view, tap_plan);
                let tap_suffix = if tap_str.is_empty() { String::new() } else { format!(" (tap {tap_str})") };
                format!("Cast {name}{tap_suffix}{}{}",
                    Self::targets_suffix(view, targets),
                    Self::sacrifice_suffix(view, *sacrifice))
            }
            Action::ActivateManaAbility { object_id, ability_index } => {
                // Name the mana this entry makes: a dual land's two abilities
                // rendered as byte-identical rows, a filter land's as six,
                // with no way to choose a colour (issue #118).
                let desc = view.battlefield.iter()
                    .find(|p| p.object_id == *object_id)
                    .and_then(|p| p.mana_abilities.iter()
                        .find(|(i, _)| i == ability_index)
                        .map(|(_, d)| d.clone()));
                match desc {
                    Some(d) => format!("Tap {}: {}", Self::perm_name(view, *object_id), d),
                    None => format!("Tap {} for mana", Self::perm_name(view, *object_id)),
                }
            }
            Action::ActivateAbility { object_id, targets, .. } =>
                format!("Activate ability: {}{}", Self::perm_name(view, *object_id),
                    Self::targets_suffix(view, targets)),
            Action::DeclareAttackers { attackers, planeswalker_attacks } => {
                if attackers.is_empty() && planeswalker_attacks.is_empty() { "Don't attack".into() }
                else {
                    let names: Vec<String> = attackers.iter()
                        .map(|(id, _)| Self::perm_name(view, *id)).collect();
                    format!("Attack with {}", names.join(", "))
                }
            }
            Action::DeclareBlockers { assignments } => {
                if assignments.is_empty() { "Don't block".into() }
                else {
                    let descs: Vec<String> = assignments.iter()
                        .map(|(b, a)| format!("{} blocks {}", Self::perm_name(view, *b), Self::perm_name(view, *a)))
                        .collect();
                    format!("Block: {}", descs.join(", "))
                }
            }
            Action::DiscardCards { cards } => {
                let names: Vec<String> = cards.iter()
                    .map(|id| Self::perm_name(view, *id)).collect();
                format!("Discard {}", names.join(", "))
            }
            Action::MulliganKeep => "Keep opening hand".into(),
            Action::MulliganMull => "Mulligan".into(),
            Action::BottomCards { cards } => {
                let names: Vec<String> = cards.iter()
                    .map(|id| Self::perm_name(view, *id)).collect();
                format!("Bottom {}", names.join(", "))
            }
            Action::Concede => "Concede".into(),
            Action::ActivateLoyaltyAbility { object_id, ability_index, targets } => {
                // Name the ability, not its index: "loyalty ability 1" told
                // the player nothing, and two abilities rendered identically
                // apart from that number (#61).
                let desc = view.battlefield.iter()
                    .find(|p| p.object_id == *object_id)
                    .and_then(|p| p.loyalty_abilities.iter()
                        .find(|(i, _)| i == ability_index)
                        .map(|(_, d)| d.clone()));
                match desc {
                    Some(d) => format!("{}: {}{}", Self::perm_name(view, *object_id),
                        d, Self::targets_suffix(view, targets)),
                    None => format!("Activate loyalty ability {} on {}{}", ability_index,
                        Self::perm_name(view, *object_id), Self::targets_suffix(view, targets)),
                }
            }
            Action::ResolveChoice { choice } => {
                use mtg_engine::actions::ResolvedChoice;
                match choice {
                    ResolvedChoice::PayDecision(true) => "Pay".into(),
                    ResolvedChoice::PayDecision(false) => "Don't pay".into(),
                    ResolvedChoice::YesNoDecision(true) => "Yes".into(),
                    ResolvedChoice::YesNoDecision(false) => "No".into(),
                    ResolvedChoice::ChosenTarget(Some(t)) => {
                        match t {
                            mtg_engine::actions::Target::Object(id) => Self::perm_name(view, *id),
                            mtg_engine::actions::Target::Player(pid) => {
                                if *pid == view.you { "You".into() } else { "Opponent".into() }
                            }
                            mtg_engine::actions::Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
                        }
                    }
                    ResolvedChoice::ChosenTarget(None) => "Decline (do nothing)".into(),
                    ResolvedChoice::ChosenCard(id) => Self::perm_name(view, *id),
                    ResolvedChoice::ChosenIndex(_, ref label) => {
                        label.clone()
                    }
                    ResolvedChoice::ChosenOrder(order) => format!("Order: {}",
                        order.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")),
                    ResolvedChoice::ChosenSubset(ids) => {
                        let names: Vec<String> = ids.iter()
                            .map(|id| Self::perm_name(view, *id))
                            .collect();
                        format!("Pile 1: [{}]", if names.is_empty() { "empty".into() } else { names.join(", ") })
                    }
                    ResolvedChoice::XFunding(response) => format!("Fund X = {}", response.x_value()),
                    ResolvedChoice::ChosenTargetSet(ts) => {
                        if ts.is_empty() {
                            "Target: (none)".into()
                        } else {
                            let names: Vec<String> = ts.iter().map(|t| match t {
                                mtg_engine::actions::Target::Object(id) => Self::perm_name(view, *id),
                                mtg_engine::actions::Target::Player(pid) =>
                                    if *pid == view.you { "You".into() } else { "Opponent".into() },
                                mtg_engine::actions::Target::Illegal => "(illegal)".into(),
                            }).collect();
                            format!("Target: {}", names.join(", "))
                        }
                    }
                    ResolvedChoice::ChosenExileSet(ids) => {
                        if ids.is_empty() {
                            "Exile: (none)".into()
                        } else {
                            let names: Vec<String> = ids.iter()
                                .map(|id| Self::perm_name(view, *id))
                                .collect();
                            format!("Exile: [{}]", names.join(", "))
                        }
                    }
                    ResolvedChoice::CancelCast => "Cancel cast".into(),
                }
            }
        }
    }

    // ── Input ──────────────────────────────────────────────────────

    fn read_line(prompt: &str) -> String {
        Self::read_line_redrawing(prompt, &|| {})
    }

    /// [`read_line`](Self::read_line), plus what to do when the terminal is
    /// resized: `redraw` repaints the frame this prompt sits in, and the
    /// prompt and everything typed so far are painted again on top of it.
    ///
    /// The TUI only ever drew when it was about to ask something, and
    /// nothing handled a resize, so after one the screen kept the frame
    /// drawn for the old width — hard-wrapped into nonsense by the terminal
    /// — until the next keystroke. At small sizes the prompt and its options
    /// were not visible at all, and the instinctive key to press is Enter,
    /// which at a priority prompt passes priority (issue #250).
    /// The column the middle panel's right border sits in, laid out exactly
    /// as [`render`](Self::render) lays it out. Content stops before it.
    ///
    /// Every prompt this CLI reads sits inside that panel, so this — not the
    /// terminal's right edge — is where a prompt and the echo of what is
    /// typed into it have to stop. At 100 columns the two are 2-3 columns
    /// apart, which was enough for the exile-cost hint to erase the frame
    /// border, and every character typed after it to erase the CARDS pane
    /// beside it (issue #320; #53 and #109 are the same overrun at the menu
    /// reader, which has had this bound for a while).
    fn middle_panel_edge() -> usize {
        Self::middle_panel_edge_at(Self::term_width())
    }

    fn term_width() -> usize {
        terminal::size().unwrap_or((100, 30)).0 as usize
    }

    /// How many columns of text the middle panel holds at terminal width
    /// `w`, laid out exactly as [`render`](Self::render) lays it out.
    /// Anything drawn into that panel — a row, a prompt, the echo of what is
    /// typed into a prompt — has this much room and no more. Taking the
    /// width as an argument is what lets the widths that matter be stated
    /// without a terminal to measure.
    fn middle_panel_width_at(w: usize) -> usize {
        let has_right = w >= 100;
        let gutter_w = w / 5;
        let right_w = if has_right { gutter_w } else { 0 };
        w.saturating_sub(gutter_w + right_w + if has_right { 2 } else { 1 })
    }

    fn middle_panel_edge_at(w: usize) -> usize {
        w / 5 + 1 + Self::middle_panel_width_at(w)
    }

    fn read_line_redrawing(prompt: &str, redraw: &dyn Fn()) -> String {
        // ONE reader for the terminal, always. This used to be a cooked-mode
        // io::stdin() read while every menu prompt reads crossterm events in
        // raw mode; two buffered readers over one fd desynchronize, and a
        // stale newline sitting in stdin's BufReader answered the
        // declare-blockers prompt as an empty line — a full 12-pair block
        // declaration became "declared no blockers" in a game-deciding
        // combat (issue #91; the concede prompt ate keystrokes across the
        // same boundary in #42). Reading key events in raw mode, like every
        // other prompt, removes the second reader outright. Leaves the
        // terminal cooked, as the old read did.
        let mut out = stdout();
        // Clipped to the panel it is drawn in, like every other row of it —
        // the caller has already put the cursor at the panel's content
        // column, which is what `mid_print` and the picker's own `clip`
        // assume too. This row was handed straight to `Print`, so a hint 3
        // columns wider than the panel drew over the frame's own border, and
        // then over the CARDS pane beside it (issue #320).
        let edge = Self::middle_panel_edge();
        let shown: String = prompt.chars().take(Self::middle_panel_width_at(Self::term_width()))
            .collect();
        let _ = execute!(out, Print(&shown));
        let _ = out.flush();
        tui_raw_on();
        // Same paste hardening as the menu reader (#50): a multi-line paste
        // must not submit on its embedded newlines.
        let _ = execute!(out, event::EnableBracketedPaste);
        // Pending type-ahead is dropped — the cooked read's mode switch did
        // this by accident, #71 does it on purpose: a keystroke must never
        // answer a prompt the player has not been shown.
        while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            let _ = event::read();
        }
        // The echo stops at the terminal's right edge minus one: an
        // unbounded echo let a 5000-character paste wrap across the whole
        // pane and scroll the frame away (issue #109; same cap idea as the
        // menu reader's, #53). Input beyond the cap still lands in `buf`,
        // it just isn't painted.
        let (mut start_col, mut start_row) = cursor::position().unwrap_or((0, 0));
        let mut echo_cap = edge.saturating_sub(start_col as usize + 1);
        let mut buf = String::new();
        loop {
            let Some(ev) = read_event_guarded() else { continue };
            if let Event::Resize(w, _) = ev {
                // Repaint the frame at the new size, then this prompt and
                // whatever has been typed into it.
                redraw();
                // The panel is a different width now, so the prompt is
                // re-clipped to the new one rather than to the old.
                let reshown: String = prompt.chars()
                    .take(Self::middle_panel_width_at(w as usize)).collect();
                let _ = execute!(out, Print(&reshown));
                let _ = out.flush();
                (start_col, start_row) = cursor::position().unwrap_or((start_col, start_row));
                echo_cap = Self::middle_panel_edge_at(w as usize)
                    .saturating_sub(start_col as usize + 1);
                repaint_input_line(&mut out, start_col, start_row, &buf, echo_cap);
                continue;
            }
            if let Event::Paste(pasted) = &ev {
                let first = pasted.split(['\r', '\n']).next().unwrap_or("");
                buf.push_str(first);
                repaint_input_line(&mut out, start_col, start_row, &buf, echo_cap);
                continue;
            }
            let Event::Key(KeyEvent { code, modifiers, .. }) = ev else { continue };
            match code {
                KeyCode::Enter => break,
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    quit_at_prompt();
                }
                // Typing, editing (Ctrl-U kills the line, #79), and the
                // unbound keys that separate rather than vanish (#51, #322)
                // — one definition, shared with the menu reader.
                _ => {
                    if edit_line(&mut buf, code, modifiers) {
                        repaint_input_line(&mut out, start_col, start_row, &buf, echo_cap);
                    }
                }
            }
        }
        let _ = execute!(out, event::DisableBracketedPaste, Print("\r\n"));
        tui_raw_off();
        buf.trim().to_string()
    }


    /// A y/n confirmation that answers on a single keypress: `y` confirms,
    /// `n` or Esc declines, anything else visibly re-prompts. Runs in raw
    /// mode with explicit echo, so a stray keystroke can never sit
    /// invisibly in a line buffer (issue #42).
    fn confirm_yn(prompt: &str) -> bool {
        let mut out = stdout();
        // Remember where the prompt starts: a rejected key redraws THIS row
        // in place. The old reprompt printed a fresh row per junk key,
        // eating the LOG panel one row at a time (issue #125).
        let (px, py) = cursor::position().unwrap_or((0, 20));
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let _ = execute!(out, Print(prompt));
        let _ = out.flush();
        let was_raw = terminal::is_raw_mode_enabled().unwrap_or(false);
        tui_raw_on();
        // Line-buffered, like every other prompt in this program. Reading a
        // single key meant the first 'y' ANYWHERE in what the player was
        // typing ended the game: "maybe" conceded on its third character,
        // with no Enter, while they were still typing (issue #249). The
        // answer is short, but "the keystroke that ends the game" cannot be
        // one the player has not finished choosing.
        let echo_col = px + u16::try_from(prompt.chars().count()).unwrap_or(0);
        let echo_cap = (term_w as usize).saturating_sub(echo_col as usize + 1);
        let mut buf = String::new();
        let answer = loop {
            let Some(Event::Key(KeyEvent { code, modifiers, .. })) = read_event_guarded() else {
                continue;
            };
            match code {
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    quit_at_prompt();
                }
                // Escape is "no" on its own — it is not text, so it needs no
                // Enter.
                KeyCode::Esc => break false,
                KeyCode::Backspace => {
                    buf.pop();
                    repaint_input_line(&mut out, echo_col, py, &buf, echo_cap);
                }
                KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                    buf.clear();
                    repaint_input_line(&mut out, echo_col, py, &buf, echo_cap);
                }
                KeyCode::Char(c) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                    buf.push(c);
                    repaint_input_line(&mut out, echo_col, py, &buf, echo_cap);
                }
                KeyCode::Enter => {
                    match buf.trim().to_lowercase().as_str() {
                        "y" | "yes" => break true,
                        "n" | "no" | "" => break false,
                        _ => {
                            let msg = format!("Please answer y or n. {}", prompt.trim_start());
                            let clipped: String = msg.chars()
                                .take((term_w as usize).saturating_sub(px as usize + 1))
                                .collect();
                            let _ = execute!(stdout(), cursor::MoveTo(px, py),
                                Clear(ClearType::UntilNewLine), Print(clipped));
                            let _ = stdout().flush();
                            buf.clear();
                        }
                    }
                }
                _ => {}
            }
        };
        if !was_raw {
            tui_raw_off();
        }
        let _ = execute!(stdout(), Print("\r\n"));
        answer
    }

    /// Read a line of input, but detect '/' immediately (without Enter)
    /// to trigger card search. Returns None if '/' was pressed first.
    /// The action-menu reader. `redraw` repaints the menu when the terminal
    /// is resized (issue #250 — see
    /// [`read_line_redrawing`](Self::read_line_redrawing)).
    fn read_line_with_search_redrawing(_col: u16, redraw: &dyn Fn()) -> Option<String> {
        // Prompt "> " is already printed by render.
        let mut out = stdout();
        tui_raw_on();
        let mut buf = String::new();

        // The echo stops at the middle panel's right edge (same layout math
        // as `render`): an unbounded echo let one long pasted line wrap over
        // the card panel and scroll the whole UI away (#53). Input beyond
        // the cap still lands in `buf`, it just isn't painted.
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let w = term_w as usize;
        let has_right = w >= 100;
        let gutter = w / 5;
        let mid_w = w.saturating_sub(gutter + if has_right { gutter + 2 } else { 1 });
        let echo_cap = mid_w.saturating_sub("  > ".len());
        // The line is repainted from `buf` (see `repaint_input_line`), so
        // there is no parallel echo model to drift from it (#281).
        let (mut start_col, mut start_row) = cursor::position().unwrap_or((0, 0));

        // Bracketed paste, enabled only for this raw-mode read: without it a
        // multi-line paste arrives as N keystroke sequences whose embedded
        // newlines SUBMIT — one stray paste answered thirty prompts, made
        // cleanup discards for both seats, and advanced eleven turns (#50).
        // With it the terminal delivers the paste as one event, the first
        // line lands in the buffer, and nothing submits until a real Enter.
        let _ = execute!(out, event::EnableBracketedPaste);

        // An event peeked for the `rr` chord that turned out not to be the
        // second `r` is handled here rather than dropped: consuming it
        // destroyed whatever the player typed next, Enter included, inside a
        // 300 ms window (issue #284).
        let mut pending: Option<Event> = None;
        let result = loop {
            let ev = match pending.take() {
                Some(ev) => ev,
                None => match read_event_guarded() {
                    Some(ev) => ev,
                    None => continue,
                },
            };
            if let Event::Resize(..) = ev {
                redraw();
                (start_col, start_row) = cursor::position().unwrap_or((start_col, start_row));
                repaint_input_line(&mut out, start_col, start_row, &buf, echo_cap);
                continue;
            }
            if let Event::Paste(pasted) = &ev {
                let first = pasted.split(['\r', '\n']).next().unwrap_or("");
                buf.push_str(first);
                repaint_input_line(&mut out, start_col, start_row, &buf, echo_cap);
                continue;
            }
            if let Event::Key(KeyEvent { code, modifiers, .. }) = ev {
                match code {
                    KeyCode::Char('/') if buf.is_empty() && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        break None; // trigger card search
                    }
                    KeyCode::Char('r') if buf.is_empty() && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        // Wait briefly for a second 'r' to trigger hot reload.
                        // Anything else that arrives in the window is put
                        // back for the next turn of the loop, not eaten.
                        if event::poll(std::time::Duration::from_millis(300)).unwrap_or(false) {
                            match read_event_guarded() {
                                Some(Event::Key(KeyEvent { code: KeyCode::Char('r'), .. })) => {
                                    HOT_RELOAD_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
                                    tui_raw_off();
                                    break Some("__hot_reload__".into());
                                }
                                other => pending = other,
                            }
                        }
                        // Single 'r' — treat as normal input.
                        buf.push('r');
                        repaint_input_line(&mut out, start_col, start_row, &buf, echo_cap);
                    }
                    KeyCode::Enter => {
                        // Trimmed, like the other reader's line: a separator
                        // key pressed after the number (#322) must not turn
                        // "0" into a refused "0 ".
                        break Some(buf.trim().to_string());
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        quit_at_prompt();
                    }
                    // Typing; Ctrl-U to kill the line — the standard readline
                    // binding and the documented recovery from a garbled
                    // prompt (issue #79); and every unbound key as a
                    // separator, never typed (#51) and never silently
                    // dropped between two digits (#322). See `edit_line`.
                    _ => {
                        if edit_line(&mut buf, code, modifiers) {
                            repaint_input_line(&mut out, start_col, start_row, &buf, echo_cap);
                        }
                    }
                }
            }
        };


        let _ = execute!(stdout(), event::DisableBracketedPaste);
        tui_raw_off();
        result
    }

    fn show_battlefield_inspector(view: &GameView) {
        // No registry lookup: everything this page shows about a permanent
        // comes from the view, which resolves the face that is up. Reading
        // the registry by `card_id` is what gave a transformed permanent its
        // front face's text and P/T (issue #240).
        let mut out = stdout();

        loop {
            let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
            Self::print_colored(&mut out, Color::Cyan, " INSPECT BATTLEFIELD");
            let _ = execute!(out, Print("\n"));

            let your_perms: Vec<&PermanentView> = view.battlefield.iter()
                .filter(|p| p.controller == view.you).collect();
            let opp_perms: Vec<&PermanentView> = view.battlefield.iter()
                .filter(|p| p.controller != view.you).collect();

            let _ = execute!(out, SetAttribute(Attribute::Bold),
                Print(" Your permanents:\n"), SetAttribute(Attribute::Reset));
            let mut idx = 0;
            for perm in &your_perms {
                let pt = match (perm.effective_power, perm.effective_toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                let flags = format!("{}{}",
                    if perm.tapped { " [T]" } else { "" },
                    if Self::is_summoning_sick(perm) { " [S]" } else { "" });
                let loyalty = if perm.card_types.contains(&CardType::Planeswalker) {
                    let l = perm.counters.get(&mtg_engine::types::CounterType::Loyalty)
                        .copied().unwrap_or(0);
                    format!(" [{l} loyalty]")
                } else { String::new() };
                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {idx:>2}")),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}{}{}{}\n", perm.name, pt, loyalty, flags)));
                idx += 1;
            }

            let _ = execute!(out, Print("\n"));
            let _ = execute!(out, SetAttribute(Attribute::Bold),
                Print(" Opponent's permanents:\n"), SetAttribute(Attribute::Reset));
            for perm in &opp_perms {
                let pt = match (perm.effective_power, perm.effective_toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                let flags = format!("{}{}",
                    if perm.tapped { " [T]" } else { "" },
                    if Self::is_summoning_sick(perm) { " [S]" } else { "" });
                let loyalty = if perm.card_types.contains(&CardType::Planeswalker) {
                    let l = perm.counters.get(&mtg_engine::types::CounterType::Loyalty)
                        .copied().unwrap_or(0);
                    format!(" [{l} loyalty]")
                } else { String::new() };
                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {idx:>2}")),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}{}{}{}\n", perm.name, pt, loyalty, flags)));
                idx += 1;
            }

            let all_perms: Vec<&PermanentView> = your_perms.iter().chain(opp_perms.iter()).copied().collect();

            let _ = execute!(out, Print("\n  Enter number for details, or press enter to return: "));
            let _ = out.flush();
            let input = Self::read_line("");

            if input.is_empty() { return; }

            if let Ok(i) = input.parse::<usize>() {
                if i < all_perms.len() {
                    let perm = all_perms[i];
                    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
                    Self::print_colored(&mut out, Color::Cyan, &format!(" {}", perm.name));

                    // CR 205.1: the type line is supertypes, types AND
                    // subtypes, and the subtypes are the live ones — the
                    // printed ones plus anything an effect granted. Every "as
                    // long as ... is a Human" card in the set turns on a fact
                    // this page used to refuse to state (issue #297), and
                    // "Legendary" — the word that arms the legend rule — was
                    // printed nowhere in the game (issue #333).
                    let type_line = mtg_engine::types::type_line(
                        &perm.supertypes, &perm.card_types, &perm.subtypes);
                    let _ = execute!(out, Print(format!("  Type: {type_line}\n")));


                    // The permanent's live keywords and protections, which
                    // the view has always computed and no pane ever printed:
                    // a flying token rendered as a ground creature and a
                    // creature that had lost defender still read "Defender"
                    // (issues #243, #297).
                    let mut abilities: Vec<String> = perm.keywords.iter()
                        .map(|k| format!("{k:?}"))
                        .collect();
                    abilities.extend(perm.protections.iter().cloned());
                    if !abilities.is_empty() {
                        let _ = execute!(out, Print(format!("  Keywords: {}\n", abilities.join(", "))));
                    }

                    // What the card says, from the face that is up. This
                    // used to print the object's own fields, which are the
                    // FRONT face's for a transformed DFC (issue #240), the
                    // `Some(0)` sentinel for a `*/*` creature (#267), and —
                    // before Tree of Redemption's exchange became a layer-7b
                    // effect — whatever an effect had written over them
                    // (#302). "Printed", because that is the question this
                    // line answers; everything else is on the next one.
                    if perm.star_pt {
                        let _ = execute!(out, Print("  Printed P/T: */*\n".to_string()));
                    } else if let (Some(p), Some(t)) = (perm.printed_power, perm.printed_toughness) {
                        let _ = execute!(out, Print(format!("  Printed P/T: {p}/{t}\n")));
                    }
                    if let (Some(p), Some(t)) = (perm.effective_power, perm.effective_toughness) {
                        let _ = execute!(out, Print(format!("  Effective P/T: {p}/{t}\n")));
                    }
                    if perm.damage_marked > 0 {
                        let _ = execute!(out, Print(format!("  Damage marked: {}\n", perm.damage_marked)));
                    }
                    if perm.card_types.contains(&CardType::Planeswalker) {
                        let l = perm.counters.get(&mtg_engine::types::CounterType::Loyalty)
                            .copied().unwrap_or(0);
                        let _ = execute!(out, Print(format!("  Loyalty: {l}\n")));
                    }
                    // Counters are public information (CR 122.3) and this
                    // page is where a player checks them (issue #82).
                    let counters = Self::counters_suffix(&perm.counters);
                    if !counters.is_empty() {
                        let _ = execute!(out, Print(format!("  Counters:{counters}\n")));
                    }

                    let controller = if perm.controller == view.you { "You" } else { "Opponent" };
                    let _ = execute!(out, Print(format!("  Controller: {controller}\n")));
                    let _ = execute!(out, Print(format!("  Tapped: {}\n", perm.tapped)));
                    if Self::is_summoning_sick(perm) {
                        let _ = execute!(out, Print("  Summoning sick: true\n".to_string()));
                    }
                    let _ = execute!(out, Print(format!("  ID: #{}\n", perm.object_id.0)));

                    // Combat role (CR 506.3a, 509.1a). The page used to say
                    // only "Tapped: true", which is what a creature tapped
                    // for mana says too (issue #245).
                    let named = |id: mtg_engine::ids::ObjectId| -> String {
                        view.battlefield.iter().find(|p| p.object_id == id)
                            .map_or_else(|| format!("#{}", id.0), |p| format!("{} (#{})", p.name, id.0))
                    };
                    match &perm.attacking {
                        Some(mtg_engine::view::AttackTarget::Player(p)) => {
                            let who = if *p == view.you { "you" } else { "your opponent" };
                            let _ = execute!(out, Print(format!("  Attacking: {who}\n")));
                        }
                        Some(mtg_engine::view::AttackTarget::Planeswalker(w)) => {
                            let _ = execute!(out, Print(format!("  Attacking: {}\n", named(*w))));
                        }
                        None => {}
                    }
                    if !perm.blocking.is_empty() {
                        let names: Vec<String> = perm.blocking.iter().map(|&a| named(a)).collect();
                        let _ = execute!(out, Print(format!("  Blocking: {}\n", names.join(", "))));
                    }
                    if !perm.blocked_by.is_empty() {
                        let names: Vec<String> = perm.blocked_by.iter().map(|&b| named(b)).collect();
                        let _ = execute!(out, Print(format!("  Blocked by: {}\n", names.join(", "))));
                    }

                    // Attachments, by what they are: an Aura enchants
                    // (CR 303.4), an Equipment equips (CR 301.5c) — the one
                    // label for both called a Pike an enchantment (#83).
                    let (auras, equipment): (Vec<&PermanentView>, Vec<&PermanentView>) =
                        view.battlefield.iter()
                            .filter(|p| p.attached_to == Some(perm.object_id))
                            .partition(|p| p.card_types.contains(&CardType::Enchantment));
                    if !auras.is_empty() {
                        let names: Vec<&str> = auras.iter().map(|a| a.name.as_str()).collect();
                        let _ = execute!(out, Print(format!("  Enchanted by: {}\n", names.join(", "))));
                    }
                    if !equipment.is_empty() {
                        let names: Vec<&str> = equipment.iter().map(|a| a.name.as_str()).collect();
                        let _ = execute!(out, Print(format!("  Equipped with: {}\n", names.join(", "))));
                    }

                    if let Some(att) = perm.attached_to {
                        let att_name = view.battlefield.iter()
                            .find(|p| p.object_id == att)
                            .map_or("?", |p| p.name.as_str());
                        let _ = execute!(out, Print(format!("  Attached to: {att_name}\n")));
                    }
                    // A Curse names its player (CR 702.5c) — issue #81.
                    if let Some(p) = perm.attached_to_player {
                        let who = if p == view.you { "You" } else { "Opponent" };
                        let _ = execute!(out, Print(format!("  Enchanting: {who}\n")));
                    }

                    // Show the oracle text of the face that is up. Looking
                    // it up by `card_id` gave the FRONT card's text, so a
                    // transformed Cloistered Youth was headed "Unholy Fiend"
                    // and then described as a Cloistered Youth — while the
                    // engine fired the back face's ability (issue #240). The
                    // view already resolves the active face.
                    if !perm.oracle_text.is_empty() {
                        let _ = execute!(out, Print("\n"),
                            SetForegroundColor(Color::Yellow),
                            Print(format!("  {}\n", perm.oracle_text)),
                            ResetColor);
                    }

                    let _ = execute!(out, Print("\n  Press enter to return to list..."));
                    let _ = out.flush();
                    let _ = Self::read_line("");
                }
            }
        }
    }

    /// Compute the visible page window: `(start, end, page_size)` for a
    /// list of `len` lines on a terminal `term_h` rows tall, given the
    /// current `page` (0-based). Pulled out of the pager so the arithmetic
    /// is testable without a terminal.
    fn page_window(len: usize, term_h: usize, page: usize) -> (usize, usize, usize) {
        let page_size = term_h.saturating_sub(4).max(1);
        let last_page = if len == 0 { 0 } else { (len - 1) / page_size };
        let page = page.min(last_page);
        let start = page * page_size;
        let end = (start + page_size).min(len);
        (start, end, page_size)
    }

    /// Full-screen paged line viewer shared by the `l`/`g`/`e` info views.
    /// The old printers dumped every line unclamped, so anything taller
    /// than the terminal scrolled off the top with no way back and no
    /// notice (issues #101/#102). This clamps to the terminal height,
    /// pages with n/p, and always says which slice is showing.
    /// `start_at_end` opens on the last page (the log's most recent
    /// entries); the list views open at the top.
    fn show_paged_lines(title: &str, lines: &[InfoLine], start_at_end: bool) {
        let mut out = stdout();
        let h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let (_, _, page_size) = Self::page_window(lines.len(), h, 0);
        let mut page = if start_at_end && !lines.is_empty() {
            (lines.len() - 1) / page_size
        } else {
            0
        };
        loop {
            let (start, end, page_size) = Self::page_window(lines.len(), h, page);
            let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
            let heading = if lines.len() > page_size {
                format!("{} (showing {}-{} of {})", title, start + 1, end, lines.len())
            } else {
                title.to_string()
            };
            Self::print_colored(&mut out, Color::Cyan, &heading);
            let _ = execute!(out, Print("\n"));
            for line in &lines[start..end] {
                match line {
                    InfoLine::Plain(s) => {
                        let _ = execute!(out, Print(format!("{s}\n")));
                    }
                    InfoLine::Bold(s) => {
                        let _ = execute!(out, SetAttribute(Attribute::Bold),
                            Print(format!("{s}\n")), SetAttribute(Attribute::Reset));
                    }
                    InfoLine::Dim(s) => {
                        let _ = execute!(out, SetAttribute(Attribute::Dim),
                            Print(format!("{s}\n")), SetAttribute(Attribute::Reset));
                    }
                    InfoLine::Mana(s) => {
                        let _ = execute!(out, Print("   "));
                        Self::print_with_mana(&mut out, s, None);
                        let _ = execute!(out, Print("\n"));
                    }
                }
            }
            let footer = if lines.len() > page_size {
                "  n=next page, p=previous, enter=return: "
            } else {
                "  Press enter to return..."
            };
            let _ = execute!(out, Print(footer));
            let _ = out.flush();
            match Self::read_line("").trim() {
                "n" if end < lines.len() => page += 1,
                "n" => {}
                "p" => page = page.saturating_sub(1),
                _ => return,
            }
        }
    }

    /// Full-screen graveyards view, shared by the menu's `g` shortcut and
    /// the combat prompts (issue #120).
    fn show_graveyards(view: &GameView) {
        let mut lines: Vec<InfoLine> = Vec::new();
        for (pid, cards) in &view.graveyards {
            let who = if *pid == view.you { "Your" } else { "Opponent's" };
            lines.push(InfoLine::Bold(format!(" {} graveyard ({}):", who, cards.len())));
            if cards.is_empty() {
                lines.push(InfoLine::Plain("   (empty)".into()));
            } else {
                for card in cards {
                    let cost = card.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
                    let pt = match (card.power, card.toughness) {
                        (Some(p), Some(t)) => format!(" {p}/{t}"),
                        _ => String::new(),
                    };
                    lines.push(InfoLine::Mana(format!("{}{}{}", card.name, cost, pt)));
                }
            }
            lines.push(InfoLine::Plain(String::new()));
        }
        Self::show_paged_lines(" GRAVEYARDS", &lines, false);
    }

    /// Full-screen exile view, shared like `show_graveyards` (issue #120).
    /// The headline of one stack entry: what it is, whose it is, and its
    /// announced X.
    ///
    /// X is announced as the spell is cast (CR 601.2b) and the stack is a
    /// public zone (CR 400.2), so both seats are entitled to it — a Devil's
    /// Play for 12 and one for 0 used to be character-for-character
    /// identical on screen, which made responding to an X spell guesswork
    /// (issue #259).
    fn stack_entry_headline(view: &GameView, item: &mtg_engine::view::StackItemView) -> String {
        let who = if item.controller == view.you { "you" } else { "opp" };
        match item.x_value {
            Some(x) => format!("{} (X={x}) ({who})", item.name),
            None => format!("{} ({who})", item.name),
        }
    }

    /// " -> Grizzly Bears 2/2 (opp)" for one chosen target.
    fn stack_target_line(view: &GameView, target: &Target) -> String {
        match target {
            // perm_name carries the (your)/(opp) marker and resolves
            // non-battlefield objects too (#100).
            Target::Object(id) => format!(" -> {}", Self::perm_name(view, *id)),
            Target::Player(pid) =>
                if *pid == view.you { " -> you".into() } else { " -> opp".into() },
            Target::Illegal =>
                unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
        }
    }

    /// The whole stack, paged, top first.
    ///
    /// The STACK panel is a third of the pane tall and used to render into it
    /// until it ran out of rows and then stop — no count, no marker, and the
    /// last entry cut off mid-entry, so a 94-object stack looked like a
    /// two-and-a-half object one. CR 405.1 makes the stack public in full,
    /// and there was no other view in the CLI that showed it (issue #247).
    fn show_stack(view: &GameView) {
        let mut lines: Vec<InfoLine> = Vec::new();
        if view.stack.is_empty() {
            lines.push(InfoLine::Plain("  (empty)".into()));
        } else {
            lines.push(InfoLine::Bold(format!(
                " {} object(s) on the stack, top first:", view.stack.len())));
            for (i, item) in view.stack.iter().enumerate() {
                lines.push(InfoLine::Plain(format!(
                    "  {i}: {}", Self::stack_entry_headline(view, item))));
                for target in &item.targets {
                    lines.push(InfoLine::Dim(format!("     {}",
                        Self::stack_target_line(view, target).trim_start())));
                }
            }
        }
        Self::show_paged_lines(" STACK", &lines, false);
    }

    fn show_exile(view: &GameView) {
        let mut lines: Vec<InfoLine> = Vec::new();
        let your_exile: Vec<_> = view.exile.iter().filter(|c| c.owner == view.you).collect();
        let opp_exile: Vec<_> = view.exile.iter().filter(|c| c.owner != view.you).collect();
        for (who, cards) in [("Your", &your_exile), ("Opponent's", &opp_exile)] {
            lines.push(InfoLine::Bold(format!(" {} exile ({}):", who, cards.len())));
            if cards.is_empty() {
                lines.push(InfoLine::Plain("   (empty)".into()));
            } else {
                for card in cards.iter() {
                    let cost = card.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
                    let pt = match (card.power, card.toughness) {
                        (Some(p), Some(t)) => format!(" {p}/{t}"),
                        _ => String::new(),
                    };
                    lines.push(InfoLine::Plain(format!("   {}{}{}", card.name, cost, pt)));
                }
            }
            lines.push(InfoLine::Plain(String::new()));
        }
        Self::show_paged_lines(" EXILE", &lines, false);
    }

    fn show_log(log: &[String]) {
        let lines: Vec<InfoLine> = if log.is_empty() {
            vec![InfoLine::Plain("  (no events yet)".into())]
        } else {
            log.iter().map(|e| InfoLine::Dim(format!("  {e}"))).collect()
        };
        // Open on the final page: the most recent events are what a player
        // pressing `l` mid-game is usually after.
        Self::show_paged_lines(" GAME LOG", &lines, true);
    }

    fn show_deck_browser(view: &GameView) {
        let registry = mtg_engine::cards::CardRegistry::with_all_cards();
        let mut out = stdout();

        // Count per-zone for each card name the player owns.
        let mut hand_counts: HashMap<String, usize> = HashMap::new();
        let mut board_counts: HashMap<String, usize> = HashMap::new();
        let mut gy_counts: HashMap<String, usize> = HashMap::new();
        let mut exile_counts: HashMap<String, usize> = HashMap::new();

        for card in &view.your_hand {
            *hand_counts.entry(card.name.clone()).or_default() += 1;
        }
        for perm in &view.battlefield {
            if perm.controller != view.you {
                continue;
            }
            // CR 111.1: a token is not a card and does not belong in a deck
            // count. And a permanent showing its back face is still the card
            // it was printed as — counting it under the back face's name put
            // it in the header's total and then dropped it from the list,
            // because no card in the registry has that name (issue #241).
            if perm.is_token {
                continue;
            }
            let printed = registry.card_data(perm.card_id)
                .map_or_else(|| perm.name.clone(), |d| d.name);
            *board_counts.entry(printed).or_default() += 1;
        }
        for (pid, cards) in &view.graveyards {
            if *pid == view.you {
                for card in cards {
                    *gy_counts.entry(card.name.clone()).or_default() += 1;
                }
            }
        }
        for card in &view.exile {
            if card.owner == view.you {
                *exile_counts.entry(card.name.clone()).or_default() += 1;
            }
        }

        // Collect all card names the player owns in any visible zone.
        let mut all_names: Vec<String> = Vec::new();
        for map in [&hand_counts, &board_counts, &gy_counts, &exile_counts] {
            for name in map.keys() {
                if !all_names.contains(name) {
                    all_names.push(name.clone());
                }
            }
        }

        // Library cards
        let mut lib_counts: HashMap<String, usize> = HashMap::new();
        for card in &view.your_library_cards {
            *lib_counts.entry(card.name.clone()).or_default() += 1;
            if !all_names.contains(&card.name) {
                all_names.push(card.name.clone());
            }
        }

        let total_cards: usize = hand_counts.values().sum::<usize>()
            + board_counts.values().sum::<usize>()
            + gy_counts.values().sum::<usize>()
            + exile_counts.values().sum::<usize>()
            + lib_counts.values().sum::<usize>();

        let mut page = 0usize;
        loop {
            let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

            let mut cards: Vec<mtg_engine::cards::CardData> = Vec::new();
            for name in &all_names {
                if let Some(id) = registry.get_id_by_name(name) {
                    if let Some(data) = registry.card_data(id) {
                        cards.push(data);
                    }
                }
            }
            cards.sort_by(|a, b| a.name.cmp(&b.name));

            let deck_cards: Vec<&mtg_engine::cards::CardData> = cards.iter().collect();

            // Clamp to the terminal height and page — an unclamped list
            // scrolled the header and the first entries off the top with
            // no way to reach them (issue #102).
            let h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
            let (start, end, page_size) = Self::page_window(deck_cards.len(), h, page);
            page = start / page_size;
            let heading = if deck_cards.len() > page_size {
                format!(" YOUR DECK ({total_cards} cards, showing {}-{} of {} entries)",
                    start + 1, end, deck_cards.len())
            } else {
                format!(" YOUR DECK ({total_cards} cards)")
            };
            Self::print_colored(&mut out, Color::Cyan, &heading);
            let _ = execute!(out, Print("\n"));

            for (i, data) in deck_cards.iter().enumerate().take(end).skip(start) {
                let cost = data.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
                let pt = match (data.power, data.toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                let h = hand_counts.get(&data.name).copied().unwrap_or(0);
                let b = board_counts.get(&data.name).copied().unwrap_or(0);
                let g = gy_counts.get(&data.name).copied().unwrap_or(0);
                let e = exile_counts.get(&data.name).copied().unwrap_or(0);
                let lib = lib_counts.get(&data.name).copied().unwrap_or(0);
                let total = h + b + g + e + lib;

                // Build location breakdown
                let mut locs = Vec::new();
                if h > 0 { locs.push(format!("{h}hand")); }
                if b > 0 { locs.push(format!("{b}board")); }
                if g > 0 { locs.push(format!("{g}gy")); }
                if e > 0 { locs.push(format!("{e}exile")); }
                if lib > 0 { locs.push(format!("{lib}lib")); }
                let loc_str = if locs.is_empty() { String::new() } else { format!(" ({})", locs.join(", ")) };

                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {i:>2}")),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}x {}{}{}{}\n", total, data.name, cost, pt, loc_str)));
            }

            let footer = if deck_cards.len() > page_size {
                "\n  Enter number for details, n=next page, p=previous, enter=return: "
            } else {
                "\n  Enter number for details, or press enter to return: "
            };
            let _ = execute!(out, Print(footer));
            let _ = out.flush();
            let input = Self::read_line("");

            if input.is_empty() { return; }
            match input.trim() {
                "n" if end < deck_cards.len() => { page += 1; continue; }
                "n" => continue,
                "p" => { page = page.saturating_sub(1); continue; }
                _ => {}
            }

            if let Ok(idx) = input.parse::<usize>() {
                if idx < deck_cards.len() {
                    let data = deck_cards[idx];
                    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
                    Self::print_colored(&mut out, Color::Cyan, &format!(" {}", data.name));
                    let cost = data.cost.as_ref().map_or_else(|| "(none)".into(), |c| format!("{c}"));
                    let _ = execute!(out, Print(format!("  Mana cost: {cost}\n")));
                    // The whole type line as the card prints it (CR 205.1):
                    // supertypes first (issue #333), then types, then the
                    // subtypes — which used to be a separate row.
                    let _ = execute!(out, Print(format!("  Type: {}\n", mtg_engine::types::type_line(
                        &data.supertypes, &data.card_types, &data.subtypes))));

                    if let (Some(p), Some(t)) = (data.power, data.toughness) {
                        let _ = execute!(out, Print(format!("  Power/Toughness: {p}/{t}\n")));
                    }
                    if !data.keywords.is_empty() {
                        let kws: Vec<&str> = data.keywords.iter().map(|k| match k {
                            mtg_engine::types::Keyword::Flying => "Flying",
                            mtg_engine::types::Keyword::FirstStrike => "First strike",
                            mtg_engine::types::Keyword::DoubleStrike => "Double strike",
                            mtg_engine::types::Keyword::Trample => "Trample",
                            mtg_engine::types::Keyword::Deathtouch => "Deathtouch",
                            mtg_engine::types::Keyword::Lifelink => "Lifelink",
                            mtg_engine::types::Keyword::Vigilance => "Vigilance",
                            mtg_engine::types::Keyword::Flash => "Flash",
                            mtg_engine::types::Keyword::Reach => "Reach",
                            mtg_engine::types::Keyword::Haste => "Haste",
                            mtg_engine::types::Keyword::Defender => "Defender",
                            mtg_engine::types::Keyword::Hexproof => "Hexproof",
                            mtg_engine::types::Keyword::Intimidate => "Intimidate",
                            mtg_engine::types::Keyword::Menace => "Menace",
                            mtg_engine::types::Keyword::Indestructible => "Indestructible",
                        }).collect();
                        let _ = execute!(out, SetForegroundColor(Color::Blue),
                            Print(format!("  Keywords: {}\n", kws.join(", "))), ResetColor);
                    }
                    if !data.oracle_text.is_empty() {
                        let _ = execute!(out, SetForegroundColor(Color::Yellow),
                            Print(format!("\n  {}\n", data.oracle_text)), ResetColor);
                    }
                    if let Some(fb) = &data.flashback_cost {
                        let _ = execute!(out, SetForegroundColor(Color::Cyan),
                            Print(format!("  Flashback: {fb}\n")), ResetColor);
                    }
                    let _ = execute!(out, Print("\n  Press enter to return to list..."));
                    let _ = out.flush();
                    let _ = Self::read_line("");
                }
            }
        }
    }

    // ── Combat ─────────────────────────────────────────────────────

    /// Which half of a declare-attackers entry is out of range, or `None`
    /// when every index names something on the screen.
    ///
    /// `N` and `N>pwM` index two different lists — the eligible attackers
    /// and the planeswalkers the defender controls (CR 508.1a). The two
    /// range checks used to be OR'd into one bucket keyed by the ATTACKER
    /// index, so `0>pw1` against one planeswalker answered "Invalid
    /// attacker(s): 0. Valid range is 0-1." — naming an index that was
    /// legal, against the list that was not indexed (issue #287).
    ///
    /// Messages are unprefixed; the caller indents them like every other
    /// refusal in the prompt.
    fn attack_index_error(
        indices: &[usize],
        walker_attacks: &[(usize, usize)],
        eligible_len: usize,
        walkers_len: usize,
    ) -> Option<String> {
        // The creature half first: it is what `eligible[..]` is indexed
        // with, and it stays the named error even when the pw index is bad
        // too. The duplicate-index guard runs before this, so it cannot
        // repeat.
        let bad_attackers: Vec<usize> = indices.iter().copied()
            .chain(walker_attacks.iter().map(|&(a, _)| a))
            .filter(|&a| a >= eligible_len)
            .collect();
        if !bad_attackers.is_empty() {
            return Some(format!("Invalid attacker(s): {}. Valid range is 0-{}.",
                bad_attackers.iter().map(std::string::ToString::to_string)
                    .collect::<Vec<_>>().join(", "),
                eligible_len.saturating_sub(1)));
        }
        // One planeswalker can be named by two attackers ("0>pw9 1>pw9"),
        // so this half needs the dedup the other half gets for free.
        let mut bad_walkers: Vec<usize> = walker_attacks.iter()
            .map(|&(_, w)| w).filter(|&w| w >= walkers_len).collect();
        bad_walkers.sort_unstable();
        bad_walkers.dedup();
        if bad_walkers.is_empty() {
            return None;
        }
        let named = bad_walkers.iter().map(|w| format!("pw{w}"))
            .collect::<Vec<_>>().join(", ");
        Some(if walkers_len == 0 {
            format!("No planeswalker {named} to attack — the defender controls none, \
                     so use a bare number to attack the player.")
        } else {
            format!("No planeswalker {named}. Attackable planeswalkers are pw0-pw{}.",
                walkers_len - 1)
        })
    }

    /// One `blocker:attacker` pair, resolved to indices.
    ///
    /// The range check used to ride on the same match arm as the parse, so
    /// a well-formed pair whose blocker index was live in the OTHER list on
    /// the same screen ("2:0" with two blockers and three attackers) was
    /// answered with "Invalid. Use 'blocker:attacker' pairs like '0:0 1:1'"
    /// — a lecture on the syntax it had just used correctly (issue #289).
    /// "This isn't a pair of numbers" and "this number isn't on the screen"
    /// are different answers.
    fn parse_block_pair(pair: &str, n_blockers: usize, n_attackers: usize)
        -> Result<(usize, usize), String>
    {
        const SYNTAX: &str = "Invalid. Use 'blocker:attacker' pairs like '0:0 1:1'.";
        let parts: Vec<&str> = pair.split(':').collect();
        if parts.len() != 2 {
            return Err(SYNTAX.into());
        }
        let (Ok(b), Ok(a)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) else {
            return Err(SYNTAX.into());
        };
        if b >= n_blockers {
            return Err(if n_blockers == 0 {
                "You have no blockers.".to_string()
            } else {
                format!("No blocker {b}. Your blockers are 0-{}.", n_blockers - 1)
            });
        }
        if a >= n_attackers {
            return Err(if n_attackers == 0 {
                "There are no attackers.".to_string()
            } else {
                format!("No attacker {a}. Attackers are 0-{}.", n_attackers - 1)
            });
        }
        Ok((b, a))
    }

    fn choose_attackers(view: &GameView, prompt: &CombatPrompt) -> Action {
        let CombatPrompt::ChooseAttackers { eligible, must_attack, defending_player: defending,
                                            defending_planeswalkers } = prompt else {
            unreachable!()
        };
        let defending = *defending;

        if eligible.is_empty() {
            return Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] };
        }

        // Layout is computed once; `draw` repaints the whole prompt screen,
        // so the info panes can be offered here and the view restored after
        // one is closed (issue #120).
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = u16::try_from(side + 1).unwrap_or(u16::MAX);

        // The list pages like the action menu does. It used to print straight
        // down from the cursor with no pager and no marker, so on a 26-row
        // pane a player declared attacks from a list showing two of eight
        // creatures, with nothing saying the other six existed (issue #260).
        let list_offset = std::cell::Cell::new(0usize);
        let list_shown = std::cell::Cell::new(0usize);
        // The rows' heights and the lines they had, so `p` can step back
        // exactly over rows of uneven height (issue #318).
        let list_heights: std::cell::RefCell<Vec<usize>> = std::cell::RefCell::new(Vec::new());
        let list_avail = std::cell::Cell::new(0usize);
        let list_marker_h = std::cell::Cell::new(1usize);
        let draw = || -> u16 {

            Self::render(view, Some("DECLARE ATTACKERS"), &view.display_log, "", None);
            let mut out = stdout();
            let mut r = cursor::position().unwrap_or((0, 20)).1;
            let h = terminal::size().map_or(30, |(_, h)| h as usize);
            let _ = execute!(out, cursor::MoveTo(col, r),
                SetForegroundColor(Color::Yellow), SetAttribute(Attribute::Bold),
                Print(" Eligible attackers:"), SetAttribute(Attribute::Reset), ResetColor);
            r += 1;
            let panel_w = Self::middle_panel_width_at(Self::term_width());
            // Rows still owed below the list: the planeswalker block, the
            // hint line (which wraps, so its height is measured, not
            // assumed), the prompt row and the refusal row under it.
            let hint_lines = Self::wrap_indented(ATTACK_HINTS, panel_w);
            let marker_h = Self::marker_lines(
                eligible.len().saturating_sub(1), MENU_PAGE_KEYS, panel_w);
            let reserved = 2 + hint_lines.len() + if defending_planeswalkers.is_empty() {
                0
            } else {
                defending_planeswalkers.len() + 1
            };
            let avail = h.saturating_sub(r as usize + reserved);
            // Every row laid out first, so the page is measured in the lines
            // the rows actually take (issue #318).
            let layouts: Vec<CombatRowLayout> = eligible.iter().enumerate().map(|(i, &id)| {
                let tag = if must_attack.contains(&id) { " [MUST ATTACK]" } else { "" };
                Self::combat_row_layout(view, id, eligible, str_cols(&format!("  {i}: ")), tag, panel_w)
            }).collect();
            let heights: Vec<usize> = layouts.iter().map(CombatRowLayout::height).collect();
            let (offset, shown, paged) =
                Self::menu_page_lines(&heights, avail, list_offset.get(), marker_h);
            list_shown.set(shown);
            list_avail.set(avail);
            list_marker_h.set(marker_h);
            *list_heights.borrow_mut() = heights;
            for (i, &id) in eligible.iter().enumerate().skip(offset).take(shown) {
                let color = if must_attack.contains(&id) { Color::Red } else { Color::Reset };
                Self::draw_combat_row(&mut out, col, &mut r, i,
                    str_cols(&format!("  {i}: ")), &layouts[i], color);
            }

            if paged {
                let marker = Self::page_marker(offset, shown, eligible.len() - 1, MENU_PAGE_KEYS);
                for line in Self::wrap_indented(&marker, panel_w) {
                    let _ = execute!(out, cursor::MoveTo(col, r),
                        SetAttribute(Attribute::Dim), Print(&line), SetAttribute(Attribute::Reset));
                    r += 1;
                }
            }
            if !defending_planeswalkers.is_empty() {
                let _ = execute!(out, cursor::MoveTo(col, r),
                    SetForegroundColor(Color::Yellow),
                    Print(" Attackable planeswalkers (use N>pwM):"), ResetColor);
                r += 1;
                for (i, &id) in defending_planeswalkers.iter().enumerate() {
                    let _ = execute!(out, cursor::MoveTo(col, r),
                        SetAttribute(Attribute::Bold), Print(format!("  pw{i}")),
                        SetAttribute(Attribute::Reset),
                        Print(format!(": {}", Self::perm_name(view, id))));
                    r += 1;
                }
            }
            // The public zones are decision inputs during combat (CR 404.2,
            // 406.3), so the info panes are advertised and accepted here as
            // at every other prompt (issue #120).
            for line in &hint_lines {
                let _ = execute!(out, cursor::MoveTo(col, r),
                    SetAttribute(Attribute::Dim), Print(line), SetAttribute(Attribute::Reset));
                r += 1;
            }
            let _ = execute!(out, cursor::MoveTo(col, r));
            let _ = out.flush();
            r
        };
        let mut r = draw();

        // CR 508.1d: a declaration that leaves out a creature required and
        // able to attack is illegal — refuse it here, loudly, the way the
        // blocker prompt refuses illegal pairings, instead of letting the
        // engine silently auto-correct it with only a side-log trace (#66).
        // The engine's forced-attackers pass stays as the backstop for
        // non-interactive players.
        let missing_forced = |chosen: &[ObjectId]| -> Vec<ObjectId> {
            must_attack.iter().copied().filter(|id| !chosen.contains(id)).collect()
        };
        let forced_error = |missing: &[ObjectId]| -> String {
            let names: Vec<String> = missing.iter().map(|&id| Self::perm_name(view, id)).collect();
            format!("  {} must attack this combat (CR 508.1d) — include {} in the declaration.",
                names.join(", "),
                missing.iter().map(|id| eligible.iter().position(|e| e == id)
                    .map_or("?".into(), |i| i.to_string()))
                    .collect::<Vec<_>>().join(", "))
        };

        // Rejection messages render inside the pane at the prompt row, not
        // via bare println! at column 0 — those landed on the LOG panel and
        // merged with its text into garbage (issue #110).
        let w = term_w as usize;
        let mid_w = if w >= 100 { w.saturating_sub(2 * side + 2) } else { w.saturating_sub(side + 1) };
        // A refusal is drawn under the prompt and LEFT there while the player
        // retypes: it used to be printed, slept on for 900 ms and then
        // erased, so the message was on screen exactly while the program was
        // not listening and gone by the time it was — the "silent re-render
        // is indistinguishable from a hung game" symptom #76 was filed to
        // prevent, on a timer (issue #291). Everywhere else in this CLI a
        // notice is a render input that survives until the next keystroke.
        let paint_notice = |msg: Option<&str>, r: u16| {
            let _ = execute!(stdout(), cursor::MoveTo(col, r + 1), Clear(ClearType::UntilNewLine));
            if let Some(msg) = msg {
                let _ = execute!(stdout(), SetForegroundColor(Color::Red),
                    Print(clip_cols(msg, mid_w)), ResetColor);
            }
            // Back to the prompt row: `read_line` prints its prompt at the
            // cursor, so leaving it here put the prompt on the end of the
            // notice.
            let _ = execute!(stdout(), cursor::MoveTo(col, r));
            let _ = stdout().flush();
        };
        let mut notice: Option<String> = None;

        loop {
            // Clear the row before re-prompting: a rejected entry's characters
            // otherwise stay on screen and visually merge with the next
            // attempt ("7" typed over stale "abc" reads as "7bc" — issue #35).
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            // The last refusal, held until this attempt is answered (#291).
            paint_notice(notice.as_deref(), r);
            let input = Self::read_line_redrawing(
                "  Attack (numbers/all/none, enter=none)> ", &|| { draw(); });

            // Info panes (issue #120): show, then repaint this prompt.
            match input.as_str() {
                "l" => { Self::show_log(&view.display_log); r = draw(); continue; }
                "g" => { Self::show_graveyards(view); r = draw(); continue; }
                "e" => { Self::show_exile(view); r = draw(); continue; }
                "d" => { Self::show_deck_browser(view); r = draw(); continue; }
                "i" => { Self::show_battlefield_inspector(view); r = draw(); continue; }
                "s" => { Self::show_stack(view); r = draw(); continue; }
                // The list pages like the action menu (issue #260). Indices
                // stay absolute, so any number works from any page.
                "m" => {
                    list_offset.set(Self::next_menu_offset(
                        list_offset.get(), list_shown.get(), eligible.len()));
                    r = draw();
                    continue;
                }
                "p" => {
                    list_offset.set(Self::prev_menu_offset_lines(
                        &list_heights.borrow(), list_avail.get(), list_offset.get(),
                        list_marker_h.get()));
                    r = draw();
                    continue;
                }

                _ => {}
            }

            // Bare Enter means "do nothing" — here as at every other prompt
            // ([enter=pass] at the menu, enter=none at blockers). It used to
            // mean "all": the one prompt where the universal idle key took
            // the most aggressive irreversible action available, tapping the
            // whole board on a key-repeat or a stray Ctrl-D (issue #73).
            // CR 508.1a backs "none": attacking is a choice, and choosing no
            // attackers is always legal (must-attack is enforced below).
            if input.is_empty() || input == "none" || input == "n" {
                if !must_attack.is_empty() {
                    notice = Some(forced_error(&must_attack.iter().copied().collect::<Vec<_>>()));
                    continue;
                }
                return Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] };
            }
            if input == "all" || input == "a" {
                return Action::DeclareAttackers {
                    attackers: eligible.iter().map(|&id| (id, defending)).collect(),
                    planeswalker_attacks: vec![],
                };
            }

            let tokens: Vec<&str> = input.split(|c: char| c.is_whitespace() || c == ',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            // "N" attacks the player; "N>pwM" sends attacker N at
            // planeswalker M.
            let mut indices: Vec<usize> = Vec::new();
            let mut walker_attacks: Vec<(usize, usize)> = Vec::new();
            let mut parsed = 0usize;
            for t in &tokens {
                if let Some((a, w)) = t.split_once(">pw") {
                    if let (Ok(a), Ok(w)) = (a.parse::<usize>(), w.parse::<usize>()) {
                        walker_attacks.push((a, w));
                        parsed += 1;
                    }
                } else if let Ok(i) = t.parse::<usize>() {
                    indices.push(i);
                    parsed += 1;
                }
            }
            if parsed == tokens.len() {
                // A creature attacks or it doesn't (CR 508.1a): a repeated
                // index is a typo, not a double attack — refuse it loudly
                // (issue #108). The engine de-duplicates too, as the
                // authority for non-interactive players.
                let mut listed: Vec<usize> = indices.iter().copied()
                    .chain(walker_attacks.iter().map(|&(a, _)| a))
                    .collect();
                listed.sort_unstable();
                let before_dedup = listed.len();
                listed.dedup();
                if listed.len() != before_dedup {
                    notice = Some("  Duplicate attacker index: list each creature at most once.".to_string());
                    continue;
                }
                let index_error = Self::attack_index_error(
                    &indices, &walker_attacks, eligible.len(), defending_planeswalkers.len());
                if index_error.is_none() {
                    let chosen: Vec<ObjectId> = indices.iter().map(|&i| eligible[i])
                        .chain(walker_attacks.iter().map(|&(a, _)| eligible[a]))
                        .collect();
                    let missing = missing_forced(&chosen);
                    if !missing.is_empty() {
                        notice = Some(forced_error(&missing));
                        continue;
                    }
                    return Action::DeclareAttackers {
                        attackers: indices.iter().map(|&i| (eligible[i], defending)).collect(),
                        planeswalker_attacks: walker_attacks.iter()
                            .map(|&(a, w)| (eligible[a], defending_planeswalkers[w]))
                            .collect(),
                    };
                }
                if let Some(msg) = index_error {
                    notice = Some(format!("  {msg}"));
                }
            } else if defending_planeswalkers.is_empty() {
                notice = Some("  Invalid input. Enter numbers like '0 2', 'all', 'a', or 'none'.".to_string());
            } else {
                // The prompt advertises N>pwM two rows above; a player who
                // mistypes it should be shown the form, not told to enter
                // plain numbers.
                notice = Some("  Invalid input. Enter numbers like '0 2', '0>pw0', 'all', 'a', or 'none'.".to_string());
            }
        }
    }

    fn choose_blockers(view: &GameView, prompt: &CombatPrompt) -> Action {
        let CombatPrompt::ChooseBlockers { eligible_blockers, attackers: attacker_ids, legal_blocks, min_blockers } = prompt else {
            unreachable!()
        };

        if eligible_blockers.is_empty() {
            return Action::DeclareBlockers { assignments: vec![] };
        }

        // Layout once; `draw` repaints the whole prompt screen so the info
        // panes can be offered here too (issue #120).
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
        let w = term_w as usize;
        let mid_w = if w >= 100 { w.saturating_sub(2 * side + 2) } else { w.saturating_sub(side + 1) };

        // Both lists page like the action menu (issue #260): the prompt used
        // to print straight down from the cursor, so a pane that could not
        // hold them lost the tail with nothing saying so — and the block a
        // player types is only as good as the list they can see.
        let atk_offset = std::cell::Cell::new(0usize);
        let atk_shown = std::cell::Cell::new(0usize);
        let blk_offset = std::cell::Cell::new(0usize);
        let blk_shown = std::cell::Cell::new(0usize);
        let draw = || -> u16 {
            Self::render(view, Some("DECLARE BLOCKERS"), &view.display_log, "", None);
            let mut out = stdout();
            let mut r = cursor::position().unwrap_or((0, 20)).1;
            let h = terminal::size().map_or(30, |(_, h)| h as usize);
            let panel_w = Self::middle_panel_width_at(Self::term_width());
            // Rows below: the blockers header, the hint line (measured, since
            // it wraps), the prompt row and the refusal row under it. The two
            // lists split what is left.
            let hint_lines = Self::wrap_indented(BLOCK_HINTS, panel_w);
            let body = h.saturating_sub(r as usize + 3 + hint_lines.len());
            let atk_avail = (body / 2).max(1);
            let atk_marker_h = Self::marker_lines(
                attacker_ids.len().saturating_sub(1), ATTACKERS_PAGE_KEYS, panel_w);
            let blk_marker_h = Self::marker_lines(
                eligible_blockers.len().saturating_sub(1), BLOCKERS_PAGE_KEYS, panel_w);
            let _ = execute!(out, cursor::MoveTo(col, r),
                SetForegroundColor(Color::Red), SetAttribute(Attribute::Bold),
                Print(" Attackers:"), SetAttribute(Attribute::Reset), ResetColor);
            r += 1;
            // Rows are laid out whole and the page measured in their lines
            // (issue #318).
            let atk_layouts: Vec<CombatRowLayout> = attacker_ids.iter().enumerate().map(|(i, &id)| {
                // CR 509.1b: say the minimum-blockers requirement (menace,
                // Terror of Kruin Pass) up front — an unmarked menace attacker
                // took a single block the engine then discarded (issue #72).
                let note = min_blockers.get(&id)
                    .map(|min| format!(" [needs {min}+ blockers]"))
                    .unwrap_or_default();
                Self::combat_row_layout(view, id, attacker_ids, str_cols(&format!("  {i}: ")), &note, panel_w)
            }).collect();
            let atk_heights: Vec<usize> = atk_layouts.iter().map(CombatRowLayout::height).collect();
            let (atk_off, atk_n, atk_paged) =
                Self::menu_page_lines(&atk_heights, atk_avail, atk_offset.get(), atk_marker_h);
            atk_shown.set(atk_n);
            for i in atk_off..atk_off + atk_n {
                Self::draw_combat_row(&mut out, col, &mut r, i,
                    str_cols(&format!("  {i}: ")), &atk_layouts[i], Color::Reset);
            }

            if atk_paged {
                let marker = Self::page_marker(
                    atk_off, atk_n, attacker_ids.len() - 1, ATTACKERS_PAGE_KEYS);
                for line in Self::wrap_indented(&marker, panel_w) {
                    let _ = execute!(out, cursor::MoveTo(col, r),
                        SetAttribute(Attribute::Dim), Print(&line), SetAttribute(Attribute::Reset));
                    r += 1;
                }
            }
            let _ = execute!(out, cursor::MoveTo(col, r),
                SetForegroundColor(Color::Green), SetAttribute(Attribute::Bold),
                Print(" Your blockers:"), SetAttribute(Attribute::Reset), ResetColor);
            r += 1;
            let blk_avail = h.saturating_sub(r as usize + 3).max(1);
            let blk_layouts: Vec<CombatRowLayout> = eligible_blockers.iter().enumerate().map(|(i, &id)| {
                // Which attackers this creature may legally block (CR 509.1b —
                // evasion like flying is per-pairing, so say it up front).
                let legal: Vec<String> = legal_blocks.get(&id)
                    .map(|atts| attacker_ids.iter().enumerate()
                        .filter(|(_, a)| atts.contains(a))
                        .map(|(ai, _)| ai.to_string())
                        .collect())
                    .unwrap_or_default();
                let note = if legal.len() == attacker_ids.len() {
                    String::new()
                } else if legal.is_empty() {
                    " (can block: none)".to_string()
                } else {
                    format!(" (can block: {})", legal.join(" "))
                };
                Self::combat_row_layout(view, id, eligible_blockers, str_cols(&format!("  {i}: ")), &note, panel_w)
            }).collect();
            let blk_heights: Vec<usize> = blk_layouts.iter().map(CombatRowLayout::height).collect();
            let (blk_off, blk_n, blk_paged) =
                Self::menu_page_lines(&blk_heights, blk_avail, blk_offset.get(), blk_marker_h);
            blk_shown.set(blk_n);
            for i in blk_off..blk_off + blk_n {
                Self::draw_combat_row(&mut out, col, &mut r, i,
                    str_cols(&format!("  {i}: ")), &blk_layouts[i], Color::Reset);
            }

            if blk_paged {
                let marker = Self::page_marker(
                    blk_off, blk_n, eligible_blockers.len() - 1, BLOCKERS_PAGE_KEYS);
                for line in Self::wrap_indented(&marker, panel_w) {
                    let _ = execute!(out, cursor::MoveTo(col, r),
                        SetAttribute(Attribute::Dim), Print(&line), SetAttribute(Attribute::Reset));
                    r += 1;
                }
            }
            // Blocking is exactly where the public zones are decision inputs
            // (CR 404.2, 406.3) — advertise the info panes here (#120).
            for line in &hint_lines {
                let _ = execute!(out, cursor::MoveTo(col, r),
                    SetAttribute(Attribute::Dim), Print(line), SetAttribute(Attribute::Reset));
                r += 1;
            }
            let _ = execute!(out, cursor::MoveTo(col, r));
            let _ = out.flush();
            r
        };
        let mut r = draw();

        // In-pane rejection rendering, as at the attack prompt (#110).
        // A refusal is drawn under the prompt and LEFT there while the player
        // retypes: it used to be printed, slept on for 900 ms and then
        // erased, so the message was on screen exactly while the program was
        // not listening and gone by the time it was — the "silent re-render
        // is indistinguishable from a hung game" symptom #76 was filed to
        // prevent, on a timer (issue #291). Everywhere else in this CLI a
        // notice is a render input that survives until the next keystroke.
        let paint_notice = |msg: Option<&str>, r: u16| {
            let _ = execute!(stdout(), cursor::MoveTo(col, r + 1), Clear(ClearType::UntilNewLine));
            if let Some(msg) = msg {
                let _ = execute!(stdout(), SetForegroundColor(Color::Red),
                    Print(clip_cols(msg, mid_w)), ResetColor);
            }
            // Back to the prompt row: `read_line` prints its prompt at the
            // cursor, so leaving it here put the prompt on the end of the
            // notice.
            let _ = execute!(stdout(), cursor::MoveTo(col, r));
            let _ = stdout().flush();
        };
        let mut notice: Option<String> = None;

        loop {
            // Same stale-row clearing as the attack prompt (issue #35), and
            // the same held-until-answered notice row (issue #291).
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            paint_notice(notice.as_deref(), r);
            let input = Self::read_line_redrawing(
                "  Block (blocker:attacker / enter=none)> ", &|| { draw(); });

            // Info panes (issue #120): show, then repaint this prompt.
            match input.as_str() {
                "l" => { Self::show_log(&view.display_log); r = draw(); continue; }
                "g" => { Self::show_graveyards(view); r = draw(); continue; }
                "e" => { Self::show_exile(view); r = draw(); continue; }
                "d" => { Self::show_deck_browser(view); r = draw(); continue; }
                "i" => { Self::show_battlefield_inspector(view); r = draw(); continue; }
                "s" => { Self::show_stack(view); r = draw(); continue; }
                // Two lists, two pagers — indices stay absolute, so any
                // number works from any page (issue #260).
                "m" => {
                    atk_offset.set(Self::next_menu_offset(
                        atk_offset.get(), atk_shown.get(), attacker_ids.len()));
                    r = draw();
                    continue;
                }
                "b" => {
                    blk_offset.set(Self::next_menu_offset(
                        blk_offset.get(), blk_shown.get(), eligible_blockers.len()));
                    r = draw();
                    continue;
                }
                _ => {}
            }

            // Same declining vocabulary as the attack prompt: a player who
            // just learned 'none' there will type it here (issue #117).
            if input.is_empty() || input == "none" || input == "n" {
                return Action::DeclareBlockers { assignments: vec![] };
            }

            let mut assignments = Vec::new();
            let mut error: Option<String> = None;
            for pair in input.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()) {
                let (b, a) = match Self::parse_block_pair(
                    pair, eligible_blockers.len(), attacker_ids.len())
                {
                    Ok(indices) => indices,
                    Err(msg) => { error = Some(msg); break; }
                };
                let (blocker, attacker) = (eligible_blockers[b], attacker_ids[a]);
                // CR 509.1b: refuse an illegal pairing here, loudly — the
                // engine would drop it, and a silently vanished block cost
                // real games (issue #40).
                if !legal_blocks.get(&blocker).is_some_and(|atts| atts.contains(&attacker)) {
                    error = Some(format!(
                        "{} can't legally block {} (evasion or a blocking restriction).",
                        Self::perm_name(view, blocker), Self::perm_name(view, attacker)));
                    break;
                }
                // The same pair twice is one block, not two.
                if assignments.contains(&(blocker, attacker)) {
                    continue;
                }
                // CR 509.1b: one blocker, one attacker — refuse here, loudly,
                // like the illegal-pairing case above.
                if assignments.iter().any(|&(b, _)| b == blocker) {
                    error = Some(format!(
                        "{} can block only one attacker (CR 509.1b).",
                        Self::perm_name(view, blocker)));
                    break;
                }
                assignments.push((blocker, attacker));
            }

            // CR 509.1b: an attacker that can't be blocked by fewer than N
            // creatures makes any 1..N-blocker declaration illegal as a
            // whole. Refuse it here, loudly, like the per-pairing cases
            // above — the engine would discard the blocks and report
            // "declared no blockers", eating the blocker on nothing
            // (issue #72).
            if error.is_none() {
                for (&attacker, &min) in min_blockers {
                    let count = assignments.iter().filter(|&&(_, a)| a == attacker).count();
                    if count > 0 && count < min as usize {
                        error = Some(format!(
                            "{} can't be blocked by fewer than {} creatures \
                             (CR 509.1b) — add blockers or drop the block.",
                            Self::perm_name(view, attacker), min));
                        break;
                    }
                }
            }

            match error {
                None => return Action::DeclareBlockers { assignments },
                // In-pane, not println! over the LOG panel (#110's fix,
                // applied to the blocker prompt too).
                Some(msg) => notice = Some(format!("  {msg}")),
            }
        }
    }
}

impl CliPlayer {
    /// Interactive library search UI: full-screen card browser with type-to-filter,
    /// arrow key navigation, oracle text display, and Enter to select.
    /// Ask the human for an X value, then auto-distribute payment across
    /// pool mana (first) and then by source category (lands → rocks → dorks).
    /// Pool drains prefer colors the pool has the most of so the player's
    /// "scarce" colored mana is preserved when possible.
    fn prompt_x_funding(
        view: &GameView,
        options: &mtg_engine::funding::FundingOptions,
        description: &str,
        can_cancel: bool,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        use mtg_engine::funding::FundingResponse;
        use mtg_engine::types::ManaType;

        // Rendered inside the TUI frame like every other prompt: bare
        // println! landed on top of the drawn frame, colliding with the log
        // panel mid-word — "Max X = 1" read as "Max X = 13 (p0) ──" — and
        // left the stale main-phase menu on screen, so players pressed Enter
        // "to retry" and silently funded X = 0 (#56).
        Self::render(view, Some(description), &view.display_log, "", None);
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
        let w = term_w as usize;
        let mid_w = if w >= 100 { w.saturating_sub(2 * side + 2) } else { w.saturating_sub(side + 1) };
        let clip = |s: &str| -> String { s.chars().take(mid_w).collect() };
        let mut r = cursor::position().unwrap_or((0, 20)).1;
        let mut out = stdout();

        let _ = execute!(out, cursor::MoveTo(col, r),
            SetForegroundColor(Color::Yellow), SetAttribute(Attribute::Bold),
            Print(clip(&format!("  Max X = {}", options.max_announceable_x()))),
            SetAttribute(Attribute::Reset), ResetColor);
        r += 1;
        let pool_summary: Vec<String> = [
            ManaType::White, ManaType::Blue, ManaType::Black,
            ManaType::Red, ManaType::Green, ManaType::Colorless,
        ].iter().filter_map(|mt| {
            let n = options.pool.get(mt).copied().unwrap_or(0);
            if n > 0 { Some(format!("{n} {mt:?}")) } else { None }
        }).collect();
        if !pool_summary.is_empty() {
            let _ = execute!(out, cursor::MoveTo(col, r),
                Print(clip(&format!("  Pool: {}", pool_summary.join(", ")))));
            r += 1;
        }
        for g in &options.groups {
            let _ = execute!(out, cursor::MoveTo(col, r),
                Print(clip(&format!("  {} x{} ({}/tap, max {})",
                    g.name, g.source_ids.len(), g.mana_per_tap, g.max_contribution()))));
            r += 1;
        }
        let _ = execute!(out, cursor::MoveTo(col, r));
        let _ = out.flush();

        let hint = if can_cancel {
            format!("  X (0-{}, c = cancel the cast) = ", options.max_announceable_x())
        } else {
            format!("  X (0-{}) = ", options.max_announceable_x())
        };
        // The refusal goes on its own row and stays there while the player
        // retypes. It used to be written over the PROMPT row and slept on,
        // so the message and the prompt were never on screen together —
        // first the message and no prompt, then the prompt and no message
        // (issue #291).
        let mut notice: Option<String> = None;
        let x: u32 = loop {
            // Clear the input row before each attempt (same as the combat
            // prompts), so a rejected entry doesn't merge with the next.
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            let _ = execute!(stdout(), cursor::MoveTo(col, r + 1), Clear(ClearType::UntilNewLine));
            if let Some(msg) = &notice {
                let _ = execute!(stdout(), cursor::MoveTo(col, r + 1),
                    SetForegroundColor(Color::Red), Print(clip(msg)), ResetColor);
            }
            let _ = execute!(stdout(), cursor::MoveTo(col, r));
            let _ = stdout().flush();
            let input = Self::read_line(&hint);
            let input = input.trim();
            // Cancelling a spell's X prompt backs out of the whole cast —
            // the engine un-stashes it with nothing spent (issue #123).
            if can_cancel && (input == "c" || input == "cancel") {
                return Action::ResolveChoice {
                    choice: ResolvedChoice::ChosenTarget(None),
                };
            }
            // Everywhere else in this CLI the idle key is the SAFE key;
            // bare Enter used to commit the cast for X=0, burning the card
            // on a stray keypress (issue #123). It re-prompts now — X=0 is
            // still available by typing 0.
            notice = Some(match input.parse::<u32>() {
                Ok(n) if n <= options.max_announceable_x() => break n,
                _ if input.is_empty() => "  Enter a value for X.".to_string(),
                _ => format!("  Enter an integer between 0 and {}.", options.max_announceable_x()),
            });
        };

        // Distribute X: drain from pool (larger color buckets first), then
        // tap sources starting from whole-ability steps. Any mismatch at
        // the end (X isn't achievable due to multi-mana-source quanta) is
        // rounded down by dropping excess.
        let mut response = FundingResponse::default();
        // A cost reduction with no generic pips to come off pays for the
        // first `x_discount` of X, so only the rest is funded with mana
        // (CR 601.2f).
        let mut remaining = options.mana_for_x(x);

        // Pool: drain largest buckets first.
        let mut pool_sorted: Vec<(ManaType, u32)> = options.pool.iter()
            .map(|(k, v)| (*k, *v))
            .collect();
        pool_sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for (mt, avail) in pool_sorted {
            if remaining == 0 { break; }
            let take = avail.min(remaining);
            if take > 0 {
                response.pool.insert(mt, take);
                remaining -= take;
            }
        }

        // Taps: iterate groups in their given order (category-sorted). For
        // each, take as many whole activations as needed.
        for g in &options.groups {
            if remaining == 0 { break; }
            if g.mana_per_tap == 0 { continue; }
            let max_taps = u32::try_from(g.source_ids.len()).unwrap_or(u32::MAX);
            // Take as many full activations as fit within `remaining`. If
            // the quantum (mana_per_tap) doesn't divide `remaining` evenly,
            // we under-tap rather than over-tap.
            let take_taps = (remaining / g.mana_per_tap).min(max_taps);
            if take_taps > 0 {
                let amount = take_taps * g.mana_per_tap;
                response.taps.insert(g.name.clone(), amount);
                remaining -= amount;
            }
        }
        if remaining > 0 {
            let _ = execute!(stdout(), cursor::MoveTo(col, r + 1), Clear(ClearType::UntilNewLine),
                Print(clip(&format!(
                    "  (could not allocate final {remaining} mana due to source quanta; X = {})",
                    x - remaining))));
            let _ = stdout().flush();
            std::thread::sleep(std::time::Duration::from_millis(900));
        }
        Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) }
    }

    /// Choose the targets for an "up to N" slot (CR 601.2c).
    ///
    /// The same marking screen as every other "choose some of these", and
    /// `c` abandons the cast: nothing has been paid — the spell is still in
    /// its origin zone — so backing out here costs nothing (#123, #262).
    fn prompt_target_set(
        view: &GameView,
        options: &[mtg_engine::actions::Target],
        fixed: &[mtg_engine::actions::Target],
        min: usize,
        max: usize,
        description: &str,
    ) -> Action {
        use mtg_engine::actions::{ResolvedChoice, Target};
        let label = |t: &Target| match t {
            Target::Object(id) => Self::target_label(view, *id),
            Target::Player(pid) => if *pid == view.you { "You".into() } else { "Opponent".into() },
            Target::Illegal => "(illegal)".into(),
        };
        let rows: Vec<String> = options.iter().map(&label).collect();
        let pick = SetPick {
            title: Self::target_set_title(description),
            question: Self::target_set_question(
                &fixed.iter().map(&label).collect::<Vec<_>>(), min, max, options.len()),
            rows,
            min,
            max,
            cancel: Some("cancel the cast"),
        };
        match Self::pick_set(view, &pick) {
            Some(ks) => Action::ResolveChoice {
                choice: ResolvedChoice::ChosenTargetSet(ks.into_iter().map(|k| options[k].clone()).collect()),
            },
            None => Action::ResolveChoice { choice: ResolvedChoice::CancelCast },
        }
    }

    /// A target object as the pane that holds it writes it: a card in a
    /// graveyard or hand with its cost and P/T, a permanent by name.
    fn target_label(view: &GameView, id: mtg_engine::ids::ObjectId) -> String {
        let card = view.your_hand.iter()
            .chain(view.graveyards.iter().flat_map(|(_, cards)| cards.iter()))
            .find(|c| c.object_id == id);
        match card {
            Some(c) => {
                let cost = c.cost.as_ref().map(|mc| format!(" {mc}")).unwrap_or_default();
                let pt = match (c.power, c.toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                format!("{}{}{}", c.name, cost, pt)
            }
            None => Self::perm_name(view, id),
        }
    }

    /// Divide the permanents into two piles (Liliana of the Veil's -6).
    ///
    /// The same marking screen as every other "choose some of these": what
    /// is marked is pile 1, what is not is pile 2. The engine never
    /// enumerated the 2^N subsets here (issue #142) and the screen does not
    /// either.
    fn prompt_pile_division(
        view: &GameView,
        permanents: &[mtg_engine::ids::ObjectId],
        description: &str,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        let rows: Vec<String> = permanents.iter().map(|id| Self::perm_name(view, *id)).collect();
        let (title, detail) = Self::rule_title(description, 60);
        let pick = SetPick {
            title,
            question: format!("{}Mark the permanents for pile 1; the {} you leave form pile 2.",
                detail.map(|d| format!("{d} ")).unwrap_or_default(),
                if permanents.len() == 1 { "one" } else { "rest" }),
            rows,
            min: 0,
            max: permanents.len(),
            cancel: None,
        };
        let chosen = Self::pick_set(view, &pick).unwrap_or_default();
        Action::ResolveChoice {
            choice: ResolvedChoice::ChosenSubset(chosen.into_iter().map(|k| permanents[k]).collect()),
        }
    }



    /// Ask the human which graveyard cards to exile as an additional cost.
    ///
    /// The same marking screen as every other "choose some of these"
    /// question. `c` abandons the whole cast: nothing has been paid at this
    /// point — the spell is still in its origin zone with
    /// `pending_spell_cast` set — and everywhere else in this CLI the idle
    /// key is the SAFE key (#123). It used to commit on Enter, burning
    /// Harvest Pyre for X=0 or silently exiling `options[0]` for a
    /// fixed-count cost, from a card the player never chose (issue #262);
    /// here Enter is refused until the count is right.
    fn prompt_exile_from_graveyard(
        view: &GameView,
        options: &[mtg_engine::ids::ObjectId],
        min: usize,
        max: usize,
        description: &str,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        let rows: Vec<String> = options.iter().map(|id| {
            // Cost and P/T, like the graveyard panel — Corpse Lunge's damage
            // IS the exiled card's power, and the picker showed names only
            // (issue #132).
            view.graveyards.iter().flat_map(|(_, cards)| cards.iter())
                .find(|c| c.object_id == *id)
                .map(|c| {
                    let cost = c.cost.as_ref().map(|mc| format!(" {mc}")).unwrap_or_default();
                    let pt = match (c.power, c.toughness) {
                        (Some(p), Some(t)) => format!(" {p}/{t}"),
                        _ => String::new(),
                    };
                    format!("{}{}{}", c.name, cost, pt)
                })
                .unwrap_or_else(|| Self::perm_name(view, *id))
        }).collect();
        let (title, detail) = Self::rule_title(description, 60);
        let pick = SetPick {
            title,
            question: format!("{}{}",
                detail.map(|d| format!("{d} ")).unwrap_or_default(),
                Self::set_question(min, max, options.len(), "cards below to exile")),
            rows,
            min,
            max,
            cancel: Some("cancel the cast"),
        };
        match Self::pick_set(view, &pick) {
            Some(ks) => Action::ResolveChoice {
                choice: ResolvedChoice::ChosenExileSet(ks.into_iter().map(|k| options[k]).collect()),
            },
            None => Action::ResolveChoice { choice: ResolvedChoice::CancelCast },
        }
    }

    /// The one-screen ordering prompt (issue #325): every item being ordered,
    /// numbered, with everything known about it; the stack it is going onto;
    /// and one line of input — the numbers in the order chosen.
    ///
    /// The screen takes the whole terminal, so the board, the stack, the
    /// graveyards, the exile zone, the log and the deck are one key away and
    /// the prompt is redrawn when the pane closes. Enter alone keeps the
    /// order as listed. The body pages with `m`/`p` when it is taller than
    /// the terminal, and no row on it is ever clipped.
    fn prompt_ordering(view: &GameView, prompt: &OrderingPrompt) -> Action {
        let n = prompt.options.len();
        let rows = Self::ordering_rows(view, prompt);
        let mut notice: Option<String> = None;
        let mut offset = 0usize;
        loop {
            let page = Self::draw_ordering_screen(view, prompt, &rows, notice.take().as_deref(), offset);
            let redraw = || { Self::draw_ordering_screen(view, prompt, &rows, None, offset); };
            let input = Self::read_line_redrawing("  Order> ", &redraw);
            match Self::parse_order_input(&input, n) {
                OrderInput::Order(order) => {
                    return Action::ResolveChoice {
                        choice: mtg_engine::actions::ResolvedChoice::ChosenOrder(order),
                    };
                }
                OrderInput::Pane(c) => match c {
                    's' => Self::show_stack(view),
                    'i' => Self::show_battlefield_inspector(view),
                    'g' => Self::show_graveyards(view),
                    'e' => Self::show_exile(view),
                    'l' => Self::show_log(&view.display_log),
                    _ => Self::show_deck_browser(view),
                },
                OrderInput::NextPage => offset = page.next_offset(),
                OrderInput::PrevPage => offset = page.prev_offset(),
                OrderInput::Invalid(why) => notice = Some(why),
            }
        }
    }

    /// One line of input at the ordering prompt, read.
    ///
    /// The numbers, in any spacing, commas allowed, are the order; every
    /// index exactly once. An empty line keeps the listed order — the
    /// screen says so, which is what makes Enter safe here (#76, #123).
    /// The pane keys and the pagers are the same letters as everywhere else.
    fn parse_order_input(input: &str, n: usize) -> OrderInput {
        let t = input.trim();
        if t.is_empty() {
            return OrderInput::Order((0..n).collect());
        }
        match t {
            "s" | "i" | "g" | "e" | "l" | "d" => return OrderInput::Pane(t.chars().next().unwrap_or('s')),
            "m" => return OrderInput::NextPage,
            "p" => return OrderInput::PrevPage,
            _ => {}
        }
        let mut order = Vec::with_capacity(n);
        for tok in t.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()) {
            let Ok(k) = tok.parse::<usize>() else {
                return OrderInput::Invalid(format!(
                    "'{}' is not a number — type the indices in order, e.g. \"2 0 1\"", quote_input(tok)));
            };
            if k >= n {
                return OrderInput::Invalid(format!("{k} is out of range — the entries are numbered 0-{}", n.saturating_sub(1)));
            }
            if order.contains(&k) {
                return OrderInput::Invalid(format!("{k} is listed twice — each entry goes in the order exactly once"));
            }
            order.push(k);
        }
        let missing: Vec<String> = (0..n).filter(|k| !order.contains(k)).map(|k| k.to_string()).collect();
        if !missing.is_empty() {
            return OrderInput::Invalid(format!(
                "every entry needs a place: missing {}", missing.join(", ")));
        }
        OrderInput::Order(order)
    }


    /// Choose a SET of cards by marking them (issue #360).
    ///
    /// The mulligan bottoming and the cleanup discard used to be offered as
    /// one menu row per subset: "bottom 3 of 7" is 35 rows of `Bottom A, B,
    /// C`, read as a combination lock, and at 100 columns fourteen of them
    /// printed as the same five lines. What a player is doing is marking
    /// cards, so that is what the screen does — every card in hand, its
    /// mark, and a number to toggle it.
    ///
    /// The screen takes the whole terminal, so the board, the stack, the
    /// graveyards, exile, the log and the deck are one key away and the
    /// prompt is redrawn when the pane closes. Enter confirms, and only
    /// when the count is right: there is no way to answer this by accident,
    /// which at a mandatory irreversible choice is the point (#123, #262).
    fn prompt_card_set(view: &GameView, prompt: &mtg_engine::actions::SetPrompt, title: &str) -> Action {
        let rows: Vec<String> = prompt.options.iter()
            .map(|id| Self::hand_card_label(view, *id)).collect();
        let pick = SetPick {
            title: title.trim().to_string(),
            question: Self::set_question(prompt.min, prompt.max, prompt.options.len(), "below"),
            rows,
            min: prompt.min,
            max: prompt.max,
            cancel: None,
        };
        match Self::pick_set(view, &pick) {
            Some(ks) => prompt.answer(ks.into_iter().map(|k| prompt.options[k]).collect()),
            // `cancel: None` means the loop never returns one.
            None => unreachable!("a card-set prompt with no cancel was cancelled"),
        }
    }

    /// The line that says how many to mark.
    fn set_question(min: usize, max: usize, n: usize, what: &str) -> String {
        if min == max {
            format!("Mark {min} of the {n} {what}.")
        } else if min == 0 {
            format!("Mark up to {max} of the {n} {what}.")
        } else {
            format!("Mark between {min} and {max} of the {n} {what}.")
        }
    }

    /// The heading at a target-set prompt.
    ///
    /// Every other marking screen is headed by a short upper-case line
    /// naming the action — "DISCARD 1 CARD", "BOTTOM 2 CARDS AFTER
    /// MULLIGAN" — and this one is headed the same way. The engine's
    /// description is "<card>: choose up to N targets", whose second half
    /// the question line below says better, against the real row count.
    fn target_set_title(description: &str) -> String {
        let name = description.split_once(": ").map_or(description, |(head, _)| head);
        format!("CHOOSE TARGETS FOR {}", name.trim().to_uppercase())
    }

    /// The question at a target-set prompt.
    ///
    /// A slot with something already named in front of it is the second half
    /// of a question — Memory's Journey names a player and then asks for
    /// cards from THEIR graveyard, and which player that is decides what the
    /// rows even are. Say it, or the list has no context.
    ///
    /// The engine's own description says "choose up to N" too, and is not
    /// repeated here: [`set_question`](Self::set_question) says the same
    /// thing against the actual row count.
    fn target_set_question(already: &[String], min: usize, max: usize, n: usize) -> String {
        let prefix = if already.is_empty() {
            String::new()
        } else {
            format!("Targeting {}. ", already.join(", "))
        };
        format!("{prefix}{}", Self::set_question(min, max, n, "targets below"))
    }

    /// A card in hand as the hand panel writes it.
    fn hand_card_label(view: &GameView, id: mtg_engine::ids::ObjectId) -> String {
        view.your_hand.iter().find(|c| c.object_id == id)
            .map(|c| {
                let cost = c.cost.as_ref().map(|mc| format!(" {mc}")).unwrap_or_default();
                let pt = match (c.power, c.toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                format!("{}{}{}", c.name, cost, pt)
            })
            .unwrap_or_else(|| Self::perm_name(view, id))
    }

    /// The one screen for "choose some of these": mark what you want.
    ///
    /// Every such question in the game comes here — the mulligan bottoming,
    /// the cleanup discard, an exile cost, a pile division, a spell's "up to
    /// N targets" — because they are all the same interaction and none of
    /// them is a menu. Returns the indices marked, or `None` when the player
    /// abandoned a choice that may be abandoned.
    fn pick_set(view: &GameView, pick: &SetPick) -> Option<Vec<usize>> {
        let mut marked: Vec<bool> = vec![false; pick.rows.len()];
        // Whether the player has touched the selection at all. Where an
        // empty answer is legal — Harvest Pyre exiling nothing, X=0 — the
        // idle key would otherwise COMMIT it, which is issue #262: the safe
        // key must not be an answer. Marking nothing on purpose is `n`.
        let mut touched = false;
        let mut notice: Option<String> = None;
        let mut offset = 0usize;
        loop {
            let page = Self::draw_set_screen(pick, &marked, notice.take().as_deref(), offset);
            let redraw = || { Self::draw_set_screen(pick, &marked, None, offset); };
            let input = Self::read_line_redrawing("  Mark> ", &redraw);
            match Self::parse_card_set_input(&input, pick.rows.len(), pick.cancel.is_some()) {
                SetInput::Toggle(ks) => {
                    touched = true;
                    for k in ks {
                        marked[k] = !marked[k];
                    }
                }
                SetInput::All => { touched = true; marked.iter_mut().for_each(|m| *m = true); }
                SetInput::None => { touched = true; marked.iter_mut().for_each(|m| *m = false); }
                SetInput::Cancel => return None,
                SetInput::Confirm => {
                    let chosen: Vec<usize> = marked.iter().enumerate()
                        .filter(|(_, m)| **m).map(|(i, _)| i).collect();
                    if chosen.len() < pick.min || chosen.len() > pick.max {
                        notice = Some(Self::set_count_error(chosen.len(), pick.min, pick.max));
                        continue;
                    }
                    if chosen.is_empty() && !touched {
                        notice = Some(SET_NOTHING_MARKED.to_string());
                        continue;
                    }
                    return Some(chosen);
                }
                SetInput::Pane(c) => match c {
                    's' => Self::show_stack(view),
                    'i' => Self::show_battlefield_inspector(view),
                    'g' => Self::show_graveyards(view),
                    'e' => Self::show_exile(view),
                    'l' => Self::show_log(&view.display_log),
                    _ => Self::show_deck_browser(view),
                },
                SetInput::NextPage => offset = page.next_offset(),
                SetInput::PrevPage => offset = page.prev_offset(),
                SetInput::Invalid(why) => notice = Some(why),
            }
        }
    }

    /// Why a confirmed selection was refused, in the terms the screen asks
    /// in: an exact count, a ceiling, or a range.
    fn set_count_error(have: usize, min: usize, max: usize) -> String {
        let card = |n: usize| if n == 1 { "card" } else { "cards" };
        if min == max {
            format!("{have} marked — mark exactly {min} {}", card(min))
        } else if min == 0 {
            format!("{have} marked — mark at most {max} {}", card(max))
        } else {
            format!("{have} marked — mark between {min} and {max} cards")
        }
    }

    /// Draw the whole set-picking screen and return the page that was drawn.
    fn draw_set_screen(pick: &SetPick, marked: &[bool], notice: Option<&str>,
                       offset: usize) -> BodyPage {
        let (term_w, term_h) = terminal::size().unwrap_or((100, 30));
        let w = term_w as usize;
        let h = term_h as usize;
        let text_w = w.saturating_sub(2);
        let chosen = marked.iter().filter(|m| **m).count();

        let mut header: Vec<(Style, String)> = Vec::new();
        header.push((Style::Title, format!(" {}", pick.title)));
        for l in Self::word_wrap(&pick.question, text_w) { header.push((Style::Plain, format!(" {l}"))); }
        let of = if pick.min == pick.max { pick.min } else { pick.max };
        header.push((Style::Bold, format!(" {chosen} of {of} marked")));
        header.push((Style::Plain, String::new()));

        let body: Vec<(Style, String)> = pick.rows.iter().enumerate()
            .flat_map(|(i, r)| {
                let mark = if marked.get(i).copied().unwrap_or(false) { "[x]" } else { "[ ]" };
                Self::wrap_indented(&format!(" {mark} {i}: {r}"), text_w)
                    .into_iter().map(|l| (Style::Row, l)).collect::<Vec<_>>()
            })
            .collect();

        let mut footer: Vec<(Style, String)> = Vec::new();
        let how_to = match pick.cancel {
            Some(label) => format!("{SET_HOW_TO} [c={label}]"),
            None => SET_HOW_TO.to_string(),
        };
        for line in Self::wrap_indented(&how_to, text_w) { footer.push((Style::Dim, line)); }
        // The notice row and the input row are always reserved.
        let reserved = header.len() + footer.len() + 2;
        let avail = h.saturating_sub(reserved).max(1);
        let page = BodyPage::new(body.len(), avail, offset);

        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
        let mut row: u16 = 0;
        let put = |out: &mut io::Stdout, row: &mut u16, style: Style, text: &str| {
            let _ = execute!(out, cursor::MoveTo(0, *row));
            match style {
                Style::Title => Self::print_colored(out, Color::Cyan, text),
                Style::Bold => { let _ = execute!(out, SetAttribute(Attribute::Bold), Print(text), SetAttribute(Attribute::Reset)); }
                Style::Dim => { let _ = execute!(out, SetAttribute(Attribute::Dim), Print(text), SetAttribute(Attribute::Reset)); }
                Style::Row => {
                    // The mark and the index in bold, the rest through the
                    // mana colourer.
                    let split = text.find(": ").map_or(text.len(), |p| p + 2);
                    let _ = execute!(out, SetAttribute(Attribute::Bold), Print(&text[..split]), SetAttribute(Attribute::Reset));
                    Self::print_with_mana(out, &text[split..], None);
                }
                Style::Plain => Self::print_with_mana(out, text, None),
            }
            *row += 1;
        };
        for (s, l) in &header { put(&mut out, &mut row, *s, l); }
        for (s, l) in &body[page.start..page.end] { put(&mut out, &mut row, *s, l); }
        if page.paged {
            put(&mut out, &mut row, Style::Dim, &format!(
                " … showing lines {}-{} of {} — m/p = next/prev page", page.start + 1, page.end, body.len()));
        }
        for (s, l) in &footer { put(&mut out, &mut row, *s, l); }
        if let Some(msg) = notice {
            let _ = execute!(out, cursor::MoveTo(0, row), SetForegroundColor(Color::Red),
                Print(clip_cols(&format!("  {msg}"), w)), ResetColor);
        }
        row += 1;
        let _ = execute!(out, cursor::MoveTo(0, row));
        let _ = out.flush();
        page
    }

    /// One line of input at a card-set prompt, read.
    ///
    /// Numbers toggle; `a`/`n` mark all or none; an empty line confirms,
    /// and is refused unless the count is right. The pane keys and the
    /// pagers are the same letters as everywhere else.
    fn parse_card_set_input(input: &str, n: usize, can_cancel: bool) -> SetInput {
        let t = input.trim();
        if t.is_empty() {
            return SetInput::Confirm;
        }
        if can_cancel && matches!(t, "c" | "cancel") {
            return SetInput::Cancel;
        }
        match t {
            "s" | "i" | "g" | "e" | "l" | "d" => return SetInput::Pane(t.chars().next().unwrap_or('s')),
            "m" => return SetInput::NextPage,
            "p" => return SetInput::PrevPage,
            "a" | "all" => return SetInput::All,
            "n" | "none" => return SetInput::None,
            _ => {}
        }
        let mut ks = Vec::new();
        for tok in t.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()) {
            let Ok(k) = tok.parse::<usize>() else {
                return SetInput::Invalid(format!(
                    "'{}' is not a number — type the number of a card to mark or unmark it", quote_input(tok)));
            };
            if k >= n {
                return SetInput::Invalid(format!("{k} is out of range — the cards are numbered 0-{}", n.saturating_sub(1)));
            }
            ks.push(k);
        }
        SetInput::Toggle(ks)
    }

    /// The rows of the ordering screen, one per option, as `(index, lines)`
    /// — the lines unwrapped; the screen wraps them to its width. With the
    /// engine's per-trigger details a row says whose ability it is, its
    /// P/T, what it does and what set it off; without them (a prompt from
    /// an older save, or a blocker list) it is the option's text.
    fn ordering_rows(view: &GameView, prompt: &OrderingPrompt) -> Vec<Vec<String>> {
        let _ = view;
        prompt.options.iter().enumerate().map(|(k, option)| {
            match prompt.details.get(k) {
                Some(d) => {
                    let pt = d.power_toughness.map(|(p, t)| format!(" {p}/{t}")).unwrap_or_default();
                    let what = if d.ability.is_empty() {
                        d.kind.clone()
                    } else {
                        format!("{}: {}", d.kind, d.ability)
                    };
                    vec![
                        format!("{} (#{}){pt} — {what}", d.source_name, d.source.0),
                        format!("triggered by: {}", d.cause),
                    ]
                }
                None => vec![option.clone()],
            }
        }).collect()
    }

    /// The oracle text of every distinct source among the triggers being
    /// ordered, from the registry — "all their info" includes what the card
    /// says, and a source that has already died is on no pane.
    fn ordering_sources(prompt: &OrderingPrompt) -> Vec<(String, Vec<String>)> {
        let registry = mtg_engine::cards::CardRegistry::with_all_cards();
        let mut seen: Vec<String> = Vec::new();
        let mut out = Vec::new();
        for d in prompt.details {
            if seen.contains(&d.source_name) { continue; }
            seen.push(d.source_name.clone());
            let Some(data) = registry.get_id_by_name(&d.source_name).and_then(|id| registry.card_data(id)) else { continue };
            let cost = data.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
            let pt = match (data.power, data.toughness) {
                (Some(p), Some(t)) => format!(" {p}/{t}"),
                _ => String::new(),
            };
            let head = format!("{}{cost} — {}{pt}", data.name,
                mtg_engine::types::type_line(&data.supertypes, &data.card_types, &data.subtypes));
            let text: Vec<String> = data.oracle_text.lines().map(str::to_string).collect();
            out.push((head, text));
        }
        out
    }

    /// Paint the ordering screen and return the page of body lines it drew.
    fn draw_ordering_screen(view: &GameView, prompt: &OrderingPrompt, rows: &[Vec<String>],
                            notice: Option<&str>, offset: usize) -> BodyPage {
        let mut out = stdout();
        let (term_w, term_h) = terminal::size().unwrap_or((100, 30));
        let (w, h) = (term_w as usize, term_h as usize);
        let text_w = w.saturating_sub(2).max(20);
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

        // Header: what is being decided.
        let (title, rule) = match prompt.kind {
            OrderingKind::Triggers => (" ORDER YOUR TRIGGERS",
                "The first you list goes on the stack FIRST and resolves LAST; the last you list resolves FIRST (CR 603.3b)."),
            OrderingKind::Blockers => (" DAMAGE ASSIGNMENT ORDER",
                "The first you list is assigned damage FIRST, and must be assigned lethal damage before the next gets any (CR 510.1c)."),
        };
        let mut header: Vec<(Style, String)> = vec![(Style::Title, title.to_string())];
        for line in Self::word_wrap(prompt.description, text_w) {
            header.push((Style::Dim, format!(" {line}")));
        }
        header.push((Style::Plain, String::new()));

        // Body: the stack as it stands, the rows, and the sources.
        let idx_w = rows.len().saturating_sub(1).to_string().len();
        let indent = idx_w + 4;
        let mut body: Vec<(Style, String)> = Vec::new();
        if prompt.kind == OrderingKind::Triggers {
            body.push((Style::Bold, " Stack now (top first):".into()));
            if view.stack.is_empty() {
                body.push((Style::Dim, "   (empty)".into()));
            } else {
                for (i, item) in view.stack.iter().enumerate().take(6) {
                    let line = format!("   {i}: {}", Self::stack_entry_headline(view, item));
                    for l in Self::wrap_row(&line, text_w) { body.push((Style::Dim, l)); }
                }
                if view.stack.len() > 6 {
                    body.push((Style::Dim, format!("   … and {} more (s = the whole stack)", view.stack.len() - 6)));
                }
            }
            body.push((Style::Plain, String::new()));
            body.push((Style::Bold, " Triggers to order:".into()));
        } else {
            body.push((Style::Bold, " Blockers to order:".into()));
        }
        for (k, lines) in rows.iter().enumerate() {
            for (j, line) in lines.iter().enumerate() {
                let wrapped = Self::wrap_row(line, text_w.saturating_sub(indent).max(10));
                for (m, piece) in wrapped.into_iter().enumerate() {
                    if j == 0 && m == 0 {
                        body.push((Style::Row, format!("  {k:>idx_w$}: {piece}")));
                    } else {
                        let style = if j == 0 { Style::Plain } else { Style::Dim };
                        body.push((style, format!("{}{piece}", " ".repeat(indent))));
                    }
                }
            }
        }
        let sources = Self::ordering_sources(prompt);
        if !sources.is_empty() {
            body.push((Style::Plain, String::new()));
            body.push((Style::Bold, " Sources:".into()));
            // Oracle text is prose, so it breaks at spaces. `wrap_row`'s
            // comma-first rule is for the rows that are lists — it turned
            // "At the beginning of your upkeep, look at ..." into a stub
            // line with two words on it.
            for (head, text) in &sources {
                for l in Self::word_wrap(head, text_w.saturating_sub(2)) { body.push((Style::Plain, format!("  {l}"))); }
                for line in text {
                    for l in Self::word_wrap(line, text_w.saturating_sub(4)) { body.push((Style::Dim, format!("    {l}"))); }
                }
            }
        }

        // Footer: how to answer, the pane keys, any refusal, the input row.
        // Both footer rows are sentences, not lists: they break at spaces
        // and every continuation lines up under the first line's indent
        // rather than starting at column 0.
        let mut footer: Vec<(Style, String)> = Vec::new();
        for line in Self::wrap_indented(&format!(" {rule}"), text_w) { footer.push((Style::Dim, line)); }
        for line in Self::wrap_indented(ORDER_HOW_TO, text_w) { footer.push((Style::Dim, line)); }
        // The notice row and the input row are always reserved.
        let reserved = header.len() + footer.len() + 2;
        let avail = h.saturating_sub(reserved).max(1);
        let page = BodyPage::new(body.len(), avail, offset);

        let mut row: u16 = 0;
        let put = |out: &mut io::Stdout, row: &mut u16, style: Style, text: &str| {
            let _ = execute!(out, cursor::MoveTo(0, *row));
            match style {
                Style::Title => Self::print_colored(out, Color::Cyan, text),
                Style::Bold => { let _ = execute!(out, SetAttribute(Attribute::Bold), Print(text), SetAttribute(Attribute::Reset)); }
                Style::Dim => { let _ = execute!(out, SetAttribute(Attribute::Dim), Print(text), SetAttribute(Attribute::Reset)); }
                Style::Row => {
                    // "  N: " in bold, the rest through the mana colourer.
                    let split = text.find(": ").map_or(text.len(), |p| p + 2);
                    let _ = execute!(out, SetAttribute(Attribute::Bold), Print(&text[..split]), SetAttribute(Attribute::Reset));
                    Self::print_with_mana(out, &text[split..], None);
                }
                Style::Plain => Self::print_with_mana(out, text, None),
            }
            *row += 1;
        };
        for (s, l) in &header { put(&mut out, &mut row, *s, l); }
        for (s, l) in &body[page.start..page.end] { put(&mut out, &mut row, *s, l); }
        if page.paged {
            put(&mut out, &mut row, Style::Dim, &format!(
                " … showing lines {}-{} of {} — m/p = next/prev page", page.start + 1, page.end, body.len()));
        }
        for (s, l) in &footer { put(&mut out, &mut row, *s, l); }
        if let Some(msg) = notice {
            let _ = execute!(out, cursor::MoveTo(0, row), SetForegroundColor(Color::Red),
                Print(clip_cols(&format!("  {msg}"), w)), ResetColor);
        }
        row += 1;
        let _ = execute!(out, cursor::MoveTo(0, row));
        let _ = out.flush();
        page
    }

    fn library_search_ui(view: &GameView, actions: &[Action], title: &str, decline: Option<Action>) -> Action {

        use mtg_engine::actions::ResolvedChoice;

        // Collect card info for each option.
        struct CardInfo {
            name: String,
            type_line: String,
            oracle_text: String,
            cost: String,
            pt: String,
            action_index: usize,
        }
        let mut cards: Vec<CardInfo> = Vec::new();
        // Naming a card (Nevermore) offers every non-land name in the
        // registry — 253 of them — as `ChosenIndex`, which this browser used
        // to ignore, so the one prompt in the game with hundreds of
        // homogeneous options was the one prompt that fell through to a flat
        // forward-only paged list. Finding Geistflame in it meant paging past
        // it and then eleven more presses to wrap around (issue #255).
        let by_name = mtg_engine::cards::CardRegistry::with_all_cards();
        for (i, action) in actions.iter().enumerate() {
            if let Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(_, name) } = action {
                let (type_line, oracle_text, cost, pt) = by_name.get_id_by_name(name)
                    .and_then(|cid| by_name.card_data(cid))
                    .map(|d| {
                        let types: Vec<&str> = d.card_types.iter().map(|t| match t {
                            CardType::Creature => "Creature",
                            CardType::Instant => "Instant",
                            CardType::Sorcery => "Sorcery",
                            CardType::Enchantment => "Enchantment",
                            CardType::Artifact => "Artifact",
                            CardType::Land => "Land",
                            CardType::Planeswalker => "Planeswalker",
                        }).collect();
                        let pt_str = match (d.power, d.toughness) {
                            (Some(pw), Some(t)) => format!("{pw}/{t}"),
                            _ => String::new(),
                        };
                        (types.join(" "), d.oracle_text.clone(),
                         d.cost.as_ref().map(|mc| format!("{mc}")).unwrap_or_default(), pt_str)
                    })
                    .unwrap_or_default();
                cards.push(CardInfo {
                    name: name.clone(), type_line, oracle_text, cost, pt, action_index: i,
                });
                continue;
            }
            if let Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(id) } = action {
                let name = Self::perm_name(view, *id);
                // Look up card info from library cards or hand.
                let (type_line, oracle_text, cost, pt) = view.your_library_cards.iter()
                    .find(|c| c.object_id == *id)
                    .or_else(|| view.your_hand.iter().find(|c| c.object_id == *id))
                    .map(|c| {
                        let types: Vec<&str> = c.card_types.iter().map(|t| match t {
                            CardType::Creature => "Creature",
                            CardType::Instant => "Instant",
                            CardType::Sorcery => "Sorcery",
                            CardType::Enchantment => "Enchantment",
                            CardType::Artifact => "Artifact",
                            CardType::Land => "Land",
                            CardType::Planeswalker => "Planeswalker",
                        }).collect();
                        let type_str = types.join(" ");
                        let cost_str = c.cost.as_ref().map(|mc| format!("{mc}")).unwrap_or_default();
                        let pt_str = match (c.power, c.toughness) {
                            (Some(p), Some(t)) => format!("{p}/{t}"),
                            _ => String::new(),
                        };
                        (type_str, c.oracle_text.clone(), cost_str, pt_str)
                    })
                    .unwrap_or_default();
                cards.push(CardInfo { name, type_line, oracle_text, cost, pt, action_index: i });
            }
        }

        let mut filter = String::new();
        let mut selected: usize = 0;
        let mut out = stdout();

        tui_raw_on();
        // A paste in the filter box must arrive as one Paste event, never as
        // keystrokes whose embedded newline SELECTS a card and whose next
        // line answers the following prompt (issue #106; same hardening as
        // read_line, #50).
        let _ = execute!(out, event::EnableBracketedPaste);

        loop {
            // Filter cards by name.
            let filtered: Vec<&CardInfo> = if filter.is_empty() {
                cards.iter().collect()
            } else {
                let lower = filter.to_lowercase();
                cards.iter().filter(|c| c.name.to_lowercase().contains(&lower)).collect()
            };

            // Clamp selection.
            if selected >= filtered.len() && !filtered.is_empty() {
                selected = filtered.len() - 1;
            }

            // Render.
            let _ = execute!(out, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0));
            // The header names the effect and the choice ("Forbidden
            // Alchemy: choose a card to put into your hand") — the literal
            // "Search Library" implied free tutoring for prompts that
            // reveal a fixed set, or discard from hand (issue #95).
            let _ = execute!(out, SetForegroundColor(Color::Yellow),
                Print(format!("═══ {title} ═══\n\r")),
                ResetColor);
            // Clipped by display columns; an unclipped 10k-char filter was
            // 50 rows of echo re-painted per keystroke and looked like a
            // hang (#109). The tail is shown — it's what was last typed.
            let (term_w_now, _) = terminal::size().unwrap_or((80, 24));
            let avail = (term_w_now as usize).saturating_sub("Filter: _".len() + 1);
            let shown = if str_cols(&filter) > avail {
                let tail: String = filter.chars().rev().collect::<String>();
                let mut kept = clip_cols(&tail, avail.saturating_sub(1));
                kept = kept.chars().rev().collect();
                format!("\u{2026}{kept}")
            } else {
                filter.clone()
            };
            let _ = execute!(out, Print(format!("Filter: {shown}_\n\r\n\r")));

            let (_, term_height) = terminal::size().unwrap_or((80, 24));
            let max_list = (term_height as usize).saturating_sub(8); // Leave room for detail

            let start = selected.saturating_sub(max_list / 2);
            let visible: Vec<_> = filtered.iter().enumerate().skip(start).take(max_list).collect();

            // An empty result on a mandatory prompt must say so — a blank
            // list with "Enter to select" read as a hung game (issue #124).
            if filtered.is_empty() && !filter.is_empty() {
                let _ = execute!(out, SetForegroundColor(Color::Yellow),
                    Print(format!("  No cards match '{shown}' — press Esc or Backspace to clear the filter.\n\r")),
                    ResetColor);
            }

            for (i, card) in &visible {
                if *i == selected {
                    let _ = execute!(out, SetForegroundColor(Color::Black),
                        SetAttribute(Attribute::Reverse),
                        Print(format!(" > {} ", card.name)),
                        SetAttribute(Attribute::Reset),
                        ResetColor,
                        Print("\n\r"));
                } else {
                    let _ = execute!(out, Print(format!("   {}\n\r", card.name)));
                }
            }

            // Show detail for selected card.
            if let Some(card) = filtered.get(selected) {
                let _ = execute!(out, Print("\n\r"));
                let _ = execute!(out, Print("  "));
                Self::print_with_mana(&mut out, &format!("{} {}", card.name, card.cost), Some(Color::Cyan));
                let _ = execute!(out, Print("\n\r"), ResetColor);
                let _ = execute!(out, SetForegroundColor(Color::DarkGrey),
                    Print(format!("  {}", card.type_line)),
                    ResetColor);
                if !card.pt.is_empty() {
                    let _ = execute!(out, Print(format!("  {}", card.pt)));
                }
                let _ = execute!(out, Print("\n\r"));
                // Oracle text — wrap lines.
                for line in card.oracle_text.split('\n') {
                    let _ = execute!(out, Print(format!("  {line}\n\r")));
                }
            }

            let _ = execute!(out, Print("\n\r"),
                SetForegroundColor(Color::DarkGrey),
                Print(match (decline.is_some(), filtered.is_empty() && !filter.is_empty()) {
                    // The footer names the way out — Esc/Backspace were
                    // undiscoverable exactly when the list was empty (#124).
                    (_, true) => "no matches  |  Esc or Backspace clears the filter",
                    (true, false) =>
                        "↑↓ navigate  |  type to filter  |  Enter to select  |  Esc (empty filter) = take nothing",
                    (false, false) =>
                        "↑↓ navigate  |  type to filter  |  Enter to select  |  Esc = clear filter",
                }),
                ResetColor, Print("\n\r"));
            let _ = out.flush();

            // Read input.
            let Some(ev) = read_event_guarded() else { continue };
            // Only the paste's first line reaches the filter; its newlines
            // never act as Enter (issue #106).
            if let Event::Paste(pasted) = &ev {
                let first = pasted.split(['\r', '\n']).next().unwrap_or("");
                filter.push_str(first);
                selected = 0;
                continue;
            }
            if let Event::Key(KeyEvent { code, modifiers, .. }) = ev {
                match code {
                    KeyCode::Enter => {
                        if let Some(card) = filtered.get(selected) {
                            let _ = execute!(out, event::DisableBracketedPaste);
                            tui_raw_off();
                            let _ = execute!(out, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0));
                            return actions[card.action_index].clone();
                        }
                    }
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if selected + 1 < filtered.len() { selected += 1; }
                    }
                    KeyCode::Backspace => {
                        filter.pop();
                        selected = 0;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        let _ = execute!(out, event::DisableBracketedPaste);
                        quit_at_prompt();
                    }
                    KeyCode::Char(c) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        filter.push(c);
                        selected = 0;
                    }
                    KeyCode::Esc => {
                        // With an empty filter, Esc declines an optional
                        // search ("you may search...", CR 701.19b) — the
                        // decline entry that used to force this prompt into
                        // a flat menu (issue #111). With text in the filter
                        // it clears the filter first, as before.
                        if filter.is_empty() {
                            if let Some(decline_action) = decline {
                                let _ = execute!(out, event::DisableBracketedPaste);
                                tui_raw_off();
                                let _ = execute!(out, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0));
                                return decline_action;
                            }
                        }
                        filter.clear();
                        selected = 0;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Every object an action names, in a fixed order — what makes one
    /// offered action a different offer from another.
    ///
    /// The disambiguating `(#id)` used to be added by a block that matched
    /// ONE action variant, `ResolveChoice::ChosenTarget(Some(Object))`, so
    /// every other kind of row that can collide fell out of it silently: two
    /// Wooden Stakes offering the same ability, two identical duals offering
    /// the same mana, two Islands to play (issue #257).
    fn action_object_ids(action: &Action) -> Vec<u64> {
        use mtg_engine::actions::ResolvedChoice;
        fn of_targets(targets: &[Target], ids: &mut Vec<u64>) {
            ids.extend(targets.iter().filter_map(|t| match t {
                Target::Object(id) => Some(id.0),
                _ => None,
            }));
        }
        let mut ids = Vec::new();
        match action {
            Action::PlayLand { object_id } => ids.push(object_id.0),
            // Two untapped Islands making {U} are the same offer: which one
            // taps changes nothing a player can act on, and numbering every
            // land in a six-land board would be noise, not information.
            Action::ActivateManaAbility { .. } => {}
            Action::CastSpell { object_id, targets, sacrifice, .. } => {
                ids.push(object_id.0);
                of_targets(targets, &mut ids);
                if let Some(sac) = sacrifice { ids.push(sac.0); }
            }
            Action::ActivateAbility { object_id, targets, sacrifice, .. } => {
                ids.push(object_id.0);
                of_targets(targets, &mut ids);
                if let Some(sac) = sacrifice { ids.push(sac.0); }
            }
            Action::ActivateLoyaltyAbility { object_id, targets, .. } => {
                ids.push(object_id.0);
                of_targets(targets, &mut ids);
            }
            Action::DiscardCards { cards } | Action::BottomCards { cards } =>
                ids.extend(cards.iter().map(|c| c.0)),
            Action::DeclareBlockers { assignments } =>
                ids.extend(assignments.iter().flat_map(|(b, a)| [b.0, a.0])),
            Action::DeclareAttackers { attackers, planeswalker_attacks } => {
                ids.extend(attackers.iter().map(|(a, _)| a.0));
                ids.extend(planeswalker_attacks.iter().flat_map(|(a, pw)| [a.0, pw.0]));
            }
            Action::ResolveChoice { choice } => match choice {
                ResolvedChoice::ChosenTarget(Some(t)) => of_targets(std::slice::from_ref(t), &mut ids),
                ResolvedChoice::ChosenCard(id) => ids.push(id.0),
                ResolvedChoice::ChosenSubset(objs) | ResolvedChoice::ChosenExileSet(objs) =>
                    ids.extend(objs.iter().map(|o| o.0)),
                // Nothing this row names is an object: an index, a yes/no, a
                // funding plan. Two such rows that read alike ARE the same
                // offer, and adding a number to them would say nothing.
                _ => {}
            },
            Action::PassPriority | Action::Concede | Action::AbandonGame
            | Action::MulliganKeep | Action::MulliganMull => {}
        }
        ids
    }

    /// The menu row for an action with no arm of its own, carrying the
    /// objects it names so two rows that read alike can still be told apart.
    fn menu_label_for(view: &GameView, action: &Action) -> MenuLabel {
        MenuLabel {
            text: Self::format_action(view, action),
            ids: Self::action_object_ids(action),
        }
    }


    /// Build the priority menu: the rows, and what each row stands for.
    ///
    /// Pure — no terminal, no input — so "every row names a different action"
    /// is a testable contract. It was inline in `choose_action` above a
    /// blocking read, which is how a menu that renders two different equips
    /// as one line kept shipping (issues #257, #258).
    fn build_action_menu(view: &GameView, legal: &mtg_engine::engine::LegalActions)
        -> (Vec<DisplayEntry>, Vec<MenuLabel>)
    {
        let legal_actions = &legal.actions;
        // Build a collapsed display list: non-CastSpell actions + one entry per castable spell.
        // Each entry maps to either a direct action or an interactive casting flow.
        let mut display: Vec<DisplayEntry> = Vec::new();
        let mut display_labels: Vec<MenuLabel> = Vec::new();
        // Keyed by (object, casting with an alternative cost): a spell that
        // can be cast both normally and via Rooftop Storm's "without paying
        // its mana cost" is TWO menu rows — collapsing on the object alone
        // dropped the CR 601.2b choice (issue #128).
        let mut seen_spell_objects: Vec<(mtg_engine::ids::ObjectId, bool)> = Vec::new();

        // Ordering: non-tap actions, cast spells, tap actions, concede last.
        let mut deferred_taps: Vec<(usize, MenuLabel)> = Vec::new();
        let mut deferred_concede: Option<(usize, MenuLabel)> = None;
        let mut seen_cast_labels: Vec<String> = Vec::new();
        for (i, action) in legal_actions.iter().enumerate() {
            match action {
                Action::CastSpell { object_id, alternative_cost, .. } => {
                    // Skip expanded CastSpell entries — use castable_spells instead.
                    let key = (*object_id, alternative_cost.is_some());
                    if !seen_spell_objects.contains(&key) {
                        // Find the CastableSpell entry for this way to cast.
                        if let Some(cs_idx) = legal.castable_spells.iter()
                            .position(|cs| cs.object_id == *object_id
                                && cs.alternative_cost.is_some() == alternative_cost.is_some())
                        {
                            seen_spell_objects.push(key);
                            let cs = &legal.castable_spells[cs_idx];
                            let label = Self::cast_row_label(view, cs);
                            // Deduplicate identical cast labels (e.g. two copies of same spell).
                            let full = label.full();
                            if seen_cast_labels.contains(&full) { continue; }
                            seen_cast_labels.push(full);
                            display.push(DisplayEntry::Cast(cs_idx));
                            display_labels.push(label);
                        }
                    }
                }
                Action::ActivateManaAbility { .. } => {
                    // Defer tap actions to appear after cast spells.
                    deferred_taps.push((i, Self::menu_label_for(view, action)));
                }
                Action::Concede => {
                    // Defer concede to always be last.
                    deferred_concede = Some((i, MenuLabel::plain(Self::format_action(view, action))));
                }
                // The ability's own text goes in the label: without it two
                // different abilities on one permanent rendered identically
                // and the player could not tell a 2-mana ability from a
                // 5-mana one (#61). The engine already collapses the metadata
                // into activatable_abilities, description included.
                Action::ActivateAbility { object_id, ability_index, source_card_id, targets, sacrifice, .. } => {
                    let desc = legal.activatable_abilities.iter()
                        .find(|ab| ab.object_id == *object_id
                            && ab.ability_index == *ability_index
                            && ab.source_card_id == *source_card_id)
                        .map(|ab| ab.description.clone())
                        .filter(|d| !d.is_empty());
                    // A sacrifice cost with a choice in it (CR 601.2h) is
                    // part of what this entry does: Grimgrin's two
                    // "Sacrifice another creature" entries differed only in
                    // which creature died, with nothing on screen saying so
                    // (issue #80). Sacrificing THIS permanent is already in
                    // the description, so only name a different one.
                    let sac_suffix = match sacrifice {
                        Some(sac) if sac != object_id =>
                            Self::sacrifice_suffix(view, Some(*sac)),
                        // A choose-a-creature cost picking the source itself:
                        // this entry rendered with no creature named at all,
                        // while its siblings said whom they sacrifice (#141).
                        // (A SacrificeThis cost carries no choice and no
                        // sacrifice id, so it never reaches this arm.)
                        Some(_) => ", sacrificing itself".to_string(),
                        None => String::new(),
                    };
                    // The row's identity is (source, targets, sacrifice) —
                    // two Wooden Stakes, or one Stake offered against two
                    // identical tokens, collide on the SOURCE as readily as
                    // on the target, and only the target half was ever
                    // disambiguated (issue #257).
                    let mut ids = vec![object_id.0];
                    ids.extend(targets.iter().filter_map(|t| match t {
                        Target::Object(id) => Some(id.0),
                        _ => None,
                    }));
                    if let Some(sac) = sacrifice { ids.push(sac.0); }
                    let label = match desc {
                        Some(d) => MenuLabel {
                            text: format!("{}: {d}{}{sac_suffix}", Self::perm_name(view, *object_id),
                                Self::targets_suffix(view, targets)),
                            ids,
                        },
                        None => MenuLabel {
                            text: format!("{}{sac_suffix}", Self::format_action(view, action)),
                            ids,
                        },
                    };

                    display.push(DisplayEntry::Direct(i));
                    display_labels.push(label);
                }
                // Choose-cards-from-hand menus: two Forests are
                // interchangeable, so options whose labels render identically
                // are one choice, not several. Ten of a 35-entry bottoming
                // menu were unreadable duplicates (#54). Only these variants:
                // elsewhere an identical label can hide a genuinely different
                // action (two abilities on one permanent — #61).
                Action::BottomCards { .. } | Action::DiscardCards { .. } => {
                    let label = Self::format_action(view, action);
                    if display_labels.iter().any(|l: &MenuLabel| l.full() == label) { continue; }
                    display.push(DisplayEntry::Direct(i));
                    display_labels.push(MenuLabel::plain(label));
                }
                _ => {
                    display.push(DisplayEntry::Direct(i));
                    display_labels.push(Self::menu_label_for(view, action));
                }
            }
        }

        // Append deferred tap actions after cast spells.
        for (action_idx, label) in deferred_taps {
            display.push(DisplayEntry::Direct(action_idx));
            display_labels.push(label);
        }

        // Concede is always last.
        if let Some((action_idx, label)) = deferred_concede {
            display.push(DisplayEntry::Direct(action_idx));
            display_labels.push(label);
        }

        (display, display_labels)
    }

}

impl Player for CliPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Action {
        let legal_actions = &legal.actions;

        // X-cost funding: prompt the user for an X value and auto-distribute
        // across pool mana and tap sources (pool first, then by category).
        // A richer per-source UI could be added later.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, description, .. }) =
            legal.resolution_prompt.as_ref()
        {
            // Nothing is spent at either funding prompt now: an X-cost
            // ability announces X before it pays, the way a spell does
            // (CR 601.2b before 601.2h via 602.2b, issue #290), so both are
            // cancellable (#123).
            return Self::prompt_x_funding(view, options, description, true);
        }

        // Exile-from-graveyard: prompt for a space-separated list of indices.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseExileFromGraveyard {
            options, min, max, description, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return Self::prompt_exile_from_graveyard(view, options, *min, *max, description);
        }

        // An "up to N" target slot: the same marking screen. The engine
        // stopped enumerating one cast per subset (issue #360), so this is
        // where the targets are chosen.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseTargetSet {
            options, min, max, description, fixed, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return Self::prompt_target_set(view, options, fixed, *min, *max, description);
        }

        // A set of cards out of a list: a checklist, not a menu of every
        // way of choosing (issue #360). Toggling is the whole interaction.
        if let Some(prompt) = legal.set_prompt.as_ref() {
            return Self::prompt_card_set(view, prompt, legal.context.as_deref().unwrap_or("CHOOSE CARDS"));
        }

        // Pile division: prompt for the indices that form pile 1.
        if let Some(mtg_engine::state::ResolutionChoiceKind::DividePermanentsIntoPiles {
            permanents, description, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return Self::prompt_pile_division(view, permanents, description);
        }

        // An ordering is one decision, answered on one screen that shows
        // everything being ordered (issue #325): the triggers a player puts
        // on the stack (CR 603.3b), or the blockers an attacker's damage is
        // assigned among (CR 509.2). These prompts are flat `ChosenIndex`
        // lists too, and past eight options the card browser below used to
        // take them — the whole board, the stack and every pane shortcut
        // gone at the one decision where the stack is what the player needs.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseTriggerOrder {
            description, options, details, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return Self::prompt_ordering(view, &OrderingPrompt {
                kind: OrderingKind::Triggers, description, options, details,
            });
        }
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseDamageAssignmentOrder {
            description, options, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return Self::prompt_ordering(view, &OrderingPrompt {
                kind: OrderingKind::Blockers, description, options, details: &[],
            });
        }

        // Special case: library search — show interactive card browser.
        if legal_actions.iter().all(|a| matches!(a, Action::ResolveChoice { .. }))
            && legal_actions.len() > 1
        {
            // ChosenCard choices (library/revealed search). An OPTIONAL
            // search (CR 701.19b: "you may search...") carries one trailing
            // ChosenTarget(None) decline — that entry used to disqualify the
            // browser and dump the most common kind of search as a flat
            // numbered list (issue #111). The browser now takes the decline
            // along and offers it on Esc.
            let decline = legal_actions.iter().find(|a| matches!(a,
                Action::ResolveChoice { choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(None) }
            )).cloned();
            let all_chosen_cards = legal_actions.iter().all(|a| matches!(a,
                Action::ResolveChoice {
                    choice: mtg_engine::actions::ResolvedChoice::ChosenCard(_)
                        | mtg_engine::actions::ResolvedChoice::ChosenTarget(None)
                }
            ));
            let card_count = legal_actions.len() - usize::from(decline.is_some());
            // A long list of card NAMES is the same kind of question and
            // wants the same browser (issue #255). Kept to genuinely long
            // ones: a modal choice or a card-type choice is also `ChosenIndex`
            // and reads better as three numbered rows. And kept to prompts
            // that ARE about card names: any flat `ChosenIndex` list past
            // eight entries used to qualify, which is how a trigger-ordering
            // prompt with nine triggers turned into a card search (#325).
            let naming_cards = card_count > 8 && matches!(legal.resolution_prompt,
                Some(mtg_engine::state::ResolutionChoiceKind::ChooseCardName { .. }));

            if (all_chosen_cards && card_count > 3) || naming_cards {
                let title = legal.context.as_deref().unwrap_or("Choose a card");
                return Self::library_search_ui(view, legal_actions, title, decline);
            }
        }

        let has_pass = legal_actions.iter().any(|a| matches!(a, Action::PassPriority));

        // Auto-pass when the only options are Pass and Concede.
        // (The engine handles the smarter mana-ability check with potential mana.)
        let only_pass_concede = legal_actions.iter().all(|a| matches!(a,
            Action::PassPriority | Action::Concede
        ));
        if only_pass_concede && has_pass {
            return Action::PassPriority;
        }

        // Pass mode: auto-pass until a break condition is met.
        if let Some(ref mode) = self.pass_mode.clone() {
            if has_pass {
                if Self::should_break_pass(view, legal, mode).is_some() {
                    self.pass_mode = None;
                } else {
                    return Action::PassPriority;
                }
            }
        }

        let (display, display_labels) = Self::build_action_menu(view, legal);

        // Issue #71: a decision of a different identity (seat or prompt
        // kind) must not consume keystrokes typed against an earlier
        // prompt. Every menu with a Pass option is the one ordinary
        // priority menu — a single kind, whatever the step, so holding
        // Enter to pass through your own turn keeps working. A menu
        // without Pass is a mandatory choice (discard, sacrifice,
        // bottoming, search), keyed by its context string.
        let kind = if has_pass { "priority" } else { legal.context.as_deref().unwrap_or("") };
        self.drain_stale_input(kind);

        let mut notice: Option<String> = self.pending_notice.take();
        let mut menu_offset = 0usize;
        loop {
            let pass_label = self.pass_mode.as_ref().map(|m| match m {
                PassMode::UntilNextTurn { .. } => "AUTO-PASS",
            });
            // The notice goes UNDER the heading, not in place of it: the
            // question a mandatory prompt is asking has to stay on the
            // screen while the player is being told their last keystroke
            // was refused.
            let page = Self::render_paged_noticed(view, Some(&display_labels),
                legal.context.as_deref(), notice.take().as_deref(),
                &view.display_log, &self.card_filter, pass_label, menu_offset);


            // Read input
            let (term_w, _) = terminal::size().unwrap_or((100, 30));
            let side = term_w as usize / 5;
            let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
            // The menu is repainted on resize, so the frame is never left
            // at the old width for the player to type blindly into (#250).
            let filter = self.card_filter.clone();
            let context = legal.context.clone();
            let input = Self::read_line_with_search_redrawing(col, &|| {
                Self::render_paged(view, Some(&display_labels), context.as_deref(),
                    &view.display_log, &filter, pass_label, menu_offset);
            });

            // '/' triggers card search immediately (returns None to re-render)
            if input.is_none() {
                Self::run_card_search(view, &display_labels, context.as_deref(), menu_offset);
                continue;
            }
            let input = input.unwrap();

            // Keyboard shortcuts
            match input.as_str() {
                "g" => {
                    Self::show_graveyards(view);
                    continue;
                }
                "e" => {
                    Self::show_exile(view);
                    continue;
                }
                "i" => {
                    Self::show_battlefield_inspector(view);
                    continue;
                }
                "s" => {
                    Self::show_stack(view);
                    continue;
                }
                "f" => {
                    // The switch, both ways. Auto-pass could be turned on and
                    // never off: nothing cleared `pass_mode` but a break, no
                    // key toggled it, and at the one kind of screen that ever
                    // SHOWS `[AUTO-PASS]` — a mandatory menu, with no Pass —
                    // this arm was wrapped in `if has_pass` and swallowed the
                    // key without a word (issue #296).
                    if self.pass_mode.is_some() {
                        self.pass_mode = None;
                        notice = Some("Auto-pass off.".to_string());
                        continue;
                    }
                    if !has_pass {
                        notice = Some(
                            "Auto-pass passes priority, and this prompt is a mandatory choice \
                             — answer it with an option number.".to_string());
                        continue;
                    }
                    // Pass until my next Main Phase 1 (F6-like). The break
                    // check runs against the CURRENT prompt first: if it
                    // would already break here, engaging would pass over the
                    // decision it exists to stop for (issue #48).
                    match Self::try_engage_auto_pass(view, legal) {
                        Ok(mode) => {
                            // Pressing 'f' at your own main phase is a
                            // deliberate skip of the rest of the turn — but it
                            // used to be a silent one, and the same menu one
                            // step later refused with a message that named the
                            // spell it had just declined (issue #294). Say
                            // what it declined, on the next screen shown.
                            let declined = legal.actions.iter().filter(|a| matches!(a,
                                Action::PlayLand { .. }
                                | Action::CastSpell { .. }
                                | Action::ActivateAbility { .. })).count();
                            self.pass_mode = Some(mode);
                            self.pending_notice = Some(if declined == 0 {
                                "Auto-pass on — passing to your next Main Phase 1. \
                                 Press f again to turn it off.".to_string()
                            } else {
                                format!("Auto-pass on — it declined {declined} action(s) at that \
                                         prompt, and passes to your next Main Phase 1. \
                                         Press f again to turn it off.")
                            });
                            return Action::PassPriority;
                        }
                        Err(reason) => {
                            notice = Some(format!(
                                "Auto-pass not engaged: this prompt has {}.", reason.describe()));
                        }
                    }
                    continue;
                }
                "__hot_reload__" => {
                    // Hot reload triggered by rapid 'rr' in raw mode. This is
                    // the PROCESS stopping, not a seat quitting: it used to be
                    // spelled `Concede`, which the engine recorded, so asking
                    // for a rebuild wrote "p0 conceded" permanently into the
                    // run's --log followed by a fresh GAME_START and a replay
                    // of the same game (issue #233).
                    return Action::AbandonGame;
                }
                "l" => {
                    Self::show_log(&view.display_log);
                    continue;
                }
                // Page a menu longer than the pane (issue #96); wraps back
                // to the top after the last page. A no-op on a menu that
                // fits, so 'm' never falls through to be misread as input.
                "m" => {
                    menu_offset = Self::next_menu_offset(
                        menu_offset, page.shown, display_labels.len());
                    continue;
                }
                // Backwards, because paging was forward-only: overshooting a
                // long list meant going all the way around (issue #255).
                "p" => {
                    menu_offset = Self::prev_menu_offset_lines(
                        &page.heights, page.avail, menu_offset, page.marker_h);
                    continue;
                }

                "" => {
                    // Enter = pass if available. Without a Pass option this
                    // is a mandatory choice with no "do nothing" — refuse
                    // bare Enter out loud instead of silently re-rendering:
                    // 30 swallowed Enters at a cleanup discard read as a
                    // hung game (issue #76, the menu sibling of #42).
                    if has_pass {
                        return Action::PassPriority;
                    }
                    notice = Some(format!("{} — mandatory: enter an option number",
                        legal.context.as_deref().unwrap_or("This choice")));
                    continue;
                }
                _ => {}
            }

            // Deck browser
            if input == "d" {
                Self::show_deck_browser(view);
                continue;
            }

            // (Card search is handled before this point via read_line_with_search)

            if let Ok(idx) = input.parse::<usize>() {
                if idx < display.len() {
                    match &display[idx] {
                        DisplayEntry::Direct(action_idx) => {
                            let action = &legal_actions[*action_idx];
                            if matches!(action, Action::Concede) {
                                let _ = execute!(stdout(), cursor::MoveTo(col, cursor::position().unwrap_or((0, 24)).1));
                                if !Self::confirm_yn("  Are you sure you want to concede? (y/n)> ") {
                                    continue;
                                }
                            }
                            return action.clone();
                        }
                        DisplayEntry::Cast(cs_idx) => {
                            let cs = &legal.castable_spells[*cs_idx];
                            if let Some(action) = Self::choose_targets(view, cs) {
                                return action;
                            }
                            // User cancelled target selection — re-render
                        }
                    }
                    continue;
                }
            }
            // Invalid input: say so (issue #76 — a silent re-render at a
            // full-screen menu is indistinguishable from a hung game).
            notice = Some(format!("Invalid input '{}' — enter a number 0-{}",
                quote_input(&input), display.len().saturating_sub(1)));
        }
    }

}

impl CliPlayer {
    /// Show the game state with a spinning indicator on the opponent's
    /// caret while the AI thinks. Drop the returned handle to stop.
    #[must_use]
    pub fn start_thinking(view: &GameView) -> SpinnerHandle {
        Self::render(view, None, &view.display_log, "", None);

        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
        // Opponent stats are always at row 2 from the human's perspective
        // (row 0 = turn bar, row 1 = BATTLEFIELD label, row 2 = opp stats)
        let spinner_row: u16 = 2;

        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let running_clone = running.clone();

        let handle = std::thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            let mut out = stdout();
            while running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = execute!(out,
                    cursor::SavePosition,
                    cursor::MoveTo(col, spinner_row),
                    SetForegroundColor(Color::Red),
                    Print(frames[i % frames.len()]),
                    ResetColor,
                    cursor::RestorePosition,
                );
                let _ = out.flush();
                std::thread::sleep(std::time::Duration::from_millis(80));
                i += 1;
            }
            // Don't restore caret — the next render overwrites it.
        });

        let _ = handle; // detach — thread stops when `running` goes false
        SpinnerHandle { running }
    }

    pub fn choose_combat(&mut self, view: &GameView, prompt: &CombatPrompt) -> Action {
        match prompt {
            CombatPrompt::ChooseAttackers { eligible, .. } => {
                // In pass mode, skip attacking only if we have no eligible creatures.
                // If we have creatures, break pass mode so the player can decide.
                if self.pass_mode.is_some() {
                    if eligible.is_empty() {
                        return Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] };
                    }
                    // We have creatures to attack with — break pass mode.
                    self.pass_mode = None;
                }
                Self::choose_attackers(view, prompt)
            }
            CombatPrompt::ChooseBlockers { eligible_blockers, .. } => {
                // Always break pass mode for blockers if we have eligible blockers.
                if !eligible_blockers.is_empty() {
                    self.pass_mode = None;
                }
                // If no eligible blockers, auto-declare zero blockers.
                if eligible_blockers.is_empty() {
                    return Action::DeclareBlockers { assignments: vec![] };
                }
                Self::choose_blockers(view, prompt)
            }
        }
    }
}

impl CliPlayer {
    /// The oracle-text lines the CARDS panel should print, given that the
    /// panel prints its own keyword line above them and its own flashback
    /// line below.
    ///
    /// A line is dropped when the panel already says what it says. The old
    /// filter dropped only a line that was *exactly one* keyword name, so two
    /// common shapes printed twice in a ~22-column panel (issue #265):
    ///
    /// - a keyword line naming more than one keyword — Elite Inquisitor's
    ///   "First strike, vigilance" under the generated "First strike,
    ///   Vigilance";
    /// - the flashback cost, which every flashback card's oracle text carries
    ///   with its reminder text, under the generated "Flashback {cost}".
    ///
    /// Dropping the oracle's flashback line rather than the generated one
    /// takes the reminder text with it, which is what was eating Army of the
    /// Damned's whole card box.
    fn card_panel_oracle_lines(oracle_text: &str, has_flashback_line: bool) -> Vec<&str> {
        const KEYWORD_NAMES: &[&str] = &[
            "Flying", "First strike", "Double strike", "Trample", "Deathtouch",
            "Lifelink", "Vigilance", "Flash", "Reach", "Haste", "Defender",
            "Hexproof", "Intimidate", "Menace", "Indestructible",
        ];
        // Every comma-separated part is a keyword name, so the generated
        // keyword line above already carries the whole line.
        let all_keywords = |line: &str| {
            let mut parts = line.split(',').map(str::trim).filter(|p| !p.is_empty()).peekable();
            parts.peek().is_some()
                && parts.all(|p| {
                    let p = p.trim_end_matches('.');
                    KEYWORD_NAMES.iter().any(|kw| p.eq_ignore_ascii_case(kw))
                })
        };
        oracle_text
            .split('\n')
            .filter(|line| {
                let trimmed = line.trim();
                if all_keywords(trimmed) {
                    return false;
                }
                // The generated line below prints the cost without the
                // reminder text; only drop this when there is one.
                !(has_flashback_line
                    && trimmed.len() >= "flashback ".len()
                    && trimmed[.."flashback ".len()].eq_ignore_ascii_case("flashback "))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtg_engine::engine::LegalActions;
    use mtg_engine::ids::PlayerId;
    use mtg_engine::types::ManaPool;

    /// One equip label per Champion, differing only in whom it targets and

    /// whom it sacrifices.
    fn hauberk_row(target: u64, sacrifice: u64) -> MenuLabel {
        MenuLabel {
            text: format!(
                "Demonmail Hauberk (your): Equip—Sacrifice a creature targeting Champion of the Parish {t}/{t} (your), sacrificing Champion of the Parish {s}/{s} (your)",
                t = target, s = sacrifice),
            ids: vec![7, target, sacrifice],
        }
    }

    /// Everything the row said, with the line breaks taken back out — what
    /// a wrapped row must always equal (issue #318).
    fn unwrapped(lines: &[String]) -> String {
        lines.join(" ")
    }

    /// Issue #258, then #318: eight Demonmail Hauberk equips that differ only
    /// in which Champion they target rendered as two byte-identical lines,
    /// because a clip has to lose something and lost the target. Wrapped,
    /// every row is whole: each line fits the pane, and the lines of a row
    /// read back to exactly the row.
    #[test]
    fn a_wrapped_row_fits_the_pane_and_loses_nothing() {
        let rows: Vec<MenuLabel> = (1..=4).flat_map(|t| (1..=2).map(move |s| hauberk_row(t, s)))
            .collect();
        assert_eq!(rows.len(), 8);

        let wrapped = CliPlayer::wrap_menu_rows(&rows, 113);
        for (row, lines) in rows.iter().zip(&wrapped) {
            assert!(lines.len() >= 2, "the row is wider than 113 columns: {lines:?}");
            for line in lines {
                assert!(str_cols(line) <= 113, "a line overflows the panel: {line:?}");
            }
            assert_eq!(unwrapped(lines), row.full(), "nothing is cut");
        }
        let mut texts: Vec<String> = wrapped.iter().map(|l| unwrapped(l)).collect();
        texts.sort();
        texts.dedup();
        assert_eq!(texts.len(), 8, "eight different equips, eight different rows");
    }

    /// Issue #257: rows that name different objects must be tellable apart,
    /// and rows that name the SAME objects must stay alike, or #54's
    /// collapse of interchangeable duplicates is undone. Nothing is clipped
    /// now, so the only collision left is two rows that genuinely read the
    /// same — a Stake against two identical tokens — and the id settles it.
    #[test]
    fn colliding_menu_rows_are_told_apart_by_object_id() {
        let stake = |target: u64| MenuLabel {
            text: "Wooden Stake (your): Equip targeting Zombie 2/2 (opp)".to_string(),
            ids: vec![7, target],
        };
        let texts = CliPlayer::menu_row_texts(&[stake(3), stake(4)]);
        assert_ne!(texts[0], texts[1], "different targets, different rows: {texts:#?}");
        assert!(texts[0].ends_with("(#3)") && texts[1].ends_with("(#4)"),
            "told apart by object id: {texts:#?}");

        let same = CliPlayer::menu_row_texts(&[stake(3), stake(3)]);
        assert_eq!(same[0], same[1],
            "the same offer twice is one row twice, not two numbered ones");
        let distinct = CliPlayer::menu_row_texts(&[hauberk_row(3, 1), hauberk_row(4, 1)]);
        assert!(!distinct[0].contains('#'),
            "rows that already read differently get no id: {distinct:#?}");
    }

    /// A pane too narrow for a word cuts the word at the column and carries
    /// the rest down — it does not drop it. Every character of the row is on
    /// the screen somewhere.
    #[test]
    fn a_narrow_pane_wraps_and_keeps_every_character() {
        for width in [10usize, 4, 1] {
            let lines = CliPlayer::wrap_row(&hauberk_row(3, 1).full(), width);
            for line in &lines {
                assert!(str_cols(line) <= width, "width {width}: a line overflows: {line:?}");
            }
            let squashed: String = lines.concat().replace(' ', "");
            assert_eq!(squashed, hauberk_row(3, 1).full().replace(' ', ""),
                "width {width}: every character survives");
        }
    }

    /// Issue #109 in the menu: counting chars let a wide-character label
    /// overflow the panel and paint over the CARDS pane beside it (#53). The
    /// wrap measures display columns.
    #[test]
    fn the_menu_wrap_measures_display_columns_not_chars() {
        let wide = "四人日本語のカード名がとても長い場合のテスト";
        assert!(wide.chars().count() < str_cols(wide), "test precondition: wide chars");
        let lines = CliPlayer::wrap_row(wide, 20);
        assert!(lines.len() >= 3, "{lines:?}");
        for line in &lines {
            assert!(str_cols(line) <= 20, "a line overflows the panel: {line:?}");
        }
        assert_eq!(lines.concat(), wide);
    }

    /// Issue #318, the repro: at 100 columns the middle panel gives a
    /// three-card bottoming row 52 columns, and 14 of the 25 rows clipped to
    /// 5 identical lines. Wrapped, all 25 are whole and distinct, and a row
    /// breaks after a card — after the comma — so each line ends on a name.
    #[test]
    fn the_bottoming_menu_of_issue_318_reads_whole_at_100_columns() {
        let hand = ["Disciple of Griselbrand", "Curse of Death's Hold", "Charmbreaker Devils",
                    "Kessig Cagebreakers", "Geist of Saint Traft", "Hollowhenge Scavenger"];
        let mut rows: Vec<MenuLabel> = Vec::new();
        for a in 0..hand.len() {
            for b in a + 1..hand.len() {
                for c in b + 1..hand.len() {
                    rows.push(MenuLabel::plain(format!("Bottom {}, {}, {}", hand[a], hand[b], hand[c])));
                }
            }
        }
        assert_eq!(rows.len(), 20);

        let wrapped = CliPlayer::wrap_menu_rows(&rows, 52);
        let mut seen = std::collections::HashSet::new();
        for (row, lines) in rows.iter().zip(&wrapped) {
            for line in lines {
                assert!(str_cols(line) <= 52, "a line overflows the panel: {line:?}");
            }
            assert_eq!(unwrapped(lines), row.full(), "nothing is cut");
            assert!(seen.insert(unwrapped(lines)), "distinct rows stay distinct");
            // Every line but the last ends on a whole card.
            for line in &lines[..lines.len() - 1] {
                assert!(line.ends_with(','), "a list breaks after an item: {line:?}");
            }
        }
    }

    /// The break falls after the last comma that fits, or, with no comma in
    /// reach, at the last space — and a single word wider than the pane is
    /// cut at the column rather than pushing the frame apart.
    #[test]
    fn a_row_breaks_after_a_comma_before_it_breaks_at_a_space() {
        assert_eq!(CliPlayer::wrap_row("Bottom Forest, Island, Swamp", 22),
            vec!["Bottom Forest, Island,", "Swamp"]);
        assert_eq!(CliPlayer::wrap_row("Bottom Forest, Island, Swamp", 16),
            vec!["Bottom Forest,", "Island, Swamp"]);
        assert_eq!(CliPlayer::wrap_row("Cast Brimstone Volley targeting Opponent", 24),
            vec!["Cast Brimstone Volley", "targeting Opponent"]);
        assert_eq!(CliPlayer::wrap_row("Pass priority", 40), vec!["Pass priority"]);
        assert_eq!(CliPlayer::wrap_row("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(CliPlayer::wrap_row("", 10), vec![""], "an empty row is one empty line");
        assert_eq!(CliPlayer::wrap_row("anything at all", 0), vec!["anything at all"],
            "no width is no wrapping, not an endless loop");
    }

    /// Marking cards is toggling, and the idle key confirms rather than
    /// choosing something (#123).
    #[test]
    fn a_card_set_is_marked_by_number_and_confirmed_by_enter() {
        use SetInput::*;
        assert!(matches!(CliPlayer::parse_card_set_input("", 7, false), Confirm));
        assert!(matches!(CliPlayer::parse_card_set_input("   ", 7, false), Confirm));
        assert!(matches!(CliPlayer::parse_card_set_input("3", 7, false), Toggle(ref v) if *v == vec![3]));
        assert!(matches!(CliPlayer::parse_card_set_input("0 2, 5", 7, false), Toggle(ref v) if *v == vec![0, 2, 5]));
        // Toggling the same card twice in one line is two toggles, which is
        // what "toggle" means — not an error and not a set-union.
        assert!(matches!(CliPlayer::parse_card_set_input("2 2", 7, false), Toggle(ref v) if *v == vec![2, 2]));
        assert!(matches!(CliPlayer::parse_card_set_input("a", 7, false), All));
        assert!(matches!(CliPlayer::parse_card_set_input("none", 7, false), None));
        assert!(matches!(CliPlayer::parse_card_set_input("g", 7, false), Pane('g')));
        assert!(matches!(CliPlayer::parse_card_set_input("m", 7, false), NextPage));
        assert!(matches!(CliPlayer::parse_card_set_input("p", 7, false), PrevPage));
        match CliPlayer::parse_card_set_input("7", 7, false) {
            Invalid(why) => assert!(why.contains("numbered 0-6"), "{why}"),
            other => panic!("out of range is refused, got {other:?}"),
        }
        match CliPlayer::parse_card_set_input("two", 7, false) {
            Invalid(why) => assert!(why.contains("not a number"), "{why}"),
            other => panic!("a word is refused, got {other:?}"),
        }
        // `c` abandons a choice that may be abandoned — an additional cost
        // — and is just a word at one that may not.
        assert!(matches!(CliPlayer::parse_card_set_input("c", 7, true), Cancel));
        assert!(matches!(CliPlayer::parse_card_set_input("cancel", 7, true), Cancel));
        assert!(matches!(CliPlayer::parse_card_set_input("c", 7, false), Invalid(_)));
    }

    /// A horizontal rule carries a label and nothing that has to be read.
    #[test]
    fn a_rule_is_one_line_and_the_question_goes_under_it() {
        // Short enough to be the rule's own label.
        assert_eq!(CliPlayer::rule_title("MAIN PHASE 1", 60), ("MAIN PHASE 1".into(), None));
        assert_eq!(CliPlayer::rule_title("BOTTOM 3 CARDS AFTER MULLIGAN", 60),
            ("BOTTOM 3 CARDS AFTER MULLIGAN".into(), None));

        // The card-prompt convention: the source names the rule, the
        // question goes below it.
        let long = "Delver of Secrets: reveal Delver of Secrets from the top of your \
library? (not an instant or sorcery — no transform)";
        let (title, detail) = CliPlayer::rule_title(long, 60);
        assert_eq!(title, "Delver of Secrets");
        assert_eq!(detail.as_deref(), Some("reveal Delver of Secrets from the top of your library? (not an instant or sorcery — no transform)"));
        assert!(str_cols(&title) <= 60, "the label fits the rule: {title:?}");

        // A heading with no name in front of it is labelled by kind rather
        // than cut in half, and the whole of it goes below.
        let bare = "choose a card to discard, then another, and then explain \
yourself at some considerable length";
        let (title, detail) = CliPlayer::rule_title(bare, 40);
        assert_eq!(title, "CHOICE");
        assert_eq!(detail.as_deref(), Some(bare));

        // And a name that is itself too long for the rule does not become
        // the label.
        let (title, _) = CliPlayer::rule_title(
            "Some Extremely Long Card Name That Runs On: do a thing", 20);
        assert_eq!(title, "CHOICE");
    }

    /// The rows that describe a menu — the paging marker and the hint line —
    /// are kept inside the panel by wrapping, not by cutting. Every
    /// continuation lines up under the first line's own indent.
    #[test]
    fn the_chrome_under_a_menu_wraps_inside_the_panel() {
        let hints = "  [/=search] [d=deck] [l=log] [g=gy] [e=exile] [s=stack] [m/p=page]";
        let lines = CliPlayer::wrap_indented(hints, 64);
        assert!(lines.len() > 1, "67 columns of hints do not fit 64: {lines:?}");
        assert!(lines.iter().all(|l| str_cols(l) <= 64), "nothing overflows: {lines:?}");
        assert!(lines.iter().all(|l| l.starts_with("  ")), "the indent is kept: {lines:?}");
        assert!(lines.iter().all(|l| !l.trim_end().ends_with('=')),
            "a key is never cut in half: {lines:?}");
        assert_eq!(lines.concat().split_whitespace().collect::<Vec<_>>(),
            hints.split_whitespace().collect::<Vec<_>>(),
            "and every key survives, which clipping is exactly what did not");

        // A marker measures the same whichever page it ends up describing,
        // so a page can reserve its rows before choosing one.
        let marker = CliPlayer::page_marker(0, 17, 24, MENU_PAGE_KEYS);
        assert_eq!(marker,
            "  \u{2026} showing 0-16 of 0-24 \u{2014} m/p = next/prev page (any number works)");
        assert_eq!(CliPlayer::wrap_indented(&marker, 64).len(),
            CliPlayer::marker_lines(24, MENU_PAGE_KEYS, 64),
            "what is reserved is what is drawn");
        assert_eq!(CliPlayer::marker_lines(24, MENU_PAGE_KEYS, 200), 1,
            "and a wide pane needs one row");

        // The reserve reaches the pager: a two-line marker takes two rows
        // off the budget, not one.
        assert_eq!(CliPlayer::menu_page_lines(&[1; 10], 6, 0, 1), (0, 5, true));
        assert_eq!(CliPlayer::menu_page_lines(&[1; 10], 6, 0, 2), (0, 4, true));
    }

    /// A page is measured in lines, not rows, once rows can wrap (issue
    /// #318): three two-line rows fill a six-line pane, the marker takes a
    /// line when the menu does not fit, and a row taller than the pane is
    /// still shown rather than skipped.
    #[test]
    fn paging_counts_lines_not_rows() {
        let heights = [2usize, 2, 2, 2, 1];
        // Nine lines in a six-line pane: paged, five lines of budget, so two
        // rows fit.
        assert_eq!(CliPlayer::menu_page_lines(&heights, 6, 0, 1), (0, 2, true));
        // From the third row: 2 + 2 + 1 = 5 fits the budget exactly.
        assert_eq!(CliPlayer::menu_page_lines(&heights, 6, 2, 1), (2, 3, true));
        // Everything fits: no marker, no budget lost to it.
        assert_eq!(CliPlayer::menu_page_lines(&heights, 9, 0, 1), (0, 5, false));
        // A row taller than the pane shows anyway.
        assert_eq!(CliPlayer::menu_page_lines(&[7, 1], 3, 0, 1), (0, 1, true));
        // The unit-height case is the old behaviour exactly.
        assert_eq!(CliPlayer::menu_page(30, 10, 0), CliPlayer::menu_page_lines(&[1; 30], 10, 0, 1));
        assert_eq!(CliPlayer::menu_page(30, 10, 27), CliPlayer::menu_page_lines(&[1; 30], 10, 27, 1));
    }

    /// `p` from an uneven page lands on the page that ends just above it,
    /// and from the top on the last page — not on a fixed stride's guess.
    #[test]
    fn the_previous_page_is_exact_with_uneven_rows() {
        let heights = [2usize, 2, 2, 2, 1];
        // From row 2 (the second page), back to row 0.
        assert_eq!(CliPlayer::prev_menu_offset_lines(&heights, 6, 2, 1), 0);
        // From row 4, the budget of 5 holds rows 2 and 3 (2 + 2) — not row 1.
        assert_eq!(CliPlayer::prev_menu_offset_lines(&heights, 6, 4, 1), 2);
        // From the top, the last page: rows 2..5 (2 + 2 + 1 = 5).
        assert_eq!(CliPlayer::prev_menu_offset_lines(&heights, 6, 0, 1), 2);
        // A budget too small for even one row still steps back one row.
        assert_eq!(CliPlayer::prev_menu_offset_lines(&[7, 7, 7], 3, 2, 1), 1);
        assert_eq!(CliPlayer::prev_menu_offset_lines(&[], 6, 0, 1), 0);
    }


    /// Issue #257: an untargeted ability on one of six identically-named
    /// creatures is a different action per creature, and the row's identity
    /// starts with its source.
    #[test]
    fn an_action_is_identified_by_every_object_it_names() {
        use mtg_engine::actions::{Action, Target};
        use mtg_engine::ids::ObjectId;
        let equip = Action::ActivateAbility {
            object_id: ObjectId(7),
            ability_index: 0,
            targets: vec![Target::Object(ObjectId(73))],
            tap_plan: vec![],
            sacrifice: Some(ObjectId(74)),
            x_value: None,
            source_card_id: None,
        };
        assert_eq!(CliPlayer::action_object_ids(&equip), vec![7, 73, 74],
            "source, then targets, then sacrifice");

        let land = Action::PlayLand { object_id: ObjectId(3) };
        assert_eq!(CliPlayer::action_object_ids(&land), vec![3]);

        // Two untapped Islands are the same offer; numbering them is noise.
        let tap = Action::ActivateManaAbility { object_id: ObjectId(3), ability_index: 0 };
        assert!(CliPlayer::action_object_ids(&tap).is_empty());

        // A yes/no or an index names no object at all.
        let yes = Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::YesNoDecision(true),
        };
        assert!(CliPlayer::action_object_ids(&yes).is_empty());
    }

    /// Issue #254: a forced target and a forced sacrifice are taken without
    /// asking (CR 601.2c/601.2h), so the row has to say what they are —
    /// otherwise one keypress commits a choice the player was never shown.
    #[test]
    fn a_forced_choice_is_the_one_the_row_names() {
        use mtg_engine::actions::{CastTargetSpec, Target};
        use mtg_engine::ids::{ObjectId, PlayerId};

        // One legal target: forced, so it is named rather than prompted.
        let one = CastTargetSpec::SingleTarget(vec![Target::Player(PlayerId(0))]);
        assert_eq!(CliPlayer::forced_cast_targets(&one), vec![Target::Player(PlayerId(0))]);

        // Two: a chooser runs, and the row promises nothing.
        let two = CastTargetSpec::SingleTarget(vec![
            Target::Player(PlayerId(0)), Target::Player(PlayerId(1))]);
        assert!(CliPlayer::forced_cast_targets(&two).is_empty());

        // The wide specs always prompt, however few options they hold.
        let up_to = CastTargetSpec::UpToTargets { max: 2, options: vec![Target::Player(PlayerId(1))] };
        assert!(CliPlayer::forced_cast_targets(&up_to).is_empty());

        // Same rule on the cost half.
        assert_eq!(CliPlayer::forced_sacrifice(&[ObjectId(7)]), Some(ObjectId(7)));
        assert_eq!(CliPlayer::forced_sacrifice(&[ObjectId(7), ObjectId(8)]), None);
        assert_eq!(CliPlayer::forced_sacrifice(&[]), None);
    }

    /// And the row actually carries it: "Cast Brimstone Volley targeting
    /// you" is the line that was missing.
    #[test]
    fn a_cast_row_names_the_target_it_will_take_without_asking() {
        use mtg_engine::actions::{CastableSpell, CastTargetSpec, Target};
        use mtg_engine::ids::PlayerId;

        let v = view(Step::PrecombatMain, 5, true);
        let mut cs = CastableSpell {
            object_id: ObjectId(30),
            name: "Brimstone Volley".to_string(),
            is_flashback: false,
            from_graveyard: false,
            target_spec: CastTargetSpec::SingleTarget(vec![Target::Player(PlayerId(0))]),
            tap_plan: vec![],
            exile_x_from_gy_max: None,
            sacrifice_options: vec![],
            additional_cost_label: None,
            alternative_cost: None,
        };
        let row = CliPlayer::cast_row_label(&v, &cs).full();
        assert!(row.contains("targeting you"), "got {row:?}");

        // Two targets: a chooser runs, so the row promises nothing.
        cs.target_spec = CastTargetSpec::SingleTarget(vec![
            Target::Player(PlayerId(0)), Target::Player(PlayerId(1))]);
        let row = CliPlayer::cast_row_label(&v, &cs).full();
        assert!(!row.contains("targeting"), "got {row:?}");

        // A forced sacrifice is named; an unforced one says a choice is coming.
        cs.sacrifice_options = vec![ObjectId(41)];
        cs.additional_cost_label = Some("sacrifice a creature".into());
        let row = CliPlayer::cast_row_label(&v, &cs).full();
        assert!(row.contains("sacrificing"), "got {row:?}");
        cs.sacrifice_options = vec![ObjectId(41), ObjectId(42)];
        let row = CliPlayer::cast_row_label(&v, &cs).full();
        assert!(!row.contains("sacrificing"), "got {row:?}");
        assert!(row.contains("sacrifice a creature"), "got {row:?}");
    }

    /// Issue #261: `m` is drawn on any menu taller than the pane, and the
    /// target choosers answered it with "Invalid input 'm'" on the same
    /// frame that offered it — so every option past the first page, the
    /// Cancel row included, was reachable only by typing a number the player
    /// could not see.
    #[test]
    fn m_is_a_control_wherever_the_frame_advertises_it() {
        for rows in [ChooserRows::CancelOnly, ChooserRows::DoneThenCancel] {
            assert_eq!(CliPlayer::parse_target_input("m", 8, rows), TargetInput::NextPage,
                "{rows:?}");
        }
    }

    /// The page arithmetic the marker describes, without a terminal.
    #[test]
    fn a_menu_pages_and_wraps() {
        // Everything fits: one page, no marker.
        assert_eq!(CliPlayer::menu_page(5, 10, 0), (0, 5, false));
        // Too tall: a row is spent on the marker itself.
        assert_eq!(CliPlayer::menu_page(30, 10, 0), (0, 9, true));
        // An offset is always a paged view, even when the rest fits.
        assert_eq!(CliPlayer::menu_page(30, 10, 27), (27, 3, true));
        // An offset past the end clamps rather than underflowing.
        assert_eq!(CliPlayer::menu_page(3, 10, 99), (2, 1, true));
        assert_eq!(CliPlayer::menu_page(0, 10, 0), (0, 0, false));

        // `m` walks the pages and comes back to the top.
        let mut off = 0;
        off = CliPlayer::next_menu_offset(off, 9, 30);
        assert_eq!(off, 9);
        off = CliPlayer::next_menu_offset(off, 9, 30);
        assert_eq!(off, 18);
        off = CliPlayer::next_menu_offset(off, 9, 30);
        assert_eq!(off, 27);
        assert_eq!(CliPlayer::next_menu_offset(off, 3, 30), 0, "wraps at the end");
    }

    /// Issue #262: bare Enter used to be a committed answer at the
    /// exile-cost prompt — X=0 for Harvest Pyre, or a silently auto-picked
    /// card for a fixed-count cost. Everywhere else in this CLI the idle key
    /// is the SAFE key (#123), and on the marking screen it stays one: it
    /// confirms what is marked, and an untouched screen has nothing marked.
    #[test]
    fn an_idle_key_at_a_set_prompt_commits_nothing() {
        // Where the count is fixed, an empty confirm fails the count.
        for (min, max) in [(1usize, 1usize), (2, 2), (2, 7)] {
            let msg = CliPlayer::set_count_error(0, min, max);
            assert!(msg.contains("0 marked"), "min={min} max={max}: {msg}");
        }
        // Where an empty answer is legal, the count does not refuse it — so
        // the screen does, until the player says so on purpose.
        assert!(SET_NOTHING_MARKED.contains("n for none"),
            "the way to answer 'none' is named: {SET_NOTHING_MARKED}");
        assert!(matches!(CliPlayer::parse_card_set_input("", 3, false), SetInput::Confirm));
        assert!(matches!(CliPlayer::parse_card_set_input("n", 3, false), SetInput::None),
            "and 'none' is a mark, not a confirm");
    }

    /// Issue #325: the ordering prompt reads one line — the indices in
    /// order, in any spacing, commas allowed — and refuses anything that is
    /// not every index exactly once, saying which. Enter alone keeps the
    /// order shown; the pane keys and pagers are the usual letters.
    #[test]
    fn an_ordering_is_every_index_exactly_once() {
        assert_eq!(CliPlayer::parse_order_input("2 0 1", 3), OrderInput::Order(vec![2, 0, 1]));
        assert_eq!(CliPlayer::parse_order_input(" 2,0, 1 ", 3), OrderInput::Order(vec![2, 0, 1]));
        assert_eq!(CliPlayer::parse_order_input("", 3), OrderInput::Order(vec![0, 1, 2]),
            "Enter keeps the order shown");
        assert_eq!(CliPlayer::parse_order_input("s", 3), OrderInput::Pane('s'));
        assert_eq!(CliPlayer::parse_order_input("i", 3), OrderInput::Pane('i'));
        assert_eq!(CliPlayer::parse_order_input("m", 3), OrderInput::NextPage);
        assert_eq!(CliPlayer::parse_order_input("p", 3), OrderInput::PrevPage);
        for (input, why) in [
            ("0 1", "missing 2"), ("0 1 1", "listed twice"), ("0 1 3", "out of range"),
            ("0 x 1", "not a number"), ("0 1 2 2", "listed twice"),
        ] {
            match CliPlayer::parse_order_input(input, 3) {
                OrderInput::Invalid(msg) => assert!(msg.contains(why), "{input:?}: {msg}"),
                other => panic!("{input:?} was accepted as {other:?}"),
            }
        }
    }

    /// A row of the ordering screen says whose trigger it is (by id and P/T),
    /// what it does, and what set it off; without the engine's details it is
    /// the option text.
    #[test]
    fn an_ordering_row_says_whose_what_and_why() {
        use mtg_engine::state::TriggerOrderOption;
        let v = view(Step::PrecombatMain, 8, true);
        let details = vec![TriggerOrderOption {
            source: ObjectId(34), source_name: "Unruly Mob".into(), power_toughness: Some((1, 1)),
            kind: "triggered ability".into(), ability: "put a +1/+1 counter on Unruly Mob".into(),
            cause: "Unruly Mob (#23) died".into(),
        }];
        let options = vec!["Unruly Mob's triggered ability (put a +1/+1 counter on Unruly Mob) [source 1/1, #34]".to_string()];
        let rows = CliPlayer::ordering_rows(&v, &OrderingPrompt {
            kind: OrderingKind::Triggers, description: "d", options: &options, details: &details });
        assert_eq!(rows, vec![vec![
            "Unruly Mob (#34) 1/1 — triggered ability: put a +1/+1 counter on Unruly Mob".to_string(),
            "triggered by: Unruly Mob (#23) died".to_string(),
        ]]);
        let bare = CliPlayer::ordering_rows(&v, &OrderingPrompt {
            kind: OrderingKind::Blockers, description: "d", options: &options, details: &[] });
        assert_eq!(bare, vec![vec![options[0].clone()]]);
    }

    /// The ordering screen's body pages when it is taller than the terminal,
    /// keeping a line for the marker, and `m`/`p` walk the pages and wrap.
    #[test]
    fn the_ordering_screen_pages_its_body() {
        let one = BodyPage::new(5, 10, 0);
        assert!(!one.paged);
        assert_eq!((one.start, one.end), (0, 5));
        let first = BodyPage::new(20, 6, 0);
        assert!(first.paged);
        assert_eq!((first.start, first.end), (0, 5), "five lines, one for the marker");
        let second = BodyPage::new(20, 6, first.next_offset());
        assert_eq!((second.start, second.end), (5, 10));
        let last = BodyPage::new(20, 6, BodyPage::new(20, 6, 0).prev_offset());
        assert_eq!((last.start, last.end), (15, 20), "p from the top is the last page");
        assert_eq!(last.next_offset(), 0, "m from the last page wraps to the top");
        assert_eq!(second.prev_offset(), 0);
    }

    /// Issue #333: nothing on the battlefield said a permanent was a

    /// legend, so the legend rule (CR 704.5j) fired with no warning. A
    /// legendary creature's row carries "legendary" ahead of its keywords;
    /// a non-creature legend gets the word after its name.
    #[test]
    fn a_legends_battlefield_row_says_so() {
        let mut mikaeus = creature(24, "Mikaeus, the Lunarch", 0);
        mikaeus.supertypes = vec![mtg_engine::types::Supertype::Legendary];
        mikaeus.keywords = vec![mtg_engine::types::Keyword::Flying];
        let (head, elastic, _flags) = CliPlayer::creature_row_parts(&mikaeus, None);
        assert_eq!(head, "Mikaeus, the Lunarch 2/2");
        assert_eq!(elastic, " (legendary, flying)");

        let plain = creature(25, "Grizzly Bears", 0);
        let (_, elastic, _) = CliPlayer::creature_row_parts(&plain, None);
        assert_eq!(elastic, "", "a non-legend says nothing about it");

        let mut grimoire = creature(26, "Grimoire of the Dead", 0);
        grimoire.card_types = vec![CardType::Artifact];
        grimoire.supertypes = vec![mtg_engine::types::Supertype::Legendary];
        assert_eq!(CliPlayer::legend_mark(&grimoire), " (legendary)");
        assert_eq!(CliPlayer::legend_mark(&plain), "");
    }

    /// Issue #322: a key the line reader does not bind — Tab, an arrow,

    /// Home, a function key — used to be dropped, and dropping it
    /// concatenated the digits typed either side of it: `0 <Tab> 1` was the
    /// buffer `01`, accepted as option 1, at a prompt where `0 1` is refused.
    /// Such a key now separates what is typed around it, so the same
    /// keystrokes read `0 1` and get the same refusal.
    #[test]
    fn an_unbound_key_between_two_digits_keeps_them_apart() {
        let none = KeyModifiers::NONE;
        for (name, code) in [
            ("Tab", KeyCode::Tab), ("BackTab", KeyCode::BackTab),
            ("Left", KeyCode::Left), ("Right", KeyCode::Right),
            ("Home", KeyCode::Home), ("End", KeyCode::End),
            ("Delete", KeyCode::Delete), ("Insert", KeyCode::Insert),
            ("F1", KeyCode::F(1)), ("Esc", KeyCode::Esc),
            ("Up", KeyCode::Up), ("PageDown", KeyCode::PageDown),
        ] {
            let mut buf = String::new();
            edit_line(&mut buf, KeyCode::Char('0'), none);
            let changed = edit_line(&mut buf, code, none);
            edit_line(&mut buf, KeyCode::Char('1'), none);
            assert_eq!(buf, "0 1", "{name} between two digits");
            assert!(changed, "{name} is painted, not swallowed");
        }
    }

    /// Issue #51 still holds: an unbound chord is never typed as its
    /// character. Ctrl-L is not the `l` shortcut, and crossterm's report of
    /// Ctrl-\ as the digit `4` with CONTROL set does not pick option 4. Those
    /// chords separate like any other unbound key.
    #[test]
    fn an_unbound_chord_is_a_separator_and_never_its_character() {
        let mut buf = String::from("1");
        edit_line(&mut buf, KeyCode::Char('l'), KeyModifiers::CONTROL);
        assert_eq!(buf, "1 ");
        edit_line(&mut buf, KeyCode::Char('4'), KeyModifiers::CONTROL);
        assert_eq!(buf, "1 ", "a second separator in a row adds nothing");
        edit_line(&mut buf, KeyCode::Char('x'), KeyModifiers::ALT);
        assert_eq!(buf, "1 ");
        assert!(!buf.contains('l') && !buf.contains('4') && !buf.contains('x'));
    }

    /// A separator pressed before anything is typed, or after the number,
    /// changes nothing the reader will see: the line is read trimmed, so
    /// `<Tab> 0 <Tab> <Enter>` is still the answer `0`. A separator is only
    /// ever inserted BETWEEN characters.
    #[test]
    fn a_separator_at_either_end_is_not_kept() {
        let none = KeyModifiers::NONE;
        let mut buf = String::new();
        assert!(!edit_line(&mut buf, KeyCode::Tab, none), "nothing to separate yet");
        assert_eq!(buf, "");
        edit_line(&mut buf, KeyCode::Char('0'), none);
        edit_line(&mut buf, KeyCode::Tab, none);
        assert_eq!(buf.trim(), "0");
    }

    /// The editing keys keep their meaning through the shared helper:
    /// Backspace takes one character (a separator included), Ctrl-U kills
    /// the line (#79), and the return value says whether there is anything
    /// new to paint.
    #[test]
    fn editing_keys_still_edit() {
        let none = KeyModifiers::NONE;
        let mut buf = String::new();
        assert!(!edit_line(&mut buf, KeyCode::Backspace, none), "nothing to erase");
        assert!(edit_line(&mut buf, KeyCode::Char('1'), none));
        assert!(edit_line(&mut buf, KeyCode::Char('2'), none));
        assert!(edit_line(&mut buf, KeyCode::Tab, none));
        assert_eq!(buf, "12 ");
        assert!(edit_line(&mut buf, KeyCode::Backspace, none));
        assert_eq!(buf, "12", "Backspace erases the separator like any character");
        assert!(edit_line(&mut buf, KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(buf, "");
        assert!(!edit_line(&mut buf, KeyCode::Char('u'), KeyModifiers::CONTROL),
            "an already-empty line has nothing to repaint");
    }

    /// The prompt row is drawn inside the middle panel, so it has the
    /// panel's width and not the terminal's. Three hints read 50, 60 and 61

    /// Issue #320: the prompt line has to fit what it is drawn in. The
    /// exile cost used to be asked inside the middle panel, where the hint
    /// plus room to type had to fit 58 columns; it is asked on its own
    /// screen now, whose how-to wraps to the terminal instead of running
    /// off it.
    #[test]
    fn the_set_screens_how_to_wraps_to_its_screen() {
        for width in [58usize, 80, 100, 200] {
            let lines = CliPlayer::wrap_indented(SET_HOW_TO, width);
            assert!(lines.iter().all(|l| str_cols(l) <= width),
                "at {width}: {lines:?}");
            assert_eq!(lines.concat().split_whitespace().collect::<Vec<_>>(),
                SET_HOW_TO.split_whitespace().collect::<Vec<_>>(),
                "and every key survives at {width}");
        }
    }

    /// The bound the prompt reader uses is the panel's, not the terminal's.
    /// They differ by the right gutter plus the borders — 22 columns at 100
    /// — and reading the wrong one is the whole of #320.
    #[test]
    fn the_prompt_bound_is_the_panel_not_the_terminal() {
        // Wherever a CARDS pane exists, the panel's border is strictly
        // inside the terminal, so a prompt that stopped at the terminal's
        // edge had already crossed the border and eaten into that pane.
        for w in [100usize, 120, 160, 200] {
            assert!(CliPlayer::middle_panel_edge_at(w) < w,
                "at {w} columns the panel border is inside the terminal");
        }
        // 100 exactly: 20-column gutters, a 58-column middle panel, its
        // right border in column 79 — 21 columns short of the terminal's.
        assert_eq!(CliPlayer::middle_panel_edge_at(100), 79);
        // Below 100 there is no right pane, and the panel does run to the
        // edge; the bound is then the same one and still correct.
        assert_eq!(CliPlayer::middle_panel_edge_at(80), 80);
    }

    /// The escape and the deliberate empty selection are different keys, and
    /// the empty one is still refused when the cost demands a card.
    #[test]
    fn a_set_prompt_has_a_cancel_and_a_none() {
        // An additional cost can be backed out of; a turn-based action
        // cannot, and there `c` is just a word.
        for word in ["c", "cancel"] {
            assert!(matches!(CliPlayer::parse_card_set_input(word, 3, true), SetInput::Cancel));
            assert!(matches!(CliPlayer::parse_card_set_input(word, 3, false), SetInput::Invalid(_)));
        }
        // Harvest Pyre for X=0: "none" is a mark of its own, so the empty
        // answer is chosen rather than fallen into.
        assert!(matches!(CliPlayer::parse_card_set_input("n", 3, true), SetInput::None));
        assert!(matches!(CliPlayer::parse_card_set_input("none", 3, true), SetInput::None));
        // A fixed-count cost cannot be answered with nothing.
        assert!(CliPlayer::set_count_error(0, 1, 1).contains("exactly 1"));
    }

    /// A target slot with a target already named in front of it says which
    /// one, because that is what decides the rows.
    #[test]
    fn a_target_set_question_names_what_is_already_targeted() {
        // The heading names the action, like every other marking screen,
        // and never carries the question.
        assert_eq!(CliPlayer::target_set_title("Feeling of Dread: choose up to 2 targets"),
            "CHOOSE TARGETS FOR FEELING OF DREAD");
        assert_eq!(CliPlayer::target_set_title("Memory's Journey"),
            "CHOOSE TARGETS FOR MEMORY'S JOURNEY");
        // A bare "up to N": nothing in front, so nothing to say.
        assert_eq!(CliPlayer::target_set_question(&[], 0, 2, 4),
            "Mark up to 2 of the 4 targets below.");
        // Memory's Journey: the player was named first, and the cards are
        // from that player's graveyard.
        assert_eq!(CliPlayer::target_set_question(&["Opponent".into()], 0, 3, 5),
            "Targeting Opponent. Mark up to 3 of the 5 targets below.");
        assert_eq!(CliPlayer::target_set_question(&["You".into(), "Grizzly Bears".into()], 1, 1, 2),
            "Targeting You, Grizzly Bears. Mark 1 of the 2 targets below.");
    }

    /// The refusals a set screen gives, and what each is about.
    #[test]
    fn a_set_prompt_says_why_it_refused() {
        match CliPlayer::parse_card_set_input("x", 3, false) {
            SetInput::Invalid(why) => assert!(why.contains("not a number"), "{why}"),
            other => panic!("got {other:?}"),
        }
        match CliPlayer::parse_card_set_input("5", 3, false) {
            SetInput::Invalid(why) => assert!(why.contains("numbered 0-2"), "{why}"),
            other => panic!("got {other:?}"),
        }
        assert_eq!(CliPlayer::set_count_error(1, 2, 2), "1 marked — mark exactly 2 cards");
        assert_eq!(CliPlayer::set_count_error(4, 0, 3), "4 marked — mark at most 3 cards");
        assert_eq!(CliPlayer::set_count_error(1, 2, 5), "1 marked — mark between 2 and 5 cards");
    }

    /// Issue #288: a bare Enter meant three different things at the three
    /// target choosers — `prompt_target` cancelled, and both "up to N"
    /// prompts CAST. Enter is the reversible key everywhere else in this
    /// CLI (#123), so it cancels at every chooser; stopping early keeps its
    /// own row and its own word.
    #[test]
    fn enter_abandons_the_cast_at_every_target_chooser() {
        for rows in [ChooserRows::CancelOnly, ChooserRows::DoneThenCancel] {
            assert_eq!(CliPlayer::parse_target_input("", 3, rows), TargetInput::Cancel,
                "{rows:?}");
            assert_eq!(CliPlayer::parse_target_input("   ", 3, rows), TargetInput::Cancel,
                "{rows:?}");
            assert_eq!(CliPlayer::parse_target_input("c", 3, rows), TargetInput::Cancel);
            assert_eq!(CliPlayer::parse_target_input("CANCEL", 3, rows), TargetInput::Cancel);
        }
        // Stopping early is still reachable — by its row and by its word.
        assert_eq!(CliPlayer::parse_target_input("3", 3, ChooserRows::DoneThenCancel),
            TargetInput::Done);
        assert_eq!(CliPlayer::parse_target_input("done", 3, ChooserRows::DoneThenCancel),
            TargetInput::Done);
        assert_eq!(CliPlayer::parse_target_input("4", 3, ChooserRows::DoneThenCancel),
            TargetInput::Cancel);
        // ...and is not offered where there is nothing to stop collecting.
        assert_eq!(CliPlayer::parse_target_input("3", 3, ChooserRows::CancelOnly),
            TargetInput::Cancel);
        assert_eq!(CliPlayer::parse_target_input("done", 3, ChooserRows::CancelOnly),
            TargetInput::Invalid);
    }

    /// The targets themselves, the info panes, and everything else.
    #[test]
    fn a_target_chooser_reads_indices_panes_and_nothing_else() {
        let rows = ChooserRows::DoneThenCancel;
        assert_eq!(CliPlayer::parse_target_input("0", 3, rows), TargetInput::Pick(0));
        assert_eq!(CliPlayer::parse_target_input("2", 3, rows), TargetInput::Pick(2));
        for k in ['l', 'g', 'e', 'd', 'i', '/'] {
            assert_eq!(CliPlayer::parse_target_input(&k.to_string(), 3, rows),
                TargetInput::Panel(k));
        }
        for bad in ["x", "-1", "99", "0 1", "back", "escape"] {
            assert_eq!(CliPlayer::parse_target_input(bad, 3, rows), TargetInput::Invalid,
                "{bad}");
        }
    }

    /// Issue #288: the key that abandons a cast has to be advertised on the
    /// screen the player is staring at.
    #[test]
    fn a_target_chooser_advertises_its_exit() {
        let chooser = vec![
            MenuLabel::plain("Ambush Viper 2/1 (opp)"),
            MenuLabel::plain("Cancel the cast"),
        ];
        assert!(CliPlayer::menu_hints(&chooser, true).contains("[enter=cancel]"));
        assert!(CliPlayer::menu_hints(&chooser, false).contains("[enter=cancel]"));

        // The priority menu keeps its own hint, and a menu that merely has a
        // "Cancel cast" ROW (the resolution menu) is not a chooser.
        let priority = vec![MenuLabel::plain("Pass priority"), MenuLabel::plain("Concede")];
        assert!(CliPlayer::menu_hints(&priority, true).contains("[enter=pass]"));
        assert!(!CliPlayer::menu_hints(&priority, true).contains("[enter=cancel]"));
        let resolution = vec![MenuLabel::plain("Pay {1}"), MenuLabel::plain("Cancel cast")];
        assert!(!CliPlayer::menu_hints(&resolution, true).contains("[enter="));
    }

    /// Issue #287: `N` and `N>pwM` draw on two index spaces (CR 508.1a —
    /// each attacker is sent at the defending player or at a planeswalker
    /// that player controls). The refusal names the half that was actually
    /// wrong; a legal creature index is never blamed for a mistyped `pwM`.
    #[test]
    fn a_mistyped_planeswalker_index_is_refused_as_a_planeswalker_index() {
        let msg = CliPlayer::attack_index_error(&[], &[(0, 1)], 2, 1)
            .expect("pw1 is out of range");
        assert!(msg.contains("pw1"), "names the bad planeswalker index: {msg}");
        assert!(msg.contains("pw0"), "points at the one that exists: {msg}");
        assert!(!msg.contains("Invalid attacker"), "attacker 0 was legal: {msg}");
        assert!(!msg.contains("0-1"), "the attacker range is not the answer: {msg}");
    }

    /// The zero-planeswalker case: `0>pw0` where the defender controls none
    /// used to answer "Invalid attacker(s): 0. Valid range is 0-0."
    #[test]
    fn a_walker_attack_with_no_planeswalkers_says_there_are_none() {
        let msg = CliPlayer::attack_index_error(&[], &[(0, 0)], 1, 0)
            .expect("there is no pw0");
        assert!(msg.contains("pw0"), "got {msg}");
        assert!(msg.contains("controls none"), "got {msg}");
        assert!(!msg.contains("Invalid attacker"), "got {msg}");
    }

    /// The half that was already right must not swing the other way.
    #[test]
    fn an_out_of_range_creature_index_is_still_refused_as_an_attacker_index() {
        assert_eq!(CliPlayer::attack_index_error(&[5], &[], 2, 1).as_deref(),
            Some("Invalid attacker(s): 5. Valid range is 0-1."));
        assert_eq!(CliPlayer::attack_index_error(&[], &[(5, 0)], 2, 1).as_deref(),
            Some("Invalid attacker(s): 5. Valid range is 0-1."),
            "a bad creature index inside an N>pwM token is still a creature-index error");
        assert!(CliPlayer::attack_index_error(&[], &[(5, 9)], 2, 1).unwrap()
            .contains("Invalid attacker(s): 5"),
            "with both halves wrong the creature half is named first");
        assert_eq!(CliPlayer::attack_index_error(&[0, 1], &[(0, 0)], 2, 1), None,
            "every index names something on the screen");
    }

    /// Issue #289: `2:0` is a well-formed pair whose blocker index is a live
    /// index in the OTHER list on the same screen. The range guard used to
    /// ride on the parse's match arm, so it was answered by demonstrating
    /// the syntax it had just used correctly.
    #[test]
    fn an_out_of_range_blocker_index_is_refused_by_name_and_range() {
        let e = CliPlayer::parse_block_pair("2:0", 2, 3).unwrap_err();
        assert_eq!(e, "No blocker 2. Your blockers are 0-1.");
        assert!(!e.contains("blocker:attacker"),
            "an in-range-looking pair is not a syntax error: {e}");
    }

    /// And the two lists are not swapped — which is exactly the mistake the
    /// attack prompt made for `pw` indices.
    #[test]
    fn an_out_of_range_attacker_index_names_the_attacker_list() {
        assert_eq!(CliPlayer::parse_block_pair("0:9", 4, 1).unwrap_err(),
            "No attacker 9. Attackers are 0-0.");
        assert_eq!(CliPlayer::parse_block_pair("4:0", 4, 1).unwrap_err(),
            "No blocker 4. Your blockers are 0-3.");
    }

    /// Reclassifying the range failures must not reclassify the real syntax
    /// failures.
    #[test]
    fn a_genuinely_malformed_block_pair_still_gets_the_syntax_message() {
        for p in [":0", "0:", "0:0:0", "abc:0", "-1:0", "0", "::"] {
            assert_eq!(CliPlayer::parse_block_pair(p, 4, 4).unwrap_err(),
                "Invalid. Use 'blocker:attacker' pairs like '0:0 1:1'.",
                "{p} is a syntax error");
        }
    }

    /// The happy path still maps blocker-first, attacker-second, and an
    /// empty combat list never underflows a range message.
    #[test]
    fn a_well_formed_in_range_block_pair_resolves_to_its_two_indices() {
        assert_eq!(CliPlayer::parse_block_pair("1:0", 4, 1), Ok((1, 0)));
        assert_eq!(CliPlayer::parse_block_pair("0:0", 1, 1), Ok((0, 0)));
        assert_eq!(CliPlayer::parse_block_pair("0:0", 0, 0).unwrap_err(),
            "You have no blockers.");
        assert_eq!(CliPlayer::parse_block_pair("0:0", 1, 0).unwrap_err(),
            "There are no attackers.");
    }

    /// The CARDS panel prints its own keyword line and its own flashback
    /// line; the oracle text must not repeat either of them.
    ///
    /// The old filter matched only a line that was exactly one keyword name,
    /// so Elite Inquisitor's "First strike, vigilance" printed under the
    /// generated "First strike, Vigilance", and every flashback card printed
    /// its cost twice within four lines — in a ~22-column panel, where Army
    /// of the Damned's box was entirely consumed by two copies of its
    /// flashback cost plus reminder text (issue #265).
    #[test]
    fn the_cards_panel_does_not_repeat_keywords_or_the_flashback_cost() {
        // A keyword line naming more than one keyword — the Elite Inquisitor
        // case. Dropped; the real rules text is kept.
        let inquisitor = "First strike, vigilance\n\
                          Protection from Vampires, from Werewolves, and from Zombies";
        let kept = CliPlayer::card_panel_oracle_lines(inquisitor, false);
        assert_eq!(kept, vec!["Protection from Vampires, from Werewolves, and from Zombies"],
            "the duplicated keyword line goes, the rules text stays: {kept:?}");

        // The flashback line, with its reminder text — the Army of the Damned
        // case. Dropped only when the panel prints its own flashback line.
        let army = "Create thirteen tapped 2/2 black Zombie creature tokens.\n\
                    Flashback {7}{B}{B}{B} (You may cast this card from your \
                    graveyard for its flashback cost. Then exile it.)";
        let kept = CliPlayer::card_panel_oracle_lines(army, true);
        assert_eq!(kept.len(), 1, "only the real text survives: {kept:?}");
        assert!(kept[0].starts_with("Create thirteen"), "{kept:?}");

        // With no generated flashback line there is nothing to duplicate, so
        // the oracle's own line is the only record and must be kept.
        let kept = CliPlayer::card_panel_oracle_lines(army, false);
        assert_eq!(kept.len(), 2, "nothing is dropped with no flashback line: {kept:?}");

        // The single-keyword case the old filter did handle still works, in
        // both the bare and trailing-comma forms.
        for line in ["Flying", "flying", "Flying,", "Deathtouch"] {
            assert!(CliPlayer::card_panel_oracle_lines(line, false).is_empty(),
                "{line:?} is already on the keyword line");
        }

        // A line that merely mentions a keyword is rules text, not a repeat.
        let mentions = "Target creature gains flying until end of turn.";
        assert_eq!(CliPlayer::card_panel_oracle_lines(mentions, false), vec![mentions],
            "a sentence about a keyword is not a keyword line");
        let flashback_prose = "Whenever you cast a spell with flashback, draw a card.";
        assert_eq!(CliPlayer::card_panel_oracle_lines(flashback_prose, true), vec![flashback_prose],
            "a sentence mentioning flashback is not the flashback line");
    }

    fn view(step: Step, turn_number: u32, our_turn: bool) -> GameView {
        let you = PlayerId(0);
        GameView {
            you,
            your_hand: vec![],
            your_life: 20,
            your_mana_pool: ManaPool::new(),
            your_library_size: 40,
            your_library_cards: vec![],
            your_mulligan_count: 0,
            opponents: vec![],
            battlefield: vec![],
            graveyards: vec![],
            stack: vec![],
            exile: vec![],
            first_strike_damage_step: false,
            step,
            active_player: if our_turn { you } else { PlayerId(1) },
            priority_player: Some(you),
            turn_number,
            display_log: vec![],
            full_log: vec![],
            revealed_names: HashMap::new(),
        }
    }

    fn legal(actions: Vec<Action>) -> LegalActions {
        LegalActions {
            actions,
            combat_prompt: None,
            castable_spells: vec![],
            activatable_abilities: vec![],
            context: None,
            resolution_prompt: None,
            set_prompt: None,
        }
    }

    fn cast(id: u64) -> Action {
        Action::CastSpell {
            object_id: ObjectId(id),
            targets: vec![],
            sacrifice: None,
            exile_count: None,
            exile_ids: vec![],
            alternative_cost: None,
            tap_plan: vec![],
        }
    }

    fn pass_concede_plus(mut extra: Vec<Action>) -> LegalActions {
        let mut actions = vec![Action::PassPriority, Action::Concede];
        actions.append(&mut extra);
        legal(actions)
    }

    /// `creature`, declared as an attacker.
    fn attacker(id: u64, name: &str, controller: u8) -> mtg_engine::view::PermanentView {
        let mut c = creature(id, name, controller);
        c.attacking = Some(mtg_engine::view::AttackTarget::Player(PlayerId(0)));
        c
    }

    /// Issue #255: paging was forward-only, so overshooting a 253-name list
    /// meant pressing `m` eleven more times to wrap back around to it.
    #[test]
    fn a_menu_pages_backwards_too() {
        // Ten one-line items in a five-line pane: four to a page once the
        // marker has its line.
        let ten = [1usize; 10];
        assert_eq!(CliPlayer::prev_menu_offset_lines(&ten, 5, 4, 1), 0);
        assert_eq!(CliPlayer::prev_menu_offset_lines(&ten, 5, 8, 1), 4);
        // From the top, back to the last page — the last four rows.
        assert_eq!(CliPlayer::prev_menu_offset_lines(&ten, 5, 0, 1), 6);
        // A menu that fits has one page, and `p` stays on it.
        assert_eq!(CliPlayer::prev_menu_offset_lines(&[1; 4], 5, 0, 1), 0);
        assert_eq!(CliPlayer::prev_menu_offset_lines(&[], 5, 0, 1), 0);
        // Round trip: forwards then backwards is where you started.
        let mut off = 0;
        for _ in 0..2 { off = CliPlayer::next_menu_offset(off, 4, 10); }
        assert_eq!(off, 8);
        for _ in 0..2 { off = CliPlayer::prev_menu_offset_lines(&ten, 5, off, 1); }
        assert_eq!(off, 0);
    }


    /// Issue #260: the truncation marker is the LAST row a short pane
    /// sacrifices, not the first — a menu that does not fit has to say so,
    /// or the pane reads as a game that has stopped asking (#76).
    #[test]
    fn a_menu_too_tall_for_the_pane_still_says_so() {
        // Two rows for a 32-entry menu: one option and the marker.
        let (offset, shown, paged) = CliPlayer::menu_page(32, 2, 0);
        assert_eq!((offset, shown, paged), (0, 1, true),
            "one option and a row left for the marker");

        // One row is not enough for both, and the marker is what survives —
        // the caller draws it after the options, guarded on the pane bottom.
        let (_, shown, paged) = CliPlayer::menu_page(32, 1, 0);
        assert!(paged, "still paged");
        assert_eq!(shown, 1);

        // A menu that fits keeps every row and draws no marker.
        assert_eq!(CliPlayer::menu_page(2, 20, 0), (0, 2, false));
    }

    /// Issue #259: X is announced as the spell is cast (CR 601.2b) and the
    /// stack is a public zone (CR 400.2), so both seats are entitled to it.
    /// A Devil's Play for 12 and one for 0 used to be character-for-character
    /// identical on screen.
    #[test]
    fn a_stack_entry_shows_its_announced_x() {
        let v = view(Step::PrecombatMain, 15, true);
        let mut item = mtg_engine::view::StackItemView {
            object_id: ObjectId(22),
            card_id: mtg_engine::ids::CardId(0),
            name: "Devil's Play".to_string(),
            controller: PlayerId(0),
            targets: vec![],
            x_value: Some(3),
        };
        assert_eq!(CliPlayer::stack_entry_headline(&v, &item), "Devil's Play (X=3) (you)");
        item.x_value = Some(0);
        assert_eq!(CliPlayer::stack_entry_headline(&v, &item), "Devil's Play (X=0) (you)");

        // A spell without an X says nothing about one.
        item.x_value = None;
        item.name = "Geistflame".to_string();
        item.controller = PlayerId(1);
        assert_eq!(CliPlayer::stack_entry_headline(&v, &item), "Geistflame (opp)");
    }

    /// Issue #295: the declare-attackers stop tested whether the opponent
    /// merely CONTROLLED a creature, so auto-pass stopped at a combat where
    /// they had declared no attackers at all, on a menu offering pass, a
    /// mana ability and concede.
    #[test]
    fn an_opponents_combat_with_no_attack_is_not_a_stop() {
        let mode = CliPlayer::new_pass_mode(&view(Step::Upkeep, 4, false));
        let mut v = view(Step::DeclareAttackers, 4, false);
        v.battlefield = vec![creature(9, "Bear", 1)];
        assert_eq!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![]), &mode), None,
            "they control a creature but declared no attackers");

        v.battlefield = vec![attacker(9, "Bear", 1)];
        assert_eq!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![]), &mode),
            Some(BreakReason::Attackers), "an actual attack is a stop");
    }

    /// Issue #295: the postcombat-main stop's own rationale is a your-turn
    /// one ("removal on damaged creatures"), but it had no player test — so
    /// auto-pass engaged on the opponent's turn died at THEIR main phase 2,
    /// a phase and a turn short of where it was going, and then described it
    /// as "your postcombat main phase".
    #[test]
    fn the_postcombat_main_stop_is_your_own() {
        let mode = CliPlayer::new_pass_mode(&view(Step::Upkeep, 4, false));
        let theirs = view(Step::PostcombatMain, 4, false);
        assert_eq!(CliPlayer::should_break_pass(&theirs, &pass_concede_plus(vec![]), &mode), None,
            "the opponent's postcombat main is not a stop");

        let mode = CliPlayer::new_pass_mode(&view(Step::Upkeep, 6, true));
        let ours = view(Step::PostcombatMain, 6, true);
        assert_eq!(CliPlayer::should_break_pass(&ours, &pass_concede_plus(vec![]), &mode),
            Some(BreakReason::YourPostcombatMain));
    }

    /// Issue #294: the refusal used to be reconstructed by hunting the menu
    /// for the first actionable-looking row, so a prompt blocked by the
    /// postcombat-main stop was reported as having "a castable spell it would
    /// skip". Engaging and refusing now come from one predicate, so the
    /// reason is the one that actually blocked it.
    #[test]
    fn a_refusal_names_the_clause_that_blocked_it() {
        // Postcombat main with a castable spell: the stop is the phase, not
        // the spell. The message used to hunt the menu for the first
        // actionable-looking row and blame that instead (#294), and with an
        // empty hand it blamed a spell that was not there at all (#131).
        let v = view(Step::PostcombatMain, 6, true);
        for l in [pass_concede_plus(vec![cast(4)]), pass_concede_plus(vec![])] {
            let reason = CliPlayer::try_engage_auto_pass(&v, &l).err().expect("refused");
            assert_eq!(reason, BreakReason::YourPostcombatMain);
            assert!(!reason.describe().contains("spell"), "{}", reason.describe());
        }

        // A land play is its own stop, whatever else is on the menu (#39).
        let l = pass_concede_plus(vec![Action::PlayLand { object_id: ObjectId(3) }, cast(4)]);
        assert_eq!(CliPlayer::try_engage_auto_pass(&v, &l).err(), Some(BreakReason::LandPlay));

        // And an ordinary window with nothing to skip engages.
        let quiet = view(Step::EndStep, 6, false);
        assert!(CliPlayer::try_engage_auto_pass(&quiet, &pass_concede_plus(vec![])).is_ok());
    }

    // Issue #45: 'f' pressed on our own turn before our main phase must
    // still break at THIS turn's Main Phase 1 when a spell is castable
    // there — "next Main Phase 1" is this turn's, not next turn's.
    #[test]
    fn same_turn_main_phase_castable_spell_breaks_pass() {
        // f pressed at our Draw step of turn 9.
        let mode = CliPlayer::new_pass_mode(&view(Step::Draw, 9, true));
        // Reaching our own Main Phase 1 of the same turn with a castable
        // spell (and no land to play) must prompt.
        let v = view(Step::PrecombatMain, 9, true);
        assert_eq!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![cast(1)]), &mode),
            Some(BreakReason::TargetMainPhase));
    }

    // Issue #45 companion: even with nothing castable, our own Main
    // Phase 1 of the press turn is "our next Main Phase 1" — stop there.
    #[test]
    fn same_turn_main_phase_breaks_pass_when_pressed_before_main() {
        let mode = CliPlayer::new_pass_mode(&view(Step::Upkeep, 9, true));
        let v = view(Step::PrecombatMain, 9, true);
        assert_eq!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![]), &mode),
            Some(BreakReason::TargetMainPhase));
    }

    // 'f' pressed AT our Main Phase 1 is a deliberate skip of the rest of
    // this turn: the same turn's later steps must not re-break for spells.
    #[test]
    fn press_at_main_phase_still_skips_rest_of_turn() {
        let mode = CliPlayer::new_pass_mode(&view(Step::PrecombatMain, 6, true));
        let v = view(Step::EndStep, 6, true);
        assert_eq!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![cast(1)]), &mode), None);
        // ...but next turn's upkeep with a castable spell breaks, as before.
        let v = view(Step::Upkeep, 7, true);
        assert_eq!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![cast(1)]), &mode),
            Some(BreakReason::MeaningfulAction));
    }

    // Issue #48: pressing 'f' on a prompt that already offers a land play
    // (or a castable spell alongside it) must not engage-and-pass — that
    // would silently discard the land drop before any break check runs.
    #[test]
    fn press_with_land_play_on_offer_refuses_to_engage() {
        let v = view(Step::PrecombatMain, 6, true);
        let l = pass_concede_plus(vec![
            Action::PlayLand { object_id: ObjectId(3) },
            cast(4),
        ]);
        assert_eq!(CliPlayer::try_engage_auto_pass(&v, &l).err(), Some(BreakReason::LandPlay));
    }

    // Issue #48 companion: with no land play on offer, 'f' at our own Main
    // Phase 1 is a deliberate skip and must still engage.
    #[test]
    fn press_at_own_main_without_land_engages() {
        let v = view(Step::PrecombatMain, 6, true);
        let l = pass_concede_plus(vec![cast(4)]);
        assert!(CliPlayer::try_engage_auto_pass(&v, &l).is_ok());
    }

    // Issue #109: echo and filter displays clip by display COLUMNS, so a
    // wide-character (CJK) filter can't take twice its char count in cells
    // and wrap over neighbouring panels.
    #[test]
    fn clipping_counts_display_columns_not_chars() {
        assert_eq!(col_width('a'), 1);
        assert_eq!(col_width('稲'), 2);
        // 5 wide chars = 10 columns; a 7-column clip keeps only 3 chars (6 cols).
        assert_eq!(clip_cols("稲妻稲妻稲", 7), "稲妻稲");
        assert_eq!(str_cols("稲妻稲"), 6);
        // ASCII behaves like a char clip.
        assert_eq!(clip_cols("abcdef", 4), "abcd");
        assert_eq!(clip_cols("abc", 10), "abc");
        assert_eq!(clip_cols("", 5), "");
    }

    // Issues #101/#102: the info views clamp to the terminal height and
    // page instead of silently truncating (l) or scrolling off the top
    // (g/e/d). page_window is the shared arithmetic.
    #[test]
    fn page_window_clamps_and_pages() {
        // 1086 log entries on a 50-row terminal: 46 visible per page.
        let (start, end, size) = CliPlayer::page_window(1086, 50, 0);
        assert_eq!((start, end, size), (0, 46, 46));
        // The last page holds the remainder, not a full page.
        let last_page = (1086 - 1) / 46;
        let (start, end, _) = CliPlayer::page_window(1086, 50, last_page);
        assert_eq!(end, 1086);
        assert!(end - start <= 46 && start < end);
        // A page past the end clamps to the last page.
        let (s2, e2, _) = CliPlayer::page_window(1086, 50, last_page + 7);
        assert_eq!((s2, e2), (start, end));
        // Shorter than a page: everything visible, no paging needed.
        assert_eq!(CliPlayer::page_window(10, 24, 0), (0, 10, 20));
        // Degenerate terminal heights never yield a zero page size.
        assert_eq!(CliPlayer::page_window(5, 3, 0).2, 1);
        // Empty list stays empty without panicking.
        assert_eq!(CliPlayer::page_window(0, 24, 0), (0, 0, 20));
    }

    // Issue #100: land targets carry the same (your)/(opp) marker as every
    // other permanent — without it, "Destroy target land" offered your own
    // and the opponent's Islands as byte-identical menu lines.
    #[test]
    fn land_labels_carry_the_controller_marker() {
        use mtg_engine::view::PermanentView;
        let land = |id: u64, controller: u8| PermanentView {
            object_id: ObjectId(id),
            card_id: mtg_engine::ids::CardId(0),
            name: "Island".into(),
            supertypes: vec![],
            card_types: vec![CardType::Land],
            controller: PlayerId(controller),
            owner: PlayerId(controller),
            tapped: false,
            power: None,
            toughness: None,
            effective_power: None,
            effective_toughness: None,
            damage_marked: 0,
            summoning_sick: false,
            attached_to: None,
            attached_to_player: None,
            keywords: vec![],
            subtypes: vec![],
            printed_power: None,
            printed_toughness: None,
            star_pt: false,
            is_token: false,
            protections: vec![],
            attacking: None,
            blocking: vec![],
            blocked_by: vec![],
            oracle_text: String::new(),
            counters: HashMap::new(),
            loyalty_abilities: vec![],
            mana_abilities: vec![],
            named_card: None,
        };
        let mut v = view(Step::PrecombatMain, 5, true);
        v.battlefield = vec![land(10, 0), land(11, 1)];
        let yours = CliPlayer::perm_name(&v, ObjectId(10));
        let theirs = CliPlayer::perm_name(&v, ObjectId(11));
        assert_eq!(yours, "Island (your)");
        assert_eq!(theirs, "Island (opp)");
        assert_ne!(yours, theirs, "identical lands must be distinguishable");
    }

    /// Exile is a public zone, and the one the name resolver did not look
    /// in. Runic Repetition is the only card in the pool whose targets live
    /// there, so the omission made it unplayable by sight: the menu row read
    /// "targeting obj#17", the chooser listed "0: obj#24 / 1: obj#25", and no
    /// pane anywhere shows object ids (issue #332).
    #[test]
    fn a_card_in_exile_is_named_the_way_one_in_a_graveyard_is() {
        let card = |id: u64, name: &str| mtg_engine::view::CardView {
            object_id: ObjectId(id),
            card_id: mtg_engine::ids::CardId(0),
            name: name.into(),
            cost: None,
            supertypes: vec![],
            card_types: vec![CardType::Instant],
            power: None,
            toughness: None,
            oracle_text: String::new(),
            owner: PlayerId(0),
            flashback_cost: None,
        };
        let mut v = view(Step::PrecombatMain, 5, true);
        v.exile = vec![card(24, "Dream Twist"), card(25, "Think Twice")];
        v.graveyards = vec![(PlayerId(0), vec![card(30, "Armored Skaab")])];

        assert_eq!(CliPlayer::perm_name(&v, ObjectId(24)), "Dream Twist");
        assert_eq!(CliPlayer::perm_name(&v, ObjectId(25)), "Think Twice");
        assert_eq!(CliPlayer::perm_name(&v, ObjectId(30)), "Armored Skaab",
            "the zone next door, which already worked");
        // An id in no visible zone still falls back, so the resolver has not
        // started inventing names.
        assert_eq!(CliPlayer::perm_name(&v, ObjectId(99)), format!("{}", ObjectId(99)));
    }

    /// A creature for the combat-list tests.
    fn creature(id: u64, name: &str, controller: u8) -> mtg_engine::view::PermanentView {
        mtg_engine::view::PermanentView {
            object_id: ObjectId(id),
            card_id: mtg_engine::ids::CardId(0),
            name: name.into(),
            supertypes: vec![],
            card_types: vec![CardType::Creature],
            controller: PlayerId(controller),
            owner: PlayerId(controller),
            tapped: false,
            power: Some(2),
            toughness: Some(2),
            effective_power: Some(2),
            effective_toughness: Some(2),
            damage_marked: 0,
            summoning_sick: false,
            attached_to: None,
            attached_to_player: None,
            keywords: vec![],
            subtypes: vec![],
            printed_power: None,
            printed_toughness: None,
            star_pt: false,
            is_token: false,
            protections: vec![],
            attacking: None,
            blocking: vec![],
            blocked_by: vec![],
            oracle_text: String::new(),
            counters: HashMap::new(),
            loyalty_abilities: vec![],
            mana_abilities: vec![],
            named_card: None,
        }
    }

    /// Issue #268: two same-named attackers that differ only in marked
    /// damage are the case where the block decision turns on the difference,
    /// and the list rendered them identically.
    #[test]
    fn a_combat_entry_shows_marked_damage_and_disambiguates() {
        let mut hurt = creature(63, "Crossway Vampire", 1);
        hurt.damage_marked = 1;
        let fresh = creature(64, "Crossway Vampire", 1);
        let mut v = view(Step::DeclareBlockers, 18, false);
        v.battlefield = vec![hurt, fresh];
        let ids = vec![ObjectId(63), ObjectId(64)];

        let a = CliPlayer::combat_entry(&v, ObjectId(63), &ids);
        let b = CliPlayer::combat_entry(&v, ObjectId(64), &ids);
        assert!(a.contains("(1d)"), "the damaged one says so: {a}");
        assert_ne!(a, b, "the defender must be able to tell them apart");
    }

    /// Issue #268/#136: two identical creatures with nothing to tell them
    /// apart get the object id, the way the target pickers already do.
    #[test]
    fn identical_combat_entries_fall_back_to_the_object_id() {
        let mut v = view(Step::DeclareBlockers, 18, false);
        v.battlefield = vec![creature(70, "Walking Corpse", 1), creature(71, "Walking Corpse", 1)];
        let ids = vec![ObjectId(70), ObjectId(71)];

        assert!(CliPlayer::combat_entry(&v, ObjectId(70), &ids).contains("(#70)"));
        assert!(CliPlayer::combat_entry(&v, ObjectId(71), &ids).contains("(#71)"));
    }

    /// Issue #219: an attacker aimed at a planeswalker rendered exactly like
    /// one aimed at the player, and the defender was asked to block blind.
    #[test]
    fn a_combat_entry_names_the_planeswalker_being_attacked() {
        let mut walker = creature(51, "Liliana of the Veil", 0);
        walker.card_types = vec![CardType::Planeswalker];
        walker.counters.insert(mtg_engine::types::CounterType::Loyalty, 4);
        let mut at_walker = creature(18, "Terror of Kruin Pass", 1);
        at_walker.attacking = Some(mtg_engine::view::AttackTarget::Planeswalker(ObjectId(51)));
        let mut at_player = creature(19, "Terror of Kruin Pass", 1);
        at_player.attacking = Some(mtg_engine::view::AttackTarget::Player(PlayerId(0)));
        let mut v = view(Step::DeclareBlockers, 25, false);
        v.battlefield = vec![walker, at_walker, at_player];
        let ids = vec![ObjectId(18), ObjectId(19)];

        let on_walker = CliPlayer::combat_entry(&v, ObjectId(18), &ids);
        let on_player = CliPlayer::combat_entry(&v, ObjectId(19), &ids);
        assert!(on_walker.contains("Liliana of the Veil [4 loyalty]"), "got {on_walker}");
        assert!(!on_player.contains("Liliana"), "got {on_player}");
    }

    /// Issue #328: the combat lists were emitted at their natural length
    /// while the battlefield rows beside them were already clamped (#244).
    /// An Elite Inquisitor's ability list is ~100 columns on its own, so its
    /// row ran through the pane's right border and overwrote the CARDS pane
    /// on exactly the line the defender reads to choose a block.
    ///
    /// Nothing gives way (#318): the row wraps under its index, every line
    /// fits the pane, and the note the caller paints in colour rides on the
    /// last line when it fits there and on a line of its own when it does
    /// not.
    #[test]
    fn a_combat_row_wraps_to_the_pane_it_is_drawn_in() {

        let mut inq = creature(90, "Elite Inquisitor", 0);
        inq.keywords = vec![mtg_engine::types::Keyword::FirstStrike,
                            mtg_engine::types::Keyword::Vigilance];
        inq.protections = vec!["protection from Vampires".into(),
                               "protection from Werewolves".into(),
                               "protection from Zombies".into()];
        let mut v = view(Step::DeclareBlockers, 7, true);
        v.battlefield = vec![inq];
        let ids = vec![ObjectId(90)];
        // 100 columns: the narrowest width at which the CARDS pane exists,
        // and the one the issue's capture was taken at.
        let panel = CliPlayer::middle_panel_width_at(100);

        let unclamped = CliPlayer::combat_entry(&v, ObjectId(90), &ids);
        assert!(unclamped.chars().count() > panel,
            "test precondition: the entry is wider than the pane ({} > {panel})",
            unclamped.chars().count());

        let prefix_w = str_cols("  0: ");
        let note = " (can block: 0)";
        let row = CliPlayer::combat_row_layout(&v, ObjectId(90), &ids, prefix_w, note, panel);
        assert!(row.lines.len() >= 2, "the entry wraps: {row:?}");
        for line in row.lines.iter().chain(&row.note_lines) {
            assert!(prefix_w + str_cols(line) <= panel,
                "every line fits the {panel}-column pane: {line:?}");
        }
        assert_eq!(row.text(), format!("{unclamped}{note}"), "nothing is cut");
        assert!(row.lines[0].starts_with("Elite Inquisitor"), "{row:?}");

        // A note that fits after the last line stays there; one that does
        // not gets a line of its own rather than pushing through the border.
        let short = CliPlayer::combat_row_layout(&v, ObjectId(90), &ids, prefix_w, "", panel);
        assert!(short.note_lines.is_empty() && short.note.is_empty());
        let wide_note = format!(" (can block: {})", (0..30).map(|i| i.to_string()).collect::<Vec<_>>().join(" "));
        let long = CliPlayer::combat_row_layout(&v, ObjectId(90), &ids, prefix_w, &wide_note, panel);
        assert!(!long.note_lines.is_empty(), "{long:?}");
        assert_eq!(long.height(), long.lines.len() + long.note_lines.len());
        assert_eq!(long.text(), format!("{unclamped}{wide_note}"));
    }

    /// The row keeps the tail a block decision cannot do without — which
    /// planeswalker an attacker is aimed at (CR 508.1a) and the id that
    /// tells two identical rows apart (#136) — and, now that nothing is
    /// cut, the ability list too.
    #[test]
    fn a_wrapped_combat_row_keeps_the_attack_target_and_the_id() {

        let mut walker = creature(51, "Liliana of the Veil", 0);
        walker.card_types = vec![CardType::Planeswalker];
        walker.counters.insert(mtg_engine::types::CounterType::Loyalty, 4);
        let long = |id: u64| {
            let mut c = creature(id, "Terror of Kruin Pass", 1);
            c.keywords = vec![mtg_engine::types::Keyword::FirstStrike,
                              mtg_engine::types::Keyword::Vigilance,
                              mtg_engine::types::Keyword::Trample];
            c.protections = vec!["protection from Vampires".into(),
                                 "protection from Werewolves".into()];
            c.attacking = Some(mtg_engine::view::AttackTarget::Planeswalker(ObjectId(51)));
            c
        };
        let mut v = view(Step::DeclareBlockers, 25, false);
        v.battlefield = vec![walker, long(18), long(19)];
        let ids = vec![ObjectId(18), ObjectId(19)];

        let panel = CliPlayer::middle_panel_width_at(100);
        let row = CliPlayer::combat_row_layout(&v, ObjectId(18), &ids, str_cols("  0: "), "", panel);
        let text = row.text();
        assert!(text.starts_with("Terror of Kruin Pass"), "the creature is named: {text}");
        assert!(text.ends_with("(#18)"),
            "and the id that tells the two apart is on the end: {text}");
        assert!(text.contains("Liliana of the Veil [4 loyalty]"),
            "so is the attack target, whole: {text}");
        assert!(text.contains("vigilance"), "and so is the ability list: {text}");
        for line in &row.lines {
            assert!(str_cols("  0: ") + str_cols(line) <= panel, "a line overflows: {line:?}");
        }
    }


    /// Issue #243: the keywords the block turns on are on the line the block
    /// is chosen from.
    #[test]
    fn a_combat_entry_carries_the_live_keywords() {
        let mut flier = creature(101, "Spirit", 1);
        flier.keywords = vec![mtg_engine::types::Keyword::Flying];
        let mut v = view(Step::DeclareBlockers, 7, false);
        v.battlefield = vec![flier];

        let entry = CliPlayer::combat_entry(&v, ObjectId(101), &[ObjectId(101)]);
        assert!(entry.contains("flying"), "got {entry}");
    }

    /// Issue #221: `[S]` is a creature restriction (CR 302.6). It means
    /// nothing on a planeswalker or an enchantment, and nothing on a hasty
    /// creature (#139).
    #[test]
    fn summoning_sickness_is_only_asked_about_creatures() {
        let mut walker = creature(30, "Liliana of the Veil", 0);
        walker.card_types = vec![CardType::Planeswalker];
        walker.summoning_sick = true;
        assert!(!CliPlayer::is_summoning_sick(&walker));

        let mut hasty = creature(31, "Hasty Thing", 0);
        hasty.summoning_sick = true;
        hasty.keywords = vec![mtg_engine::types::Keyword::Haste];
        assert!(!CliPlayer::is_summoning_sick(&hasty));

        let mut sick = creature(32, "Fresh Thing", 0);
        sick.summoning_sick = true;
        assert!(CliPlayer::is_summoning_sick(&sick));
    }

    /// Issue #270: the tap / sickness / damage flags are what the row is read
    /// for, so a long attachment list is what gets elided, not them.
    #[test]
    fn a_long_attachment_list_never_pushes_the_flags_off_the_row() {
        let row = CliPlayer::elide_middle(
            "Galvanic Juggernaut 8/7",
            " [Silver-Inlaid Dagger,Silver-Inlaid Dagger,Butcher's Cleaver,Mask of Avacyn]",
            " [T] (3d)",
            60,
        );
        assert!(row.ends_with(" [T] (3d)"), "got {row}");
        assert!(row.contains('…'), "the attachment list is what shortens: {row}");
        assert!(row.chars().count() <= 60);
    }

    // Issue #39 guard: a land play breaks auto-pass on any turn, even the
    // press turn, whatever step the press happened at.
    #[test]
    fn land_play_always_breaks_pass() {
        let mode = CliPlayer::new_pass_mode(&view(Step::PrecombatMain, 6, true));
        let v = view(Step::PostcombatMain, 6, true);
        let l = pass_concede_plus(vec![Action::PlayLand { object_id: ObjectId(3) }]);
        assert_eq!(CliPlayer::should_break_pass(&v, &l, &mode), Some(BreakReason::LandPlay));
    }
}

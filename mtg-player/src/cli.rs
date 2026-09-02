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
        let seq = b"\x1b[?2004l\x1b[?25h\r\n";
        libc::write(libc::STDOUT_FILENO, seq.as_ptr().cast(), seq.len());
        libc::_exit(128 + sig);
    }
}

/// Install SIGHUP/SIGTERM/SIGINT handlers that restore the terminal
/// before exiting (issue #78). A signal landing while a prompt held the
/// terminal in raw mode used to leave the pty raw for the inheriting
/// shell — no echo, no line editing, no Ctrl-C, staircased output — on
/// the ordinary close-the-window (SIGHUP) and `kill`/`timeout` (SIGTERM)
/// paths, forcing a blind `stty sane`. The runner calls this once, before
/// the first prompt, when a human CLI seat exists.
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
    }
}

/// Restore the terminal for normal line-oriented output after the TUI:
/// leave raw mode, clear the last rendered frame, and home the cursor.
/// The runner calls this before printing the end-of-game summary so the
/// summary doesn't land on top of a stale frame and visually merge with
/// leftover rows (issue #47).
pub fn reset_terminal_for_exit() {
    let _ = terminal::disable_raw_mode();
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

/// One round of an "up to N targets" prompt (see `prompt_target_up_to`).
enum UpToPick {
    Pick(mtg_engine::actions::Target),
    Done,
    Cancel,
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

pub struct CliPlayer {
    name: String,
    /// When set, auto-pass priority until the specified condition.
    pass_mode: Option<PassMode>,
    /// Filter string for the card reference panel.
    card_filter: String,
}

impl CliPlayer {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pass_mode: None,
            card_filter: String::new(),
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
        let _ = terminal::enable_raw_mode();
        while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            let _ = event::read();
        }
        if !was_raw {
            let _ = terminal::disable_raw_mode();
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
    /// engage, or None when the current prompt already meets the break
    /// condition — passing would silently discard actions (a land play, a
    /// castable spell) that auto-pass promises never to skip (issue #48).
    fn try_engage_auto_pass(
        view: &GameView,
        legal: &mtg_engine::engine::LegalActions,
    ) -> Option<PassMode> {
        let mode = Self::new_pass_mode(view);
        if Self::should_break_pass(view, legal, &mode) { None } else { Some(mode) }
    }

    /// Check whether the current pass mode should break and return control
    /// to the player. Returns true if the player should be prompted.
    fn should_break_pass(
        view: &GameView,
        legal: &mtg_engine::engine::LegalActions,
        mode: &PassMode,
    ) -> bool {
        match mode {
            PassMode::UntilNextTurn { activated_turn, before_our_main } => {
                // "Our next Main Phase 1" is this turn's when 'f' was pressed
                // before it, and a later turn's otherwise (issue #45 — the
                // spell/ability clauses below used to require a strictly
                // later turn, silently skipping a same-turn castable spell).
                let reached_target_turn = view.turn_number > *activated_turn
                    || (*before_our_main && view.turn_number == *activated_turn);

                // A land drop is never auto-passed, whatever the turn: once
                // per turn and free, a land play is always worth stopping
                // for (issue #39).
                if legal.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })) {
                    return true;
                }

                // Break at our precombat main once we reach the target turn.
                if view.active_player == view.you
                    && reached_target_turn
                    && view.step == Step::PrecombatMain
                {
                    return true;
                }

                // Break on our turn if we have meaningful actions (cast spells,
                // play lands, activate non-mana abilities) — even outside main phase.
                if view.active_player == view.you
                    && reached_target_turn
                {
                    let has_meaningful = legal.actions.iter().any(|a| matches!(a,
                        Action::PlayLand { .. }
                        | Action::CastSpell { .. }
                        | Action::ActivateAbility { .. }
                    ));
                    if has_meaningful {
                        return true;
                    }
                }

                // Break if something is on the stack AND we have a meaningful
                // response (not just pass/concede/mana abilities).
                if !view.stack.is_empty() {
                    let has_response = legal.actions.iter().any(|a| !matches!(a,
                        Action::PassPriority | Action::Concede | Action::ActivateManaAbility { .. }
                    ));
                    if has_response {
                        return true;
                    }
                }

                // Break at opponent's DeclareAttackers only if they have creatures
                // that could be attacking.
                if view.active_player != view.you
                    && view.step == Step::DeclareAttackers
                {
                    let opp_has_creatures = view.battlefield.iter().any(|p| {
                        p.controller != view.you
                            && p.card_types.contains(&CardType::Creature)
                    });
                    if opp_has_creatures {
                        return true;
                    }
                }

                // Break after combat ends (post-combat main on either turn)
                // so the player can use removal/burn on damaged creatures.
                if view.step == Step::PostcombatMain {
                    return true;
                }

                false
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

    fn render(view: &GameView, actions: Option<&[String]>, message: Option<&str>, log: &[String], card_filter: &str, pass_mode_label: Option<&str>) {
        let _ = Self::render_paged(view, actions, message, log, card_filter, pass_mode_label, 0);
    }

    /// `render`, starting the action menu at `menu_offset` (issue #96 — a
    /// menu longer than the pane is paged with 'm', not guessed at).
    /// Returns how many menu entries were shown from that offset.
    fn render_paged(view: &GameView, actions: Option<&[String]>, message: Option<&str>, log: &[String], card_filter: &str, pass_mode_label: Option<&str>, menu_offset: usize) -> usize {
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
            let mut srow: u16 = 1;
            for item in &view.stack {
                if srow >= u16::try_from(stack_h).unwrap_or(u16::MAX) { break; }
                let who = if item.controller == view.you { "you" } else { "opp" };
                let text = format!("{} ({})", item.name, who);
                // Wrap if too long for panel.
                let max_w = left_w.saturating_sub(1);
                for line in Self::word_wrap(&text, max_w) {
                    if srow as usize >= stack_h { break; }
                    let _ = execute!(out, cursor::MoveTo(1, srow), Print(&line));
                    srow += 1;
                }
                for target in &item.targets {
                    if srow >= u16::try_from(stack_h).unwrap_or(u16::MAX) { break; }
                    let target_name = match target {
                        mtg_engine::actions::Target::Object(id) => {
                            // perm_name carries the (your)/(opp) marker and
                            // resolves non-battlefield objects too (#100).
                            format!(" -> {}", Self::perm_name(view, *id))
                        }
                        mtg_engine::actions::Target::Player(pid) => {
                            if *pid == view.you { " -> you".into() } else { " -> opp".into() }
                        }
                        mtg_engine::actions::Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
                    };
                    let truncated: String = target_name.chars().take(left_w.saturating_sub(1)).collect();
                    let _ = execute!(out, cursor::MoveTo(1, srow),
                        SetAttribute(Attribute::Dim), Print(&truncated), SetAttribute(Attribute::Reset));
                    srow += 1;
                }
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
            Step::CombatDamage => "Combat Damage",
            Step::EndCombat => "End Combat",
            Step::PostcombatMain => "Main Phase 2",
            Step::EndStep => "End Step",
            Step::Cleanup => "Cleanup (Discard to 7)",
        };

        // Turn/phase bar
        let whose_turn = if view.active_player == view.you { "Your turn" } else { "Opponent's turn" };
        let pass_label = pass_mode_label.map(|l| format!(" [{l}]")).unwrap_or_default();
        let status = format!(" Turn {} - {} | {}", view.turn_number, step_name, whose_turn);
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

        // Actions separator with optional label (always drawn, wraps if needed)
        if let Some(msg) = message {
            let prefix = "─── ";
            let indent = "    ";
            let prefix_len = prefix.chars().count(); // 4
            // Leave room for prefix/indent + trailing space + at least 1 dash
            let text_w = mid_w.saturating_sub(prefix_len + 2);
            let wrapped = Self::word_wrap(msg, text_w);
            for (i, line) in wrapped.iter().enumerate() {
                let leader = if i == 0 { prefix } else { indent };
                let label = format!("{leader}{line} ");
                let full = format!("{}{}", label, "─".repeat(mid_w.saturating_sub(label.chars().count())));
                let _ = execute!(out, cursor::MoveTo(mid_col, row),
                    SetAttribute(Attribute::Dim), Print(&full), SetAttribute(Attribute::Reset));
                let left_border = if i == 0 { "├" } else { "│" };
                let _ = execute!(out, cursor::MoveTo(u16::try_from(left_w).unwrap_or(u16::MAX), row),
                    SetAttribute(Attribute::Dim), Print(left_border), SetAttribute(Attribute::Reset));
                if has_right {
                    let right_border = if i == 0 { "┤" } else { "│" };
                    let _ = execute!(out, cursor::MoveTo(right_sep_col, row),
                        SetAttribute(Attribute::Dim), Print(right_border), SetAttribute(Attribute::Reset));
                }
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
        let mut menu_shown = 0usize;
        if let Some(labels) = actions {
            // Rows left for the menu once the hint and prompt rows below it
            // are reserved. A menu longer than the pane used to keep printing
            // past the bottom — 11 of 35 mulligan-bottom options were simply
            // invisible (#60) — and the marker #60 added still left hidden
            // entries reachable only by typing a number the player could not
            // see, which mis-cast a spell in a real game (#96). The menu now
            // renders a page starting at `menu_offset`, advanced with 'm';
            // indices are absolute, so any number works from any page.
            let avail = h.saturating_sub(row as usize + 2);
            let offset = menu_offset.min(labels.len().saturating_sub(1));
            let remaining = labels.len() - offset;
            let paged = offset > 0 || remaining > avail;
            let shown = if paged {
                avail.saturating_sub(1).max(1).min(remaining)
            } else {
                remaining
            };
            menu_shown = shown;
            for (i, label) in labels.iter().enumerate().skip(offset).take(shown) {
                let _ = execute!(out, cursor::MoveTo(mid_col, row),
                    SetAttribute(Attribute::Bold), Print(format!("  {i}")),
                    SetAttribute(Attribute::Reset), Print(": "));
                // Clip to the panel like every other row — but from the
                // middle, and never silently: the tail is what tells
                // otherwise-identical entries apart (" targeting X",
                // ", sacrificing Y"), and end-clipping it re-created the
                // #36 blind-target menu for any ability whose description
                // ran long — three self-hits in real games (issue #80).
                let plen = 4 + i.to_string().chars().count();
                let label = Self::clip_middle(label, mid_w.saturating_sub(plen));
                Self::print_action_label(&mut out, &label);
                row += 1;
            }
            if paged {
                let marker = format!("  … showing {}-{} of 0-{} — m = next page (any number works)",
                    offset, offset + shown - 1, labels.len() - 1);
                let marker: String = marker.chars().take(mid_w).collect();
                let _ = execute!(out, cursor::MoveTo(mid_col, row),
                    SetAttribute(Attribute::Dim), Print(marker), SetAttribute(Attribute::Reset));
                row += 1;
            }
            let has_pass = labels.first().is_some_and(|l| l == "Pass priority");
            let hints = if has_pass {
                "  [enter=pass] [f=auto-pass] [/=search] [d=deck] [l=log] [g=gy] [e=exile]"
            } else {
                "  [/=search] [d=deck] [l=log] [g=gy] [e=exile]"
            };
            // Clipped to the panel like every other row — at full length this
            // ate the right border and the card panel behind it (#53).
            let hints: String = hints.chars().take(mid_w).collect();
            let _ = execute!(out, cursor::MoveTo(mid_col, row),
                SetAttribute(Attribute::Dim), Print(hints), SetAttribute(Attribute::Reset));
            row += 1;
        }

        // ── Right panel: card reference ──
        if has_right {
            let registry = mtg_engine::cards::CardRegistry::with_all_cards();
            let card_refs = Self::build_card_refs(view, &registry, card_filter);
            Self::render_right_panel(&mut out, &card_refs, right_col, right_w, h, card_filter);
        }

        // Print prompt and move cursor to input area in middle panel
        let _ = execute!(out, cursor::MoveTo(mid_col, row), Print("  > "));
        let _ = out.flush();
        menu_shown
    }

    /// Display name for a counter kind, as it reads on a battlefield line.
    fn counter_display_name(ct: mtg_engine::types::CounterType) -> &'static str {
        use mtg_engine::types::CounterType;
        match ct {
            CounterType::PlusOnePlusOne => "+1/+1",
            CounterType::MinusOneMinusOne => "-1/-1",
            CounterType::Loyalty => "loyalty",
            CounterType::Slime => "slime",
            CounterType::Study => "study",
            CounterType::Hatchling => "hatchling",
        }
    }

    /// " {2 hatchling, 3 +1/+1}" for a permanent's non-loyalty counters, or
    /// "" when it has none. Counters are public information (CR 122.3) and
    /// were invisible everywhere in the CLI except the log line that added
    /// them (issue #82); loyalty stays with the planeswalker line's own
    /// `[N loyalty]` rendering (#58).
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
                for (i, (name, untapped, tapped)) in summary.iter().enumerate() {
                    if i > 0 {
                        let _ = execute!(out, SetForegroundColor(color), Print(", "), ResetColor);
                    }
                    let total = untapped + tapped;
                    let _ = execute!(out, SetForegroundColor(color), Print(format!("{total}x ")), ResetColor);
                    if let Some(bg) = CliPlayer::basic_land_bg(name) {
                        let _ = execute!(out, SetBackgroundColor(bg), SetForegroundColor(Color::Black),
                            Print(name), ResetColor);
                    } else {
                        let _ = execute!(out, SetForegroundColor(color), Print(name), ResetColor);
                    }
                    if *tapped > 0 && *untapped > 0 {
                        let _ = execute!(out, SetForegroundColor(color),
                            Print(format!(" ({tapped} tapped)")), ResetColor);
                    } else if *tapped > 0 {
                        let _ = execute!(out, SetForegroundColor(color), Print(" (tapped)"), ResetColor);
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
            let creature_labels = creatures.iter().map(|c| {
                let pt = match (c.effective_power, c.effective_toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => match (c.power, c.toughness) {
                        (Some(p), Some(t)) => format!(" {p}/{t}"),
                        _ => String::new(),
                    },
                };
                let auras = aura_map.get(&c.object_id)
                    .map(|names| format!(" [{}]", names.join(",")))
                    .unwrap_or_default();
                let dmg = if c.damage_marked > 0 { format!(" ({}d)", c.damage_marked) } else { String::new() };
                let flags = format!("{}{}",
                    if c.tapped { " [T]" } else { "" },
                    if c.summoning_sick { " [S]" } else { "" });
                format!("{}{}{}{}{}{}", c.name, pt,
                    CliPlayer::counters_suffix(&c.counters), auras, dmg, flags)
            }).collect();
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
                    format!("{}{}{}", e.name, host,
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
                .map(|a| format!("{}{}{}", a.name,
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
                let text = format!("  {} [{loyalty} loyalty]{dmg}", pw.name);
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
    fn build_card_refs(view: &GameView, registry: &mtg_engine::cards::CardRegistry, filter: &str) -> Vec<mtg_engine::cards::CardData> {
        let mut seen: Vec<String> = Vec::new();
        let mut card_ids: Vec<mtg_engine::ids::CardId> = Vec::new();

        let mut add = |name: &str, card_id: mtg_engine::ids::CardId| {
            if !seen.contains(&name.to_string()) {
                seen.push(name.to_string());
                card_ids.push(card_id);
            }
        };

        // Priority 1: cards in your hand
        for c in &view.your_hand {
            add(&c.name, c.card_id);
        }
        // Priority 2: cards on the stack
        for s in &view.stack {
            add(&s.name, s.card_id);
        }
        // Priority 3: opponent's battlefield (skip basic lands)
        for p in view.battlefield.iter().filter(|p| p.controller != view.you) {
            let is_basic = registry.card_data(p.card_id)
                .is_some_and(|d| d.supertypes.contains(&mtg_engine::types::Supertype::Basic));
            if is_basic { continue; }
            add(&p.name, p.card_id);
        }
        // Priority 4: your battlefield (skip basic lands)
        for p in view.battlefield.iter().filter(|p| p.controller == view.you) {
            let is_basic = registry.card_data(p.card_id)
                .is_some_and(|d| d.supertypes.contains(&mtg_engine::types::Supertype::Basic));
            if is_basic { continue; }
            add(&p.name, p.card_id);
        }
        // Priority 5: graveyard flashback cards (yours)
        for (pid, cards) in &view.graveyards {
            if *pid == view.you {
                for c in cards {
                    if c.flashback_cost.is_some() {
                        add(&c.name, c.card_id);
                    }
                }
            }
        }
        // Priority 6: recently seen cards in graveyards (both players,
        // most recent first — instants/sorceries that just resolved,
        // creatures that just died). Graveyard order is chronological.
        for (_, cards) in &view.graveyards {
            for c in cards.iter().rev() {
                add(&c.name, c.card_id);
            }
        }
        // Priority 7: exile (cards that were exiled)
        for c in &view.exile {
            add(&c.name, c.card_id);
        }

        // Look up CardData, filter out basic lands, apply text filter
        let filter_lower = filter.to_lowercase();
        card_ids.iter()
            .filter_map(|id| registry.card_data(*id))
            .filter(|d| !d.supertypes.contains(&mtg_engine::types::Supertype::Basic))
            .filter(|d| filter.is_empty() || d.name.to_lowercase().contains(&filter_lower))
            .collect()
    }

    /// Render the card reference panel in the right column.
    fn render_right_panel(out: &mut io::Stdout, cards: &[mtg_engine::cards::CardData],
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
            let text = format!(" /{filter}");
            let pad = right_w.saturating_sub(text.chars().count());
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
            let cost_str = card.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
            let name_line = format!("{}{}", card.name, cost_str);
            let truncated: String = name_line.chars().take(content_w).collect();
            let _ = execute!(out, cursor::MoveTo(right_col, row), SetAttribute(Attribute::Bold));
            Self::print_with_mana(out, &truncated, None);
            let _ = execute!(out, SetAttribute(Attribute::Reset));
            row += 1;
            if row >= max_row { break; }

            // Type line + P/T
            let types: Vec<&str> = card.card_types.iter().map(|t| match t {
                CardType::Creature => "Creature",
                CardType::Instant => "Instant",
                CardType::Sorcery => "Sorcery",
                CardType::Enchantment => "Enchantment",
                CardType::Artifact => "Artifact",
                CardType::Land => "Land",
                CardType::Planeswalker => "Planeswalker",
            }).collect();
            let subtypes = if card.subtypes.is_empty() { String::new() }
                else { format!(" — {}", card.subtypes.join(" ")) };
            let pt = match (card.power, card.toughness) {
                (Some(p), Some(t)) => format!(" {p}/{t}"),
                _ => String::new(),
            };
            let type_line = format!("{}{}{}", types.join(" "), subtypes, pt);
            let truncated: String = type_line.chars().take(content_w).collect();
            let _ = execute!(out, cursor::MoveTo(right_col, row),
                SetAttribute(Attribute::Dim), Print(&truncated), SetAttribute(Attribute::Reset));
            row += 1;
            if row >= max_row { break; }

            // Keywords
            if !card.keywords.is_empty() {
                let kw_str: Vec<&str> = card.keywords.iter().map(|k| match k {
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

            // Oracle text (word-wrapped), skipping lines that just repeat keywords
            if !card.oracle_text.is_empty() {
                let keyword_names: Vec<&str> = vec![
                    "Flying", "First strike", "Double strike", "Trample", "Deathtouch",
                    "Lifelink", "Vigilance", "Flash", "Reach", "Haste", "Defender",
                    "Hexproof", "Intimidate", "Menace", "Indestructible",
                ];
                let lines: Vec<&str> = card.oracle_text.split('\n').collect();
                let non_keyword_text: Vec<&str> = lines.iter()
                    .filter(|line| {
                        let trimmed = line.trim().trim_end_matches(',');
                        !keyword_names.iter().any(|kw| trimmed.eq_ignore_ascii_case(kw))
                    })
                    .copied()
                    .collect();
                let text = non_keyword_text.join("\n");
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
            if let Some(fb) = &card.flashback_cost {
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
    fn run_card_search(&mut self, view: &GameView, actions: &[String]) {
        let _ = terminal::enable_raw_mode();

        self.card_filter.clear();

        loop {
            // Re-render with current filter
            Self::render(view, Some(actions), None, &view.display_log, &self.card_filter, None);

            // Move actual cursor to the search box in the right gutter
            let (term_w, _) = terminal::size().unwrap_or((100, 30));
            let w = term_w as usize;
            let gutter_w = w / 5;
            let mid_w = w.saturating_sub(gutter_w * 2 + 2);
            let right_col = u16::try_from(gutter_w + 1 + mid_w + 1).unwrap_or(u16::MAX);
            let cursor_x = right_col + 2 + u16::try_from(self.card_filter.chars().count()).unwrap_or(u16::MAX);
            let _ = execute!(stdout(), cursor::MoveTo(cursor_x, 1));
            let _ = stdout().flush();

            // Read one key event
            if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
                match code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('/') => {
                        self.card_filter.clear();
                        break;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        let _ = terminal::disable_raw_mode();
                        std::process::exit(0);
                    }
                    KeyCode::Backspace => {
                        self.card_filter.pop();
                    }
                    // A chord the UI doesn't bind (Ctrl-A, Alt-x, ...) is
                    // ignored, never inserted as its bare character (#51).
                    KeyCode::Char(c) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        self.card_filter.push(c);
                    }
                    _ => {}
                }
            }
        }

        let _ = terminal::disable_raw_mode();
    }

    /// Interactive target selection for a castable spell.
    /// Returns None if the user cancels (presses Escape/back).
    fn choose_targets(view: &GameView, spell: &mtg_engine::actions::CastableSpell) -> Option<Action> {
        use mtg_engine::actions::CastTargetSpec;

        let chosen_targets = match &spell.target_spec {
            CastTargetSpec::NoTargets => vec![],
            CastTargetSpec::SingleTarget(options) => {
                if options.len() == 1 {
                    vec![options[0].clone()]
                } else {
                    let target = Self::prompt_target(view,options, &format!("{}: select a target", spell.name))?;
                    vec![target]
                }
            }
            CastTargetSpec::TwoTargets { first, second, second_min, second_max } => {
                let t1 = Self::prompt_target(view, first, &format!("{}: select first of two targets", spell.name))?;
                let idx = first.iter().position(|t| *t == t1)?;
                // The engine pre-narrowed each first choice's legal second-slot
                // options (e.g. "cards from THEIR graveyard" — only the chosen
                // player's cards).
                let mut remaining = second[idx].clone();
                if *second_max <= 1 {
                    if remaining.is_empty() {
                        return None;
                    }
                    let t2 = Self::prompt_target(view, &remaining, &format!("{}: select second of two targets", spell.name))?;
                    vec![t1, t2]
                } else {
                    // "Up to N" second slot: pick 0..=N.
                    let mut chosen = vec![t1];
                    for i in 0..*second_max {
                        if remaining.is_empty() { break; }
                        let label = format!("{}: select target {} of up to {}",
                            spell.name, i + 1, second_max);
                        match Self::prompt_target_optional(view, &remaining, &label) {
                            Some(target) => {
                                remaining.retain(|t| *t != target);
                                chosen.push(target);
                            }
                            None => break,
                        }
                    }
                    if chosen.len() <= *second_min {
                        // Mandatory second target not chosen — treat as cancel.
                        return None;
                    }
                    chosen
                }
            }
            CastTargetSpec::UpToTargets { max, options } => {
                let mut chosen = Vec::new();
                let mut remaining = options.clone();
                for i in 0..*max {
                    if remaining.is_empty() { break; }
                    let label = format!("{}: select target {} of up to {}",
                        spell.name, i + 1, max);
                    match Self::prompt_target_up_to(view, &remaining, &label) {
                        UpToPick::Pick(target) => {
                            remaining.retain(|t| *t != target);
                            chosen.push(target);
                        }
                        UpToPick::Done => break,
                        UpToPick::Cancel => return None,
                    }
                }
                // CR 601.2c: an "up to N targets" spell may be cast choosing
                // zero — including when no legal target exists at all. An
                // empty choice is a real cast, not a cancel; treating it as
                // one made the menu entry a silent no-op (issue #49).
                chosen
            }
        };

        // Prompt for sacrifice if the spell has a sacrifice additional cost.
        let chosen_sacrifice = match spell.sacrifice_options.len() {
            0 => None,
            1 => Some(spell.sacrifice_options[0]),
            _ => {
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
            alternative_cost: None,
            tap_plan: spell.tap_plan.clone(),
        })
    }

    /// Prompt the user to pick one target from a list. Returns None on cancel.
    fn prompt_target(view: &GameView, options: &[mtg_engine::actions::Target], label: &str) -> Option<mtg_engine::actions::Target> {
        let mut labels: Vec<String> = options.iter().map(|t| match t {
            mtg_engine::actions::Target::Object(id) => Self::perm_name(view, *id),
            mtg_engine::actions::Target::Player(pid) => {
                if *pid == view.you { "You".into() } else { "Opponent".into() }
            }
            mtg_engine::actions::Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
        }).collect();
        labels.push("Cancel".into());

        loop {
            Self::render(view, Some(&labels), Some(label), &view.display_log, "", None);
            let input = Self::read_line("");
            if input.is_empty() { return None; }
            if let Ok(idx) = input.parse::<usize>() {
                if idx < options.len() {
                    return Some(options[idx].clone());
                }
                if idx == options.len() { return None; }
            }
        }
    }

    /// Prompt for an optional target (for "up to N" spells). Empty = done.
    /// Pick one target of an "up to N" batch, or stop. `Done` casts with the
    /// targets chosen so far (legal even at zero — CR 601.2c); `Cancel`
    /// abandons the cast, which `Done` used to double as.
    fn prompt_target_up_to(view: &GameView, options: &[mtg_engine::actions::Target], label: &str) -> UpToPick {
        let mut labels: Vec<String> = options.iter().map(|t| match t {
            mtg_engine::actions::Target::Object(id) => Self::perm_name(view, *id),
            mtg_engine::actions::Target::Player(pid) => {
                if *pid == view.you { "You".into() } else { "Opponent".into() }
            }
            mtg_engine::actions::Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
        }).collect();
        labels.push("Done (cast with targets chosen so far)".into());
        labels.push("Cancel the cast".into());

        loop {
            Self::render(view, Some(&labels), Some(label), &view.display_log, "", None);
            let input = Self::read_line("");
            if input.is_empty() { return UpToPick::Done; }
            if let Ok(idx) = input.parse::<usize>() {
                if idx < options.len() {
                    return UpToPick::Pick(options[idx].clone());
                }
                if idx == options.len() { return UpToPick::Done; }
                if idx == options.len() + 1 { return UpToPick::Cancel; }
            }
        }
    }

    fn prompt_target_optional(view: &GameView, options: &[mtg_engine::actions::Target], label: &str) -> Option<mtg_engine::actions::Target> {
        let mut labels: Vec<String> = options.iter().map(|t| match t {
            mtg_engine::actions::Target::Object(id) => Self::perm_name(view, *id),
            mtg_engine::actions::Target::Player(pid) => {
                if *pid == view.you { "You".into() } else { "Opponent".into() }
            }
            mtg_engine::actions::Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
        }).collect();
        labels.push("Done".into());

        loop {
            Self::render(view, Some(&labels), Some(label), &view.display_log, "", None);
            let input = Self::read_line("");
            if input.is_empty() { return None; }
            if let Ok(idx) = input.parse::<usize>() {
                if idx < options.len() {
                    return Some(options[idx].clone());
                }
                if idx == options.len() { return None; }
            }
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
            .or_else(|| view.revealed_names.get(&id).cloned())
            .unwrap_or_else(|| format!("{id}"))
    }

    /// Truncate `s` to `cap` characters by dropping the MIDDLE behind a
    /// visible '…': the head names the permanent and ability, the tail
    /// carries the disambiguating choice (" targeting X", ", sacrificing
    /// Y"), and both must survive — end-clipping the tail rendered N
    /// byte-identical menu entries whose choice silently decided who got
    /// hit (issue #80, defeating the #36 fix).
    fn clip_middle(s: &str, cap: usize) -> String {
        let len = s.chars().count();
        if len <= cap {
            return s.to_string();
        }
        if cap <= 1 {
            return "…".chars().take(cap).collect();
        }
        // Rough 3:2 split favors the head; the ellipsis takes one slot.
        let tail_len = (cap - 1) * 2 / 5;
        let head_len = cap - 1 - tail_len;
        let head: String = s.chars().take(head_len).collect();
        let tail: String = s.chars().skip(len - tail_len).collect();
        format!("{head}…{tail}")
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

    fn format_action(view: &GameView, action: &Action) -> String {
        match action {
            Action::PassPriority => "Pass priority".into(),
            Action::PlayLand { object_id } =>
                format!("Play land {}", Self::perm_name(view, *object_id)),
            Action::CastSpell { object_id, targets, tap_plan, .. } => {
                let name = Self::perm_name(view, *object_id);
                let tap_str = Self::format_tap_plan(view, tap_plan);
                let tap_suffix = if tap_str.is_empty() { String::new() } else { format!(" (tap {tap_str})") };
                if targets.is_empty() {
                    format!("Cast {name}{tap_suffix}")
                } else {
                    let target_names: Vec<String> = targets.iter().map(|t| match t {
                        Target::Object(id) => Self::perm_name(view, *id),
                        Target::Player(pid) => {
                            if *pid == view.you { "you".into() } else { "opponent".into() }
                        }
                        Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
                    }).collect();
                    format!("Cast {} -> {}{}", name, target_names.join(", "), tap_suffix)
                }
            }
            Action::ActivateManaAbility { object_id, .. } =>
                format!("Tap {} for mana", Self::perm_name(view, *object_id)),
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
                    ResolvedChoice::ChosenSubset(ids) => {
                        let names: Vec<String> = ids.iter()
                            .map(|id| Self::perm_name(view, *id))
                            .collect();
                        format!("Pile 1: [{}]", if names.is_empty() { "empty".into() } else { names.join(", ") })
                    }
                    ResolvedChoice::XFunding(response) => format!("Fund X = {}", response.x_value()),
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
        let _ = execute!(out, Print(prompt));
        let _ = out.flush();
        let _ = terminal::enable_raw_mode();
        // Same paste hardening as the menu reader (#50): a multi-line paste
        // must not submit on its embedded newlines.
        let _ = execute!(out, event::EnableBracketedPaste);
        // Pending type-ahead is dropped — the cooked read's mode switch did
        // this by accident, #71 does it on purpose: a keystroke must never
        // answer a prompt the player has not been shown.
        while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            let _ = event::read();
        }
        let mut buf = String::new();
        loop {
            let ev = match event::read() {
                Ok(ev) => ev,
                Err(_) => continue,
            };
            if let Event::Paste(pasted) = &ev {
                let first = pasted.split(['\r', '\n']).next().unwrap_or("");
                buf.push_str(first);
                let _ = execute!(out, Print(first));
                let _ = out.flush();
                continue;
            }
            let Event::Key(KeyEvent { code, modifiers, .. }) = ev else { continue };
            match code {
                KeyCode::Enter => break,
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    let _ = execute!(out, event::DisableBracketedPaste);
                    let _ = terminal::disable_raw_mode();
                    std::process::exit(0);
                }
                // Ctrl-U kills the line (issue #79), here as in the menu reader.
                KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                    for _ in 0..buf.chars().count() {
                        let _ = execute!(out, Print("\x08 \x08"));
                    }
                    buf.clear();
                    let _ = out.flush();
                }
                KeyCode::Backspace => {
                    if buf.pop().is_some() {
                        let _ = execute!(out, Print("\x08 \x08"));
                        let _ = out.flush();
                    }
                }
                KeyCode::Char(c) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                    buf.push(c);
                    let _ = execute!(out, Print(c.to_string()));
                    let _ = out.flush();
                }
                // Unbound chords are ignored, never typed (#51).
                _ => {}
            }
        }
        let _ = execute!(out, event::DisableBracketedPaste, Print("\r\n"));
        let _ = terminal::disable_raw_mode();
        buf.trim().to_string()
    }

    /// A y/n confirmation that answers on a single keypress: `y` confirms,
    /// `n` or Esc declines, anything else visibly re-prompts. Runs in raw
    /// mode with explicit echo, so a stray keystroke can never sit
    /// invisibly in a line buffer (issue #42).
    fn confirm_yn(prompt: &str) -> bool {
        let mut out = stdout();
        let _ = execute!(out, Print(prompt));
        let _ = out.flush();
        let was_raw = terminal::is_raw_mode_enabled().unwrap_or(false);
        let _ = terminal::enable_raw_mode();
        let answer = loop {
            if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
                match code {
                    // Bare y/n only: a control/alt chord must not answer a
                    // confirmation prompt (#51).
                    KeyCode::Char('y' | 'Y') if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => { let _ = execute!(stdout(), Print("y")); break true; }
                    KeyCode::Char('n' | 'N') if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => { let _ = execute!(stdout(), Print("n")); break false; }
                    KeyCode::Esc => { let _ = execute!(stdout(), Print("n")); break false; }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        let _ = terminal::disable_raw_mode();
                        std::process::exit(0);
                    }
                    _ => {
                        let _ = execute!(stdout(), Print("\r\n  Please answer y or n. "), Print(prompt.trim_start()));
                        let _ = stdout().flush();
                    }
                }
            }
        };
        if !was_raw {
            let _ = terminal::disable_raw_mode();
        }
        let _ = execute!(stdout(), Print("\r\n"));
        answer
    }

    /// Read a line of input, but detect '/' immediately (without Enter)
    /// to trigger card search. Returns None if '/' was pressed first.
    fn read_line_with_search(_col: u16) -> Option<String> {
        // Prompt "> " is already printed by render.
        let mut out = stdout();
        let _ = terminal::enable_raw_mode();
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
        let mut echoed: usize = 0;

        // Bracketed paste, enabled only for this raw-mode read: without it a
        // multi-line paste arrives as N keystroke sequences whose embedded
        // newlines SUBMIT — one stray paste answered thirty prompts, made
        // cleanup discards for both seats, and advanced eleven turns (#50).
        // With it the terminal delivers the paste as one event, the first
        // line lands in the buffer, and nothing submits until a real Enter.
        let _ = execute!(out, event::EnableBracketedPaste);

        let result = loop {
            let ev = match event::read() {
                Ok(ev) => ev,
                Err(_) => continue,
            };
            if let Event::Paste(pasted) = &ev {
                let first = pasted.split(['\r', '\n']).next().unwrap_or("");
                for c in first.chars() {
                    buf.push(c);
                    if echoed < echo_cap {
                        let _ = execute!(out, Print(c.to_string()));
                        echoed += 1;
                    }
                }
                let _ = out.flush();
                continue;
            }
            if let Event::Key(KeyEvent { code, modifiers, .. }) = ev {
                match code {
                    KeyCode::Char('/') if buf.is_empty() && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        break None; // trigger card search
                    }
                    KeyCode::Char('r') if buf.is_empty() && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        // Wait briefly for a second 'r' to trigger hot reload.
                        if event::poll(std::time::Duration::from_millis(300)).unwrap_or(false) {
                            if let Ok(Event::Key(KeyEvent { code: KeyCode::Char('r'), .. })) = event::read() {
                                HOT_RELOAD_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
                                let _ = terminal::disable_raw_mode();
                                break Some("__hot_reload__".into());
                            }
                        }
                        // Single 'r' — treat as normal input.
                        buf.push('r');
                        if echoed < echo_cap {
                            let _ = execute!(out, Print("r"));
                            let _ = out.flush();
                            echoed += 1;
                        }
                    }
                    KeyCode::Enter => {
                        break Some(buf.clone());
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        let _ = execute!(stdout(), event::DisableBracketedPaste);
                        let _ = terminal::disable_raw_mode();
                        std::process::exit(0);
                    }
                    KeyCode::Backspace => {
                        if buf.pop().is_some() && echoed > buf.chars().count() {
                            // Erase character on screen (only ones echoed)
                            let _ = execute!(out, Print("\x08 \x08"));
                            let _ = out.flush();
                            echoed -= 1;
                        }
                    }
                    // Ctrl-U: kill the line, the standard readline binding
                    // and the documented recovery from a garbled prompt.
                    // Without it the stray characters stayed in the buffer
                    // and corrupted the next input — exactly the situation
                    // the recovery step exists for (issue #79).
                    KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                        buf.clear();
                        while echoed > 0 {
                            let _ = execute!(out, Print("\x08 \x08"));
                            echoed -= 1;
                        }
                        let _ = out.flush();
                    }
                    // Unbound chords are ignored, never typed: Ctrl-L must
                    // not become the 'l' shortcut, and crossterm reports the
                    // 0x1C-0x1F control codes (Ctrl-\ among them - SIGQUIT's
                    // key) as the DIGITS 4-7 with CONTROL set, which used to
                    // silently pick menu entries (#51).
                    KeyCode::Char(c) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        buf.push(c);
                        if echoed < echo_cap {
                            let _ = execute!(out, Print(c.to_string()));
                            let _ = out.flush();
                            echoed += 1;
                        }
                    }
                    _ => {}
                }
            }
        };

        let _ = execute!(stdout(), event::DisableBracketedPaste);
        let _ = terminal::disable_raw_mode();
        result
    }

    fn show_battlefield_inspector(view: &GameView) {
        let registry = mtg_engine::cards::CardRegistry::with_all_cards();
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
                    if perm.summoning_sick { " [S]" } else { "" });
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
                    if perm.summoning_sick { " [S]" } else { "" });
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

                    let types: Vec<&str> = perm.card_types.iter().map(|t| match t {
                        CardType::Land => "Land",
                        CardType::Creature => "Creature",
                        CardType::Instant => "Instant",
                        CardType::Sorcery => "Sorcery",
                        CardType::Enchantment => "Enchantment",
                        CardType::Artifact => "Artifact",
                        CardType::Planeswalker => "Planeswalker",
                    }).collect();
                    let _ = execute!(out, Print(format!("  Type: {}\n", types.join(" "))));

                    if let (Some(p), Some(t)) = (perm.power, perm.toughness) {
                        let _ = execute!(out, Print(format!("  Base P/T: {p}/{t}\n")));
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
                    let _ = execute!(out, Print(format!("  Summoning sick: {}\n", perm.summoning_sick)));
                    let _ = execute!(out, Print(format!("  ID: #{}\n", perm.object_id.0)));

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

                    // Show oracle text
                    if let Some(data) = registry.card_data(perm.card_id) {
                        if !data.oracle_text.is_empty() {
                            let _ = execute!(out, Print("\n"),
                                SetForegroundColor(Color::Yellow),
                                Print(format!("  {}\n", data.oracle_text)),
                                ResetColor);
                        }
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
            if perm.controller == view.you {
                *board_counts.entry(perm.name.clone()).or_default() += 1;
            }
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
                    let types: Vec<&str> = data.card_types.iter().map(|t| match t {
                        CardType::Land => "Land",
                        CardType::Creature => "Creature",
                        CardType::Instant => "Instant",
                        CardType::Sorcery => "Sorcery",
                        CardType::Enchantment => "Enchantment",
                        CardType::Artifact => "Artifact",
                        CardType::Planeswalker => "Planeswalker",
                    }).collect();
                    let _ = execute!(out, Print(format!("  Type: {}\n", types.join(" "))));
                    if !data.subtypes.is_empty() {
                        let _ = execute!(out, Print(format!("  Subtypes: {}\n", data.subtypes.join(", "))));
                    }
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

    fn choose_attackers(view: &GameView, prompt: &CombatPrompt) -> Action {
        let CombatPrompt::ChooseAttackers { eligible, must_attack, defending_player: defending,
                                            defending_planeswalkers } = prompt else {
            unreachable!()
        };
        let defending = *defending;

        if eligible.is_empty() {
            return Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] };
        }

        Self::render(view, None, Some("DECLARE ATTACKERS"), &view.display_log, "", None);

        // Get mid_col for positioning — action area starts where cursor was left
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
        let cur_row = cursor::position().unwrap_or((0, 20)).1;

        let mut out = stdout();
        let mut r = cur_row;
        let _ = execute!(out, cursor::MoveTo(col, r),
            SetForegroundColor(Color::Yellow), SetAttribute(Attribute::Bold),
            Print(" Eligible attackers:"), SetAttribute(Attribute::Reset), ResetColor);
        r += 1;
        for (i, &id) in eligible.iter().enumerate() {
            let forced = must_attack.contains(&id);
            let tag = if forced { " [MUST ATTACK]" } else { "" };
            let color = if forced { Color::Red } else { Color::Reset };
            let _ = execute!(out, cursor::MoveTo(col, r),
                SetAttribute(Attribute::Bold), Print(format!("  {i}")),
                SetAttribute(Attribute::Reset),
                Print(format!(": {}", Self::perm_name(view, id))),
                SetForegroundColor(color), Print(tag), ResetColor);
            r += 1;
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
        let _ = execute!(out, cursor::MoveTo(col, r));
        let _ = out.flush();

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

        loop {
            // Clear the row before re-prompting: a rejected entry's characters
            // otherwise stay on screen and visually merge with the next
            // attempt ("7" typed over stale "abc" reads as "7bc" — issue #35).
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            let input = Self::read_line("  Attack (numbers/all/none, enter=none)> ");

            // Bare Enter means "do nothing" — here as at every other prompt
            // ([enter=pass] at the menu, enter=none at blockers). It used to
            // mean "all": the one prompt where the universal idle key took
            // the most aggressive irreversible action available, tapping the
            // whole board on a key-repeat or a stray Ctrl-D (issue #73).
            // CR 508.1a backs "none": attacking is a choice, and choosing no
            // attackers is always legal (must-attack is enforced below).
            if input.is_empty() || input == "none" || input == "n" {
                if !must_attack.is_empty() {
                    println!("{}", forced_error(&must_attack.iter().copied().collect::<Vec<_>>()));
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
                let bad: Vec<usize> = indices.iter().copied().filter(|&i| i >= eligible.len())
                    .chain(walker_attacks.iter().filter(|&&(a, w)|
                        a >= eligible.len() || w >= defending_planeswalkers.len()).map(|&(a, _)| a))
                    .collect();
                if bad.is_empty() {
                    let chosen: Vec<ObjectId> = indices.iter().map(|&i| eligible[i])
                        .chain(walker_attacks.iter().map(|&(a, _)| eligible[a]))
                        .collect();
                    let missing = missing_forced(&chosen);
                    if !missing.is_empty() {
                        println!("{}", forced_error(&missing));
                        continue;
                    }
                    return Action::DeclareAttackers {
                        attackers: indices.iter().map(|&i| (eligible[i], defending)).collect(),
                        planeswalker_attacks: walker_attacks.iter()
                            .map(|&(a, w)| (eligible[a], defending_planeswalkers[w]))
                            .collect(),
                    };
                }
                println!("  Invalid attacker(s): {}. Valid range is 0-{}.",
                    bad.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(", "),
                    eligible.len() - 1);
            } else {
                println!("  Invalid input. Enter numbers like '0 2', 'all', 'a', or 'none'.");
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

        Self::render(view, None, Some("DECLARE BLOCKERS"), &view.display_log, "", None);

        // Get mid_col for positioning — action area starts where cursor was left
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
        let cur_row = cursor::position().unwrap_or((0, 20)).1;

        let mut out = stdout();
        let mut r = cur_row;
        let _ = execute!(out, cursor::MoveTo(col, r),
            SetForegroundColor(Color::Red), SetAttribute(Attribute::Bold),
            Print(" Attackers:"), SetAttribute(Attribute::Reset), ResetColor);
        r += 1;
        for (i, &id) in attacker_ids.iter().enumerate() {
            // CR 509.1b: say the minimum-blockers requirement (menace,
            // Terror of Kruin Pass) up front — an unmarked menace attacker
            // took a single block the engine then discarded (issue #72).
            let note = min_blockers.get(&id)
                .map(|min| format!(" [needs {min}+ blockers]"))
                .unwrap_or_default();
            let _ = execute!(out, cursor::MoveTo(col, r),
                Print(format!("  {}: {}{}", i, Self::perm_name(view, id), note)));
            r += 1;
        }
        let _ = execute!(out, cursor::MoveTo(col, r),
            SetForegroundColor(Color::Green), SetAttribute(Attribute::Bold),
            Print(" Your blockers:"), SetAttribute(Attribute::Reset), ResetColor);
        r += 1;
        for (i, &id) in eligible_blockers.iter().enumerate() {
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
            let _ = execute!(out, cursor::MoveTo(col, r),
                Print(format!("  {}: {}{}", i, Self::perm_name(view, id), note)));
            r += 1;
        }
        let _ = execute!(out, cursor::MoveTo(col, r));
        let _ = out.flush();

        loop {
            // Same stale-row clearing as the attack prompt (issue #35).
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            let input = Self::read_line("  Block (blocker:attacker / enter=none)> ");

            if input.is_empty() {
                return Action::DeclareBlockers { assignments: vec![] };
            }

            let mut assignments = Vec::new();
            let mut error: Option<String> = None;
            for pair in input.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()) {
                let parts: Vec<&str> = pair.split(':').collect();
                if parts.len() != 2 {
                    error = Some("Invalid. Use 'blocker:attacker' pairs like '0:0 1:1'.".into());
                    break;
                }
                match (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    (Ok(b), Ok(a)) if b < eligible_blockers.len() && a < attacker_ids.len() => {
                        let (blocker, attacker) = (eligible_blockers[b], attacker_ids[a]);
                        // CR 509.1b: refuse an illegal pairing here, loudly —
                        // the engine would drop it, and a silently vanished
                        // block cost real games (issue #40).
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
                        // CR 509.1b: one blocker, one attacker — refuse here,
                        // loudly, like the illegal-pairing case above.
                        if assignments.iter().any(|&(b, _)| b == blocker) {
                            error = Some(format!(
                                "{} can block only one attacker (CR 509.1b).",
                                Self::perm_name(view, blocker)));
                            break;
                        }
                        assignments.push((blocker, attacker));
                    }
                    _ => {
                        error = Some("Invalid. Use 'blocker:attacker' pairs like '0:0 1:1'.".into());
                        break;
                    }
                }
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
                Some(msg) => println!("  {msg}"),
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
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        use mtg_engine::funding::FundingResponse;
        use mtg_engine::types::ManaType;

        // Rendered inside the TUI frame like every other prompt: bare
        // println! landed on top of the drawn frame, colliding with the log
        // panel mid-word — "Max X = 1" read as "Max X = 13 (p0) ──" — and
        // left the stale main-phase menu on screen, so players pressed Enter
        // "to retry" and silently funded X = 0 (#56).
        Self::render(view, None, Some(description), &view.display_log, "", None);
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
            Print(clip(&format!("  Max X = {}", options.max_x))),
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

        let x: u32 = loop {
            // Clear the input row before each attempt (same as the combat
            // prompts), so a rejected entry doesn't merge with the next.
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            let input = Self::read_line(&format!("  X (0-{}, blank = 0) = ", options.max_x));
            if input.trim().is_empty() {
                break 0;
            }
            match input.trim().parse::<u32>() {
                Ok(n) if n <= options.max_x => break n,
                _ => {
                    let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine),
                        Print(format!("  Enter an integer between 0 and {}.", options.max_x)));
                    let _ = stdout().flush();
                    std::thread::sleep(std::time::Duration::from_millis(700));
                }
            }
        };

        // Distribute X: drain from pool (larger color buckets first), then
        // tap sources starting from whole-ability steps. Any mismatch at
        // the end (X isn't achievable due to multi-mana-source quanta) is
        // rounded down by dropping excess.
        let mut response = FundingResponse::default();
        let mut remaining = x;

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

    /// Ask the human to divide permanents into two piles (Liliana of the
    /// Veil -6). Lists the permanents with indices; a space-separated list
    /// picks pile 1 and the rest form pile 2. Empty input is a legal empty
    /// pile 1. The engine no longer enumerates the 2^N subsets (issue #142),
    /// so the subset is constructed here from the structured prompt.
    fn prompt_pile_division(
        view: &GameView,
        permanents: &[mtg_engine::ids::ObjectId],
        description: &str,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;

        Self::render(view, None, Some(description), &view.display_log, "", None);
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
        let w = term_w as usize;
        let mid_w = if w >= 100 { w.saturating_sub(2 * side + 2) } else { w.saturating_sub(side + 1) };
        let clip = |s: &str| -> String { s.chars().take(mid_w).collect() };
        let mut r = cursor::position().unwrap_or((0, 20)).1;
        let mut out = stdout();

        let _ = execute!(out, cursor::MoveTo(col, r),
            SetForegroundColor(Color::Yellow),
            Print(clip("  Pick the permanents for pile 1; the rest form pile 2.")),
            ResetColor);
        r += 1;
        for (i, &id) in permanents.iter().enumerate() {
            let _ = execute!(out, cursor::MoveTo(col, r),
                SetAttribute(Attribute::Bold), Print(format!("  {i}")),
                SetAttribute(Attribute::Reset),
                Print(clip(&format!(": {}", Self::perm_name(view, id)))));
            r += 1;
        }
        let _ = execute!(out, cursor::MoveTo(col, r));
        let _ = out.flush();

        let hint = "  pile 1 indices (space-separated, blank = empty pile 1): ";
        let mut error: Option<String> = None;
        loop {
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            if let Some(msg) = error.take() {
                let _ = execute!(stdout(), Print(clip(&msg)));
                let _ = stdout().flush();
                std::thread::sleep(std::time::Duration::from_millis(700));
                let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            }
            let input = Self::read_line(hint);
            let trimmed = input.trim();
            let indices: Vec<usize> = if trimmed.is_empty() {
                vec![]
            } else {
                let parsed: Result<Vec<usize>, _> = trimmed.split_whitespace()
                    .map(str::parse::<usize>)
                    .collect();
                let Ok(v) = parsed else {
                    error = Some("  Invalid input.".into());
                    continue;
                };
                v
            };
            if indices.iter().any(|&i| i >= permanents.len()) {
                error = Some("  Index out of range.".into());
                continue;
            }
            let mut sorted = indices.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.len() != indices.len() {
                error = Some("  Duplicate indices.".into());
                continue;
            }
            let chosen: Vec<mtg_engine::ids::ObjectId> = indices.into_iter()
                .map(|i| permanents[i])
                .collect();
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenSubset(chosen) };
        }
    }

    /// Ask the human which graveyard cards to exile as an additional cost.
    /// Lists candidates with indices; accepts a space-separated list. Empty
    /// input picks the minimum subset (typically the first `min` options).
    fn prompt_exile_from_graveyard(
        view: &GameView,
        options: &[mtg_engine::ids::ObjectId],
        min: usize,
        max: usize,
        description: &str,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;

        // Rendered inside the TUI frame — bare println! straddled the panel
        // borders and let the previous frame bleed through mid-sentence (#56).
        Self::render(view, None, Some(description), &view.display_log, "", None);
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
        let w = term_w as usize;
        let mid_w = if w >= 100 { w.saturating_sub(2 * side + 2) } else { w.saturating_sub(side + 1) };
        let clip = |s: &str| -> String { s.chars().take(mid_w).collect() };
        let mut r = cursor::position().unwrap_or((0, 20)).1;
        let mut out = stdout();

        let count_line = if min == max {
            format!("  Choose exactly {min} card{}.", if min == 1 { "" } else { "s" })
        } else {
            format!("  Choose between {min} and {max} cards.")
        };
        let _ = execute!(out, cursor::MoveTo(col, r),
            SetForegroundColor(Color::Yellow), Print(clip(&count_line)), ResetColor);
        r += 1;
        for (i, &id) in options.iter().enumerate() {
            let _ = execute!(out, cursor::MoveTo(col, r),
                SetAttribute(Attribute::Bold), Print(format!("  {i}")),
                SetAttribute(Attribute::Reset),
                Print(clip(&format!(": {}", Self::perm_name(view, id)))));
            r += 1;
        }
        let _ = execute!(out, cursor::MoveTo(col, r));
        let _ = out.flush();

        // "blank = minimum" only makes sense when there is a real range.
        let hint = if min == max {
            format!("  indices (space-separated, blank = first {min}): ")
        } else {
            format!("  indices (space-separated, blank = minimum {min}): ")
        };
        let mut error: Option<String> = None;
        loop {
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            if let Some(msg) = error.take() {
                let _ = execute!(stdout(), Print(clip(&msg)));
                let _ = stdout().flush();
                std::thread::sleep(std::time::Duration::from_millis(700));
                let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            }
            let input = Self::read_line(&hint);
            let trimmed = input.trim();
            let indices: Vec<usize> = if trimmed.is_empty() {
                (0..min).collect()
            } else {
                let parsed: Result<Vec<usize>, _> = trimmed.split_whitespace()
                    .map(str::parse::<usize>)
                    .collect();
                let Ok(v) = parsed else {
                    error = Some("  Invalid input.".into());
                    continue;
                };
                v
            };
            if indices.iter().any(|&i| i >= options.len()) {
                error = Some("  Index out of range.".into());
                continue;
            }
            let mut sorted = indices.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.len() != indices.len() {
                error = Some("  Duplicate indices.".into());
                continue;
            }
            if indices.len() < min || indices.len() > max {
                error = Some(format!("  Need between {min} and {max} indices."));
                continue;
            }
            let chosen: Vec<mtg_engine::ids::ObjectId> = indices.into_iter()
                .map(|i| options[i])
                .collect();
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(chosen) };
        }
    }

    fn library_search_ui(view: &GameView, actions: &[Action], title: &str) -> Action {
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
        for (i, action) in actions.iter().enumerate() {
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

        let _ = terminal::enable_raw_mode();

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
            let _ = execute!(out, Print(format!("Filter: {filter}_\n\r\n\r")));

            let (_, term_height) = terminal::size().unwrap_or((80, 24));
            let max_list = (term_height as usize).saturating_sub(8); // Leave room for detail

            let start = selected.saturating_sub(max_list / 2);
            let visible: Vec<_> = filtered.iter().enumerate().skip(start).take(max_list).collect();

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
                Print("↑↓ navigate  |  type to filter  |  Enter to select"),
                ResetColor, Print("\n\r"));
            let _ = out.flush();

            // Read input.
            if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
                match code {
                    KeyCode::Enter => {
                        if let Some(card) = filtered.get(selected) {
                            let _ = terminal::disable_raw_mode();
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
                        let _ = terminal::disable_raw_mode();
                        std::process::exit(0);
                    }
                    KeyCode::Char(c) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                        filter.push(c);
                        selected = 0;
                    }
                    KeyCode::Esc => {
                        filter.clear();
                        selected = 0;
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Player for CliPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Action {
        enum DisplayEntry {
            Direct(usize),        // index into legal_actions
            Cast(usize),          // index into legal.castable_spells
        }

        let legal_actions = &legal.actions;

        // X-cost funding: prompt the user for an X value and auto-distribute
        // across pool mana and tap sources (pool first, then by category).
        // A richer per-source UI could be added later.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, description, .. }) =
            legal.resolution_prompt.as_ref()
        {
            return Self::prompt_x_funding(view, options, description);
        }

        // Exile-from-graveyard: prompt for a space-separated list of indices.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseExileFromGraveyard {
            options, min, max, description, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return Self::prompt_exile_from_graveyard(view, options, *min, *max, description);
        }

        // Pile division: prompt for the indices that form pile 1.
        if let Some(mtg_engine::state::ResolutionChoiceKind::DividePermanentsIntoPiles {
            permanents, description, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return Self::prompt_pile_division(view, permanents, description);
        }

        // Special case: library search — show interactive card browser.
        if legal_actions.iter().all(|a| matches!(a, Action::ResolveChoice { .. }))
            && legal_actions.len() > 1
        {
            // Check if these are ChosenCard choices (library/revealed search).
            let all_chosen_cards = legal_actions.iter().all(|a| matches!(a,
                Action::ResolveChoice { choice: mtg_engine::actions::ResolvedChoice::ChosenCard(_) }
            ));
            if all_chosen_cards && legal_actions.len() > 3 {
                let title = legal.context.as_deref().unwrap_or("Choose a card");
                return Self::library_search_ui(view, legal_actions, title);
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
                let should_break = Self::should_break_pass(view, legal, mode);
                if should_break {
                    self.pass_mode = None;
                } else {
                    return Action::PassPriority;
                }
            }
        }

        // Build a collapsed display list: non-CastSpell actions + one entry per castable spell.
        // Each entry maps to either a direct action or an interactive casting flow.
        let mut display: Vec<DisplayEntry> = Vec::new();
        let mut display_labels: Vec<String> = Vec::new();
        let mut seen_spell_objects: Vec<mtg_engine::ids::ObjectId> = Vec::new();

        // Ordering: non-tap actions, cast spells, tap actions, concede last.
        let mut deferred_taps: Vec<(usize, String)> = Vec::new();
        let mut deferred_concede: Option<(usize, String)> = None;
        let mut seen_cast_labels: Vec<String> = Vec::new();
        for (i, action) in legal_actions.iter().enumerate() {
            match action {
                Action::CastSpell { object_id, .. } => {
                    // Skip expanded CastSpell entries — use castable_spells instead.
                    if !seen_spell_objects.contains(object_id) {
                        // Find the matching CastableSpell entry.
                        if let Some(cs_idx) = legal.castable_spells.iter()
                            .position(|cs| cs.object_id == *object_id)
                        {
                            seen_spell_objects.push(*object_id);
                            let cs = &legal.castable_spells[cs_idx];
                            let verb = if cs.is_flashback { "Flashback" } else { "Cast" };
                            let tap_str = Self::format_tap_plan(view, &cs.tap_plan);
                            let label = if tap_str.is_empty() {
                                format!("{} {}", verb, cs.name)
                            } else {
                                format!("{} {} (tap {})", verb, cs.name, tap_str)
                            };
                            // Deduplicate identical cast labels (e.g. two copies of same spell).
                            if seen_cast_labels.contains(&label) { continue; }
                            seen_cast_labels.push(label.clone());
                            display.push(DisplayEntry::Cast(cs_idx));
                            display_labels.push(label);
                        }
                    }
                }
                Action::ActivateManaAbility { .. } => {
                    // Defer tap actions to appear after cast spells.
                    deferred_taps.push((i, Self::format_action(view, action)));
                }
                Action::Concede => {
                    // Defer concede to always be last.
                    deferred_concede = Some((i, Self::format_action(view, action)));
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
                            format!(", sacrificing {}", Self::perm_name(view, *sac)),
                        _ => String::new(),
                    };
                    let label = match desc {
                        Some(d) => format!("{}: {}{}{}", Self::perm_name(view, *object_id),
                            d, Self::targets_suffix(view, targets), sac_suffix),
                        None => format!("{}{}", Self::format_action(view, action), sac_suffix),
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
                    if display_labels.contains(&label) { continue; }
                    display.push(DisplayEntry::Direct(i));
                    display_labels.push(label);
                }
                _ => {
                    display.push(DisplayEntry::Direct(i));
                    display_labels.push(Self::format_action(view, action));
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

        // Issue #71: a decision of a different identity (seat or prompt
        // kind) must not consume keystrokes typed against an earlier
        // prompt. Every menu with a Pass option is the one ordinary
        // priority menu — a single kind, whatever the step, so holding
        // Enter to pass through your own turn keeps working. A menu
        // without Pass is a mandatory choice (discard, sacrifice,
        // bottoming, search), keyed by its context string.
        let kind = if has_pass { "priority" } else { legal.context.as_deref().unwrap_or("") };
        self.drain_stale_input(kind);

        let mut notice: Option<String> = None;
        let mut menu_offset = 0usize;
        loop {
            let pass_label = self.pass_mode.as_ref().map(|m| match m {
                PassMode::UntilNextTurn { .. } => "AUTO-PASS",
            });
            let menu_shown = Self::render_paged(view, Some(&display_labels),
                notice.take().as_deref().or(legal.context.as_deref()),
                &view.display_log, &self.card_filter, pass_label, menu_offset);

            // Read input
            let (term_w, _) = terminal::size().unwrap_or((100, 30));
            let side = term_w as usize / 5;
            let col = u16::try_from(side + 1).unwrap_or(u16::MAX);
            let input = Self::read_line_with_search(col);

            // '/' triggers card search immediately (returns None to re-render)
            if input.is_none() {
                self.run_card_search(view, &display_labels);
                continue;
            }
            let input = input.unwrap();

            // Keyboard shortcuts
            match input.as_str() {
                "g" => {
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
                    continue;
                }
                "e" => {
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
                    continue;
                }
                "i" => {
                    Self::show_battlefield_inspector(view);
                    continue;
                }
                "f" => {
                    // Pass until my next Main Phase 1 (F6-like). The break
                    // check runs against the CURRENT prompt first: if it
                    // would already break here (a land play or castable
                    // spell on offer), engaging would silently discard those
                    // actions (issue #48) — refuse instead.
                    if has_pass {
                        if let Some(mode) = Self::try_engage_auto_pass(view, legal) {
                            self.pass_mode = Some(mode);
                            return Action::PassPriority;
                        }
                        notice = Some(
                            "Auto-pass not engaged: this prompt has actions it would \
                             skip (land play / castable spell). Pass with 0 first to \
                             decline them.".to_string());
                    }
                    continue;
                }
                "__hot_reload__" => {
                    // Hot reload triggered by rapid 'rr' in raw mode.
                    return Action::Concede;
                }
                "l" => {
                    Self::show_log(&view.display_log);
                    continue;
                }
                // Page a menu longer than the pane (issue #96); wraps back
                // to the top after the last page. A no-op on a menu that
                // fits, so 'm' never falls through to be misread as input.
                "m" => {
                    menu_offset = if menu_offset + menu_shown >= display_labels.len() {
                        0
                    } else {
                        menu_offset + menu_shown
                    };
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
                input, display.len().saturating_sub(1)));
        }
    }

}

impl CliPlayer {
    /// Show the game state with a spinning indicator on the opponent's
    /// caret while the AI thinks. Drop the returned handle to stop.
    #[must_use]
    pub fn start_thinking(view: &GameView) -> SpinnerHandle {
        Self::render(view, None, None, &view.display_log, "", None);

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

#[cfg(test)]
mod tests {
    use super::*;
    use mtg_engine::engine::LegalActions;
    use mtg_engine::ids::PlayerId;
    use mtg_engine::types::ManaPool;

    /// Issue #80: a menu label longer than the panel is clipped from the
    /// MIDDLE with a visible ellipsis, never from the end — the tail is
    /// what tells otherwise-identical entries apart (" targeting X").
    #[test]
    fn clip_middle_keeps_the_disambiguating_tail() {
        let label = "Olivia Voldaren 3/3 (your): {1}{R}: Deal 1 damage to \
                     another target creature, make it a Vampire, +1/+1 \
                     counter on Olivia targeting Fiend Hunter 1/3 (opp)";
        let clipped = CliPlayer::clip_middle(label, 113);
        assert_eq!(clipped.chars().count(), 113);
        assert!(clipped.contains('…'), "truncation is visible");
        assert!(clipped.starts_with("Olivia Voldaren"), "the head survives");
        assert!(clipped.ends_with("targeting Fiend Hunter 1/3 (opp)"),
            "the target suffix survives: {clipped}");

        // Short labels pass through untouched, cap-edge cases don't panic.
        assert_eq!(CliPlayer::clip_middle("Pass priority", 113), "Pass priority");
        assert_eq!(CliPlayer::clip_middle("abcdef", 1), "…");
        assert_eq!(CliPlayer::clip_middle("abcdef", 0), "");
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
        assert!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![cast(1)]), &mode));
    }

    // Issue #45 companion: even with nothing castable, our own Main
    // Phase 1 of the press turn is "our next Main Phase 1" — stop there.
    #[test]
    fn same_turn_main_phase_breaks_pass_when_pressed_before_main() {
        let mode = CliPlayer::new_pass_mode(&view(Step::Upkeep, 9, true));
        let v = view(Step::PrecombatMain, 9, true);
        assert!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![]), &mode));
    }

    // 'f' pressed AT our Main Phase 1 is a deliberate skip of the rest of
    // this turn: the same turn's later steps must not re-break for spells.
    #[test]
    fn press_at_main_phase_still_skips_rest_of_turn() {
        let mode = CliPlayer::new_pass_mode(&view(Step::PrecombatMain, 6, true));
        let v = view(Step::EndStep, 6, true);
        assert!(!CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![cast(1)]), &mode));
        // ...but next turn's upkeep with a castable spell breaks, as before.
        let v = view(Step::Upkeep, 7, true);
        assert!(CliPlayer::should_break_pass(&v, &pass_concede_plus(vec![cast(1)]), &mode));
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
        assert!(CliPlayer::try_engage_auto_pass(&v, &l).is_none());
    }

    // Issue #48 companion: with no land play on offer, 'f' at our own Main
    // Phase 1 is a deliberate skip and must still engage.
    #[test]
    fn press_at_own_main_without_land_engages() {
        let v = view(Step::PrecombatMain, 6, true);
        let l = pass_concede_plus(vec![cast(4)]);
        assert!(CliPlayer::try_engage_auto_pass(&v, &l).is_some());
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
            oracle_text: String::new(),
            counters: HashMap::new(),
            loyalty_abilities: vec![],
        };
        let mut v = view(Step::PrecombatMain, 5, true);
        v.battlefield = vec![land(10, 0), land(11, 1)];
        let yours = CliPlayer::perm_name(&v, ObjectId(10));
        let theirs = CliPlayer::perm_name(&v, ObjectId(11));
        assert_eq!(yours, "Island (your)");
        assert_eq!(theirs, "Island (opp)");
        assert_ne!(yours, theirs, "identical lands must be distinguishable");
    }

    // Issue #39 guard: a land play breaks auto-pass on any turn, even the
    // press turn, whatever step the press happened at.
    #[test]
    fn land_play_always_breaks_pass() {
        let mode = CliPlayer::new_pass_mode(&view(Step::PrecombatMain, 6, true));
        let v = view(Step::PostcombatMain, 6, true);
        let l = pass_concede_plus(vec![Action::PlayLand { object_id: ObjectId(3) }]);
        assert!(CliPlayer::should_break_pass(&v, &l, &mode));
    }
}

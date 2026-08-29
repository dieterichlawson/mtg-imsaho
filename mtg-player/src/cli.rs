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
    /// Pass until our next Main Phase 1 on a later turn.
    UntilNextTurn { activated_turn: u32 },
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

    // ── Pass mode logic ─────────────────────────────────────────────

    /// Check whether the current pass mode should break and return control
    /// to the player. Returns true if the player should be prompted.
    fn should_break_pass(
        view: &GameView,
        legal: &mtg_engine::engine::LegalActions,
        mode: &PassMode,
    ) -> bool {
        match mode {
            PassMode::UntilNextTurn { activated_turn } => {
                // Break at our precombat main on a later turn.
                if view.active_player == view.you
                    && view.turn_number > *activated_turn
                    && view.step == Step::PrecombatMain
                {
                    return true;
                }

                // Break on our turn if we have meaningful actions (cast spells,
                // play lands, activate non-mana abilities) — even outside main phase.
                if view.active_player == view.you
                    && view.turn_number > *activated_turn
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
                            view.battlefield.iter().find(|p| p.object_id == *id).map_or_else(|| " -> ?".into(), |p| format!(" -> {}", p.name))
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
        row = Self::render_battlefield_at(&mut out, &opp_perms, Color::Red, mid_col, row, mid_w, &view.battlefield, false);
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
        row = Self::render_battlefield_at(&mut out, &your_perms, Color::Green, mid_col, row, mid_w, &view.battlefield, true);

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
        if let Some(labels) = actions {
            for (i, label) in labels.iter().enumerate() {
                let _ = execute!(out, cursor::MoveTo(mid_col, row),
                    SetAttribute(Attribute::Bold), Print(format!("  {i}")),
                    SetAttribute(Attribute::Reset), Print(": "));
                Self::print_action_label(&mut out, label);
                row += 1;
            }
            let has_pass = labels.first().is_some_and(|l| l == "Pass priority");
            let hints = if has_pass {
                "  [enter=pass] [f=auto-pass] [/=search] [d=deck] [l=log] [g=gy] [e=exile]"
            } else {
                "  [/=search] [d=deck] [l=log] [g=gy] [e=exile]"
            };
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
    }

    /// Render battlefield permanents at a specific column/row, return next row.
    fn render_battlefield_at(out: &mut io::Stdout, perms: &[&PermanentView], color: Color,
                              col: u16, mut row: u16, max_w: usize,
                              all_perms: &[PermanentView], lands_last: bool) -> u16 {
        let has_type = |p: &&PermanentView, t: CardType| p.card_types.contains(&t);
        let lands: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Land)).collect();
        let creatures: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Creature)).collect();
        let enchantments: Vec<_> = perms.iter().filter(|p|
            has_type(p, CardType::Enchantment) && !has_type(p, CardType::Creature)).collect();
        let artifacts: Vec<_> = perms.iter().filter(|p|
            has_type(p, CardType::Artifact) && !has_type(p, CardType::Creature) && !has_type(p, CardType::Land)).collect();

        // Build aura map from ALL permanents (auras can be controlled by a different
        // player than the creature they're attached to, e.g. Pacifism).
        let mut aura_map: HashMap<ObjectId, Vec<String>> = HashMap::new();
        for p in all_perms {
            if p.attached_to.is_some() && p.card_types.contains(&CardType::Enchantment) {
                if let Some(target_id) = p.attached_to {
                    aura_map.entry(target_id).or_default().push(p.name.clone());
                }
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

        // Helper: render creatures, enchantments, artifacts
        let render_nonlands = |out: &mut io::Stdout, row: &mut u16| {
            for c in &creatures {
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
                let text = format!("  {}{}{}{}{}", c.name, pt, auras, dmg, flags);
                let truncated: String = text.chars().take(max_w).collect();
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(color), Print(&truncated), ResetColor);
                *row += 1;
            }
            for e in &enchantments {
                if e.attached_to.is_some() { continue; }
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(Color::Magenta), Print(format!("  {}", e.name)), ResetColor);
                *row += 1;
            }
            for a in &artifacts {
                let t = if a.tapped { " [T]" } else { "" };
                let _ = execute!(out, cursor::MoveTo(col, *row), Print(format!("  {}{}", a.name, t)));
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
                    KeyCode::Char(c) => {
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
            CastTargetSpec::TwoTargets(options1, options2) => {
                let t1 = Self::prompt_target(view,options1, &format!("{}: select first of two targets", spell.name))?;
                let remaining: Vec<_> = options2.iter().filter(|t| **t != t1).cloned().collect();
                if remaining.is_empty() {
                    return None;
                }
                let t2 = Self::prompt_target(view,&remaining, &format!("{}: select second of two targets", spell.name))?;
                vec![t1, t2]
            }
            CastTargetSpec::UpToTargets { max, options } => {
                let mut chosen = Vec::new();
                let mut remaining = options.clone();
                for i in 0..*max {
                    if remaining.is_empty() { break; }
                    let label = format!("{}: select target {} of up to {}",
                        spell.name, i + 1, max);
                    match Self::prompt_target_optional(view,&remaining, &label) {
                        Some(target) => {
                            remaining.retain(|t| *t != target);
                            chosen.push(target);
                        }
                        None => break,
                    }
                }
                if chosen.is_empty() {
                    return None;
                }
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
                let is_land = p.card_types.iter().all(|t| matches!(t, CardType::Land));
                if is_land {
                    format!("{}{}", p.name, pt)
                } else {
                    let owner = if p.controller == view.you { "your" } else { "opp" };
                    format!("{}{} ({})", p.name, pt, owner)
                }
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
            Action::ActivateAbility { object_id, .. } =>
                format!("Activate ability: {}", Self::perm_name(view, *object_id)),
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
            Action::ActivateLoyaltyAbility { object_id, ability_index, .. } =>
                format!("Activate loyalty ability {} on {}", ability_index, Self::perm_name(view, *object_id)),
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
        // A cooked-mode read while the terminal is still raw silently
        // swallows keystrokes ('\r' never ends the line, nothing echoes) —
        // the concede prompt ate three keypresses this way (issue #42).
        // Force cooked mode for the read; callers re-enable raw themselves.
        let _ = terminal::disable_raw_mode();
        print!("{prompt}");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
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
                    KeyCode::Char('y' | 'Y') => { let _ = execute!(stdout(), Print("y")); break true; }
                    KeyCode::Char('n' | 'N') | KeyCode::Esc => { let _ = execute!(stdout(), Print("n")); break false; }
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

        let result = loop {
            if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
                match code {
                    KeyCode::Char('/') if buf.is_empty() => {
                        break None; // trigger card search
                    }
                    KeyCode::Char('r') if buf.is_empty() => {
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
                        let _ = execute!(out, Print("r"));
                        let _ = out.flush();
                    }
                    KeyCode::Enter => {
                        break Some(buf.clone());
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        let _ = terminal::disable_raw_mode();
                        std::process::exit(0);
                    }
                    KeyCode::Backspace => {
                        if buf.pop().is_some() {
                            // Erase character on screen
                            let _ = execute!(out, Print("\x08 \x08"));
                            let _ = out.flush();
                        }
                    }
                    KeyCode::Char(c) => {
                        buf.push(c);
                        let _ = execute!(out, Print(c.to_string()));
                        let _ = out.flush();
                    }
                    _ => {}
                }
            }
        };

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
                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {idx:>2}")),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}{}{}\n", perm.name, pt, flags)));
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
                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {idx:>2}")),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}{}{}\n", perm.name, pt, flags)));
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

                    let controller = if perm.controller == view.you { "You" } else { "Opponent" };
                    let _ = execute!(out, Print(format!("  Controller: {controller}\n")));
                    let _ = execute!(out, Print(format!("  Tapped: {}\n", perm.tapped)));
                    let _ = execute!(out, Print(format!("  Summoning sick: {}\n", perm.summoning_sick)));
                    let _ = execute!(out, Print(format!("  ID: #{}\n", perm.object_id.0)));

                    // Show attached auras
                    let auras: Vec<&PermanentView> = view.battlefield.iter()
                        .filter(|p| p.attached_to == Some(perm.object_id))
                        .collect();
                    if !auras.is_empty() {
                        let _ = execute!(out, Print("  Enchanted by: "));
                        let names: Vec<&str> = auras.iter().map(|a| a.name.as_str()).collect();
                        let _ = execute!(out, Print(format!("{}\n", names.join(", "))));
                    }

                    if let Some(att) = perm.attached_to {
                        let att_name = view.battlefield.iter()
                            .find(|p| p.object_id == att)
                            .map_or("?", |p| p.name.as_str());
                        let _ = execute!(out, Print(format!("  Attached to: {att_name}\n")));
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

    fn show_log(log: &[String]) {
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
        Self::print_colored(&mut out, Color::Cyan, " GAME LOG");
        if log.is_empty() {
            let _ = execute!(out, Print("  (no events yet)\n"));
        } else {
            let h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
            let visible = h.saturating_sub(4); // leave room for header/footer
            let start = if log.len() > visible { log.len() - visible } else { 0 };
            for entry in &log[start..] {
                let _ = execute!(out, SetAttribute(Attribute::Dim),
                    Print(format!("  {entry}\n")), SetAttribute(Attribute::Reset));
            }
        }
        let _ = execute!(out, Print("\n  Press enter to return..."));
        let _ = out.flush();
        let _ = Self::read_line("");
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

        loop {
            let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
            Self::print_colored(&mut out, Color::Cyan,
                &format!(" YOUR DECK ({total_cards} cards)"));
            let _ = execute!(out, Print("\n"));

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

            for (i, data) in deck_cards.iter().enumerate() {
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

            let _ = execute!(out, Print("\n  Enter number for details, or press enter to return: "));
            let _ = out.flush();
            let input = Self::read_line("");

            if input.is_empty() { return; }

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

        loop {
            // Clear the row before re-prompting: a rejected entry's characters
            // otherwise stay on screen and visually merge with the next
            // attempt ("7" typed over stale "abc" reads as "7bc" — issue #35).
            let _ = execute!(stdout(), cursor::MoveTo(col, r), Clear(ClearType::UntilNewLine));
            let input = Self::read_line("  Attack (numbers/all/none)> ");

            if input == "none" || input == "n" {
                return Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] };
            }
            if input.is_empty() || input == "all" || input == "a" {
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
        let CombatPrompt::ChooseBlockers { eligible_blockers, attackers: attacker_ids, legal_blocks } = prompt else {
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
            let _ = execute!(out, cursor::MoveTo(col, r),
                Print(format!("  {}: {}", i, Self::perm_name(view, id))));
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
                        assignments.push((blocker, attacker));
                    }
                    _ => {
                        error = Some("Invalid. Use 'blocker:attacker' pairs like '0:0 1:1'.".into());
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

        println!("\n{description}");
        println!("Max X = {}", options.max_x);
        let pool_summary: Vec<String> = [
            ManaType::White, ManaType::Blue, ManaType::Black,
            ManaType::Red, ManaType::Green, ManaType::Colorless,
        ].iter().filter_map(|mt| {
            let n = options.pool.get(mt).copied().unwrap_or(0);
            if n > 0 { Some(format!("{n} {mt:?}")) } else { None }
        }).collect();
        if !pool_summary.is_empty() {
            println!("Pool: {}", pool_summary.join(", "));
        }
        for g in &options.groups {
            println!(
                "  {} x{} ({}/tap, max {})",
                g.name, g.source_ids.len(), g.mana_per_tap, g.max_contribution()
            );
        }
        let _ = view;

        let x: u32 = loop {
            let input = Self::read_line("X = ");
            if input.trim().is_empty() {
                break 0;
            }
            match input.trim().parse::<u32>() {
                Ok(n) if n <= options.max_x => break n,
                _ => println!("Enter an integer between 0 and {}.", options.max_x),
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
            println!(
                "(Note: could not allocate final {remaining} mana due to multi-mana source quanta; X = {})",
                x - remaining,
            );
        }
        Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) }
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

        println!("\n{description}");
        if min == max {
            println!("Choose exactly {min} card{}.", if min == 1 { "" } else { "s" });
        } else {
            println!("Choose between {min} and {max} cards.");
        }
        for (i, &id) in options.iter().enumerate() {
            println!("  [{i}] {}", Self::perm_name(view, id));
        }

        loop {
            let input = Self::read_line("indices (space-separated, blank = minimum): ");
            let trimmed = input.trim();
            let indices: Vec<usize> = if trimmed.is_empty() {
                (0..min).collect()
            } else {
                let parsed: Result<Vec<usize>, _> = trimmed.split_whitespace()
                    .map(str::parse::<usize>)
                    .collect();
                let Ok(v) = parsed else {
                    println!("Invalid input.");
                    continue;
                };
                v
            };
            if indices.iter().any(|&i| i >= options.len()) {
                println!("Index out of range.");
                continue;
            }
            let mut sorted = indices.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.len() != indices.len() {
                println!("Duplicate indices.");
                continue;
            }
            if indices.len() < min || indices.len() > max {
                println!("Need between {min} and {max} indices.");
                continue;
            }
            let chosen: Vec<mtg_engine::ids::ObjectId> = indices.into_iter()
                .map(|i| options[i])
                .collect();
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(chosen) };
        }
    }

    fn library_search_ui(view: &GameView, actions: &[Action]) -> Action {
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
            let _ = execute!(out, SetForegroundColor(Color::Yellow),
                Print("═══ Search Library ═══\n\r"),
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
                    KeyCode::Char(c) => {
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

        // Special case: library search — show interactive card browser.
        if legal_actions.iter().all(|a| matches!(a, Action::ResolveChoice { .. }))
            && legal_actions.len() > 1
        {
            // Check if these are ChosenCard choices (library/revealed search).
            let all_chosen_cards = legal_actions.iter().all(|a| matches!(a,
                Action::ResolveChoice { choice: mtg_engine::actions::ResolvedChoice::ChosenCard(_) }
            ));
            if all_chosen_cards && legal_actions.len() > 3 {
                return Self::library_search_ui(view, legal_actions);
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

        loop {
            let pass_label = self.pass_mode.as_ref().map(|m| match m {
                PassMode::UntilNextTurn { .. } => "AUTO-PASS",
            });
            Self::render(view, Some(&display_labels), legal.context.as_deref(),
                &view.display_log, &self.card_filter, pass_label);

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
                    let mut out = stdout();
                    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
                    Self::print_colored(&mut out, Color::Cyan, " GRAVEYARDS");
                    let _ = execute!(out, Print("\n"));
                    for (pid, cards) in &view.graveyards {
                        let who = if *pid == view.you { "Your" } else { "Opponent's" };
                        let _ = execute!(out,
                            SetAttribute(Attribute::Bold),
                            Print(format!(" {} graveyard ({}):\n", who, cards.len())),
                            SetAttribute(Attribute::Reset));
                        if cards.is_empty() {
                            let _ = execute!(out, Print("   (empty)\n"));
                        } else {
                            for card in cards {
                                let cost = card.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
                                let pt = match (card.power, card.toughness) {
                                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                                    _ => String::new(),
                                };
                                let _ = execute!(out, Print("   "));
                                Self::print_with_mana(&mut out, &format!("{}{}{}", card.name, cost, pt), None);
                                let _ = execute!(out, Print("\n"));
                            }
                        }
                        let _ = execute!(out, Print("\n"));
                    }
                    let _ = execute!(out, Print("  Press enter to return..."));
                    let _ = out.flush();
                    let _ = Self::read_line("");
                    continue;
                }
                "e" => {
                    let mut out = stdout();
                    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
                    Self::print_colored(&mut out, Color::Cyan, " EXILE");
                    let _ = execute!(out, Print("\n"));
                    let your_exile: Vec<_> = view.exile.iter().filter(|c| c.owner == view.you).collect();
                    let opp_exile: Vec<_> = view.exile.iter().filter(|c| c.owner != view.you).collect();
                    let _ = execute!(out, SetAttribute(Attribute::Bold),
                        Print(format!(" Your exile ({}):\n", your_exile.len())),
                        SetAttribute(Attribute::Reset));
                    if your_exile.is_empty() {
                        let _ = execute!(out, Print("   (empty)\n"));
                    } else {
                        for card in &your_exile {
                            let cost = card.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
                            let pt = match (card.power, card.toughness) {
                                (Some(p), Some(t)) => format!(" {p}/{t}"),
                                _ => String::new(),
                            };
                            let _ = execute!(out, Print(format!("   {}{}{}\n", card.name, cost, pt)));
                        }
                    }
                    let _ = execute!(out, Print("\n"));
                    let _ = execute!(out, SetAttribute(Attribute::Bold),
                        Print(format!(" Opponent's exile ({}):\n", opp_exile.len())),
                        SetAttribute(Attribute::Reset));
                    if opp_exile.is_empty() {
                        let _ = execute!(out, Print("   (empty)\n"));
                    } else {
                        for card in &opp_exile {
                            let cost = card.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
                            let pt = match (card.power, card.toughness) {
                                (Some(p), Some(t)) => format!(" {p}/{t}"),
                                _ => String::new(),
                            };
                            let _ = execute!(out, Print(format!("   {}{}{}\n", card.name, cost, pt)));
                        }
                    }
                    let _ = execute!(out, Print("\n  Press enter to return..."));
                    let _ = out.flush();
                    let _ = Self::read_line("");
                    continue;
                }
                "i" => {
                    Self::show_battlefield_inspector(view);
                    continue;
                }
                "f" => {
                    // Pass until my next Main Phase 1 (F6-like).
                    if has_pass {
                        self.pass_mode = Some(PassMode::UntilNextTurn {
                            activated_turn: view.turn_number,
                        });
                        return Action::PassPriority;
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
                "" => {
                    // Enter = pass if available
                    if has_pass {
                        return Action::PassPriority;
                    }
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
                }
            }
            // Invalid input — just re-render
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

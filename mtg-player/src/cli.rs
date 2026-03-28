use std::collections::HashMap;
use std::io::{self, Write, stdout};

use crossterm::{
    cursor, execute,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Color, SetForegroundColor, SetAttribute, Attribute, ResetColor, Print},
    terminal::{self, Clear, ClearType},
};

use mtg_engine::actions::{Action, CombatPrompt, Target};
use mtg_engine::types::Step;
use mtg_engine::ids::ObjectId;
use mtg_engine::types::CardType;
use mtg_engine::view::{GameView, CardView, PermanentView};

use crate::Player;

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
pub struct CliPlayer {
    name: String,
    /// When set, auto-pass priority until our next turn.
    /// Stores the turn number when 'f' was pressed.
    pass_until_turn_after: Option<u32>,
    /// Filter string for the card reference panel.
    card_filter: String,
}

impl CliPlayer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pass_until_turn_after: None,
            card_filter: String::new(),
        }
    }

    // ── Rendering ──────────────────────────────────────────────────

    fn render(view: &GameView, actions: Option<&[Action]>, message: Option<&str>, log: &[String], card_filter: &str) {
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
        let mid_col = (left_w + 1) as u16;
        let right_sep_col = (left_w + 1 + mid_w) as u16;
        let right_col = if has_right { right_sep_col + 1 } else { 0 };

        // ── Draw vertical separators ──
        for r in 0..h {
            let _ = execute!(out, cursor::MoveTo(left_w as u16, r as u16),
                SetAttribute(Attribute::Dim), Print("│"), SetAttribute(Attribute::Reset));
        }
        if has_right {
            for r in 0..h {
                let _ = execute!(out, cursor::MoveTo(right_sep_col, r as u16),
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
        let _ = execute!(out, cursor::MoveTo(left_w as u16, 0),
            SetAttribute(Attribute::Dim), Print("┤"), SetAttribute(Attribute::Reset));
        if view.stack.is_empty() {
            let _ = execute!(out, cursor::MoveTo(1, 1),
                SetAttribute(Attribute::Dim), Print("(empty)"), SetAttribute(Attribute::Reset));
        } else {
            let mut srow: u16 = 1;
            for item in view.stack.iter() {
                if srow >= stack_h as u16 { break; }
                let who = if item.controller == view.you { "you" } else { "opp" };
                let text = format!("{} ({})", item.name, who);
                let truncated: String = text.chars().take(left_w.saturating_sub(1)).collect();
                let _ = execute!(out, cursor::MoveTo(1, srow),
                    Print(&truncated));
                srow += 1;
                for target in &item.targets {
                    if srow >= stack_h as u16 { break; }
                    let target_name = match target {
                        mtg_engine::actions::Target::Object(id) => {
                            view.battlefield.iter().find(|p| p.object_id == *id)
                                .map(|p| format!(" -> {}", p.name))
                                .unwrap_or_else(|| " -> ?".into())
                        }
                        mtg_engine::actions::Target::Player(pid) => {
                            if *pid == view.you { " -> you".into() } else { " -> opp".into() }
                        }
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
        let _ = execute!(out, cursor::MoveTo(0, log_start as u16),
            SetAttribute(Attribute::Dim), Print(&log_line), SetAttribute(Attribute::Reset));
        let _ = execute!(out, cursor::MoveTo(left_w as u16, log_start as u16),
            SetAttribute(Attribute::Dim), Print("┤"), SetAttribute(Attribute::Reset));
        if !log.is_empty() {
            let log_visible = h.saturating_sub(log_start + 2);
            let start = if log.len() > log_visible { log.len() - log_visible } else { 0 };
            for (i, entry) in log[start..].iter().enumerate() {
                let r = (log_start + 1 + i) as u16;
                if r >= term_h - 1 { break; }
                let truncated: String = entry.chars().take(left_w.saturating_sub(1)).collect();
                let _ = execute!(out, cursor::MoveTo(1, r),
                    SetAttribute(Attribute::Dim), Print(&truncated), SetAttribute(Attribute::Reset));
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
        let status = format!(" Turn {} - {} | {}", view.turn_number, step_name, whose_turn);
        let _ = execute!(out, cursor::MoveTo(mid_col, row),
            SetAttribute(Attribute::Bold), Print(&status), SetAttribute(Attribute::Reset));
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
        let opp_caret = if !is_your_turn { "▸ " } else { "  " };

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
        let _ = execute!(out, cursor::MoveTo(left_w as u16, row),
            SetAttribute(Attribute::Dim), Print("├"), SetAttribute(Attribute::Reset));
        row += 1;

        // Opponent status line
        let _ = execute!(out, cursor::MoveTo(mid_col, row));
        if !is_your_turn {
            let _ = execute!(out, SetForegroundColor(Color::Red), SetAttribute(Attribute::Bold));
        } else {
            let _ = execute!(out, SetForegroundColor(Color::Red));
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
            (if lands.is_empty() { 0 } else { 1 }) + creatures.len() + other
        };

        // Pad the divider area so the total battlefield is at least 9 lines
        // (opp_status + opp_board + divider + your_board + your_status = content)
        let min_bf_height: u16 = 9;
        let content_rows = 2 + opp_rows + your_row_count as u16; // 2 for status lines
        let padding = if content_rows + 1 < min_bf_height {
            (min_bf_height - content_rows) as u16
        } else {
            1 // at least 1 line for the divider
        };

        // Draw divider with padding
        let divider_mid = row + padding / 2;
        let dots = "· · ·";
        let dots_pad = mid_w.saturating_sub(dots.chars().count()) / 2;
        let _ = execute!(out, cursor::MoveTo(mid_col + dots_pad as u16, divider_mid),
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
        let _ = execute!(out, cursor::MoveTo(left_w as u16, row),
            SetAttribute(Attribute::Dim), Print("├"), SetAttribute(Attribute::Reset));
        row += 1;

        // Hand
        if view.your_hand.is_empty() {
            Self::mid_print(&mut out, mid_col, &mut row, mid_w, "  (empty)", None, false);
        } else {
            for card in &view.your_hand {
                let cost = card.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
                let pt = match (card.power, card.toughness) {
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                    _ => String::new(),
                };
                Self::mid_print(&mut out, mid_col, &mut row, mid_w,
                    &format!("  {}{}{}", card.name, cost, pt), None, false);
            }
        }

        // Mana pool at bottom of hand area
        if !view.your_mana_pool.is_empty() {
            let mana_str: Vec<String> = view.your_mana_pool.mana.iter()
                .filter(|(_, &v)| v > 0)
                .map(|(t, v)| format!("{:?}:{}", t, v))
                .collect();
            Self::mid_print(&mut out, mid_col, &mut row, mid_w,
                &format!("  Mana: {}", mana_str.join(" ")), Some(Color::Yellow), false);
        }

        // Message
        if let Some(msg) = message {
            Self::mid_print(&mut out, mid_col, &mut row, mid_w, &format!(" {}", msg), Some(Color::Yellow), true);
        }

        // Actions separator (always drawn)
        let mid_sep: String = "─".repeat(mid_w);
        let _ = execute!(out, cursor::MoveTo(mid_col, row),
            SetAttribute(Attribute::Dim), Print(&mid_sep), SetAttribute(Attribute::Reset));
        let _ = execute!(out, cursor::MoveTo(left_w as u16, row),
            SetAttribute(Attribute::Dim), Print("├"), SetAttribute(Attribute::Reset));
        row += 1;

        // Action list (only when actions are provided)
        if let Some(actions) = actions {
            for (i, action) in actions.iter().enumerate() {
                let desc = Self::format_action(view, action);
                let _ = execute!(out, cursor::MoveTo(mid_col, row),
                    SetAttribute(Attribute::Bold), Print(format!("  {}", i)),
                    SetAttribute(Attribute::Reset), Print(format!(": {}", desc)));
                row += 1;
            }
            let has_pass = actions.first().map(|a| matches!(a, Action::PassPriority)).unwrap_or(false);
            let hints = if has_pass {
                "  [enter=pass] [f=pass turn] [/=search] [d=deck] [l=log] [g=gy] [e=exile]"
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
                let parts: Vec<String> = summary.iter().map(|(name, untapped, tapped)| {
                    let total = untapped + tapped;
                    if *tapped == 0 { format!("{}x {}", total, name) }
                    else if *untapped == 0 { format!("{}x {} (tapped)", total, name) }
                    else { format!("{}x {} ({} tapped)", total, name, tapped) }
                }).collect();
                let text = format!("  Lands: {}", parts.join(", "));
                let truncated: String = text.chars().take(max_w).collect();
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(color), Print(&truncated), ResetColor);
                *row += 1;
            }
        };

        // Helper: render creatures, enchantments, artifacts
        let render_nonlands = |out: &mut io::Stdout, row: &mut u16| {
            for c in &creatures {
                let pt = match (c.effective_power, c.effective_toughness) {
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                    _ => match (c.power, c.toughness) {
                        (Some(p), Some(t)) => format!(" {}/{}", p, t),
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
                let id_tag = format!(" #{}", c.object_id.0);
                let text = format!("  {}{}{}{}{}{}", c.name, pt, auras, dmg, flags, id_tag);
                let truncated: String = text.chars().take(max_w).collect();
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(color), Print(&truncated), ResetColor);
                *row += 1;
            }
            for e in &enchantments {
                if e.attached_to.is_some() { continue; }
                let _ = execute!(out, cursor::MoveTo(col, *row),
                    SetForegroundColor(Color::Magenta), Print(format!("  {} #{}", e.name, e.object_id.0)), ResetColor);
                *row += 1;
            }
            for a in &artifacts {
                let t = if a.tapped { " [T]" } else { "" };
                let _ = execute!(out, cursor::MoveTo(col, *row), Print(format!("  {}{} #{}", a.name, t, a.object_id.0)));
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
        if let Some(c) = color { let _ = execute!(out, SetForegroundColor(c)); }
        if bold { let _ = execute!(out, SetAttribute(Attribute::Bold)); }
        let truncated: String = text.chars().take(max_w).collect();
        let _ = execute!(out, Print(&truncated));
        if bold { let _ = execute!(out, SetAttribute(Attribute::Reset)); }
        if color.is_some() { let _ = execute!(out, ResetColor); }
        *row += 1;
    }

    fn print_colored(out: &mut impl Write, color: Color, text: &str) {
        let _ = execute!(out, SetForegroundColor(color), SetAttribute(Attribute::Bold),
            Print(format!("{}\n", text)), SetAttribute(Attribute::Reset), ResetColor);
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
        // Priority 3: opponent's battlefield (non-land)
        for p in view.battlefield.iter().filter(|p| p.controller != view.you) {
            if p.card_types.iter().all(|t| matches!(t, CardType::Land)) { continue; }
            add(&p.name, p.card_id);
        }
        // Priority 4: your battlefield (non-land)
        for p in view.battlefield.iter().filter(|p| p.controller == view.you) {
            if p.card_types.iter().all(|t| matches!(t, CardType::Land)) { continue; }
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
            let text = format!(" /{}", filter);
            let pad = right_w.saturating_sub(text.chars().count());
            format!("{}{}", text, " ".repeat(pad))
        };
        let _ = execute!(out, cursor::MoveTo(right_col, 1),
            SetAttribute(Attribute::Dim), Print(&search_display), SetAttribute(Attribute::Reset));

        let content_w = right_w.saturating_sub(1); // text margin

        let mut row: u16 = 2; // cards start below search box
        let max_row = (h as u16).saturating_sub(1);

        for card in cards {
            if row >= max_row { break; }

            // Name + cost
            let cost_str = card.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
            let name_line = format!("{}{}", card.name, cost_str);
            let truncated: String = name_line.chars().take(content_w).collect();
            let _ = execute!(out, cursor::MoveTo(right_col, row),
                SetAttribute(Attribute::Bold), Print(&truncated), SetAttribute(Attribute::Reset));
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
                (Some(p), Some(t)) => format!(" {}/{}", p, t),
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
                        let _ = execute!(out, cursor::MoveTo(right_col, row), Print(&line));
                        row += 1;
                    }
                }
            }

            // Flashback cost
            if let Some(fb) = &card.flashback_cost {
                if row < max_row {
                    let fb_line = format!("Flashback {}", fb);
                    let truncated: String = fb_line.chars().take(content_w).collect();
                    let _ = execute!(out, cursor::MoveTo(right_col, row),
                        SetForegroundColor(Color::Cyan), Print(&truncated), ResetColor);
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
    fn run_card_search(&mut self, view: &GameView, actions: &[Action]) {
        let _ = terminal::enable_raw_mode();

        self.card_filter.clear();

        loop {
            // Re-render with current filter
            Self::render(view, Some(actions), None, &view.display_log, &self.card_filter);

            // Move actual cursor to the search box in the right gutter
            let (term_w, _) = terminal::size().unwrap_or((100, 30));
            let w = term_w as usize;
            let gutter_w = w / 5;
            let mid_w = w.saturating_sub(gutter_w * 2 + 2);
            let right_col = (gutter_w + 1 + mid_w + 1) as u16;
            let cursor_x = right_col + 2 + self.card_filter.chars().count() as u16;
            let _ = execute!(stdout(), cursor::MoveTo(cursor_x, 1));
            let _ = stdout().flush();

            // Read one key event
            if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
                match code {
                    KeyCode::Esc | KeyCode::Enter => {
                        self.card_filter.clear();
                        break;
                    }
                    KeyCode::Char('/') => {
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

    // ── Action formatting ──────────────────────────────────────────

    fn perm_name(view: &GameView, id: ObjectId) -> String {
        view.battlefield.iter()
            .find(|p| p.object_id == id)
            .map(|p| {
                let pt = match (p.effective_power, p.effective_toughness) {
                    (Some(pw), Some(t)) => format!(" {}/{}", pw, t),
                    _ => String::new(),
                };
                format!("{}{}", p.name, pt)
            })
            .or_else(|| view.your_hand.iter()
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.stack.iter()
                .find(|s| s.object_id == id)
                .map(|s| s.name.clone()))
            .unwrap_or_else(|| format!("{}", id))
    }

    fn format_action(view: &GameView, action: &Action) -> String {
        match action {
            Action::PassPriority => "Pass priority".into(),
            Action::PlayLand { object_id } =>
                format!("Play land {}", Self::perm_name(view, *object_id)),
            Action::CastSpell { object_id, targets, .. } => {
                let name = Self::perm_name(view, *object_id);
                if targets.is_empty() {
                    format!("Cast {}", name)
                } else {
                    let target_names: Vec<String> = targets.iter().map(|t| match t {
                        Target::Object(id) => Self::perm_name(view, *id),
                        Target::Player(pid) => {
                            if *pid == view.you { "you".into() } else { "opponent".into() }
                        }
                    }).collect();
                    format!("Cast {} -> {}", name, target_names.join(", "))
                }
            }
            Action::ActivateManaAbility { object_id, .. } =>
                format!("Tap {} for mana", Self::perm_name(view, *object_id)),
            Action::DeclareAttackers { attackers } => {
                if attackers.is_empty() { "Don't attack".into() }
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
            Action::Concede => "Concede".into(),
        }
    }

    // ── Input ──────────────────────────────────────────────────────

    fn read_line(prompt: &str) -> String {
        print!("{}", prompt);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
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
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                    _ => String::new(),
                };
                let flags = format!("{}{}",
                    if perm.tapped { " [T]" } else { "" },
                    if perm.summoning_sick { " [S]" } else { "" });
                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {:>2}", idx)),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}{}{} #{}\n", perm.name, pt, flags, perm.object_id.0)));
                idx += 1;
            }

            let _ = execute!(out, Print("\n"));
            let _ = execute!(out, SetAttribute(Attribute::Bold),
                Print(" Opponent's permanents:\n"), SetAttribute(Attribute::Reset));
            for perm in &opp_perms {
                let pt = match (perm.effective_power, perm.effective_toughness) {
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                    _ => String::new(),
                };
                let flags = format!("{}{}",
                    if perm.tapped { " [T]" } else { "" },
                    if perm.summoning_sick { " [S]" } else { "" });
                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {:>2}", idx)),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}{}{} #{}\n", perm.name, pt, flags, perm.object_id.0)));
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
                        let _ = execute!(out, Print(format!("  Base P/T: {}/{}\n", p, t)));
                    }
                    if let (Some(p), Some(t)) = (perm.effective_power, perm.effective_toughness) {
                        let _ = execute!(out, Print(format!("  Effective P/T: {}/{}\n", p, t)));
                    }
                    if perm.damage_marked > 0 {
                        let _ = execute!(out, Print(format!("  Damage marked: {}\n", perm.damage_marked)));
                    }

                    let controller = if perm.controller == view.you { "You" } else { "Opponent" };
                    let _ = execute!(out, Print(format!("  Controller: {}\n", controller)));
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
                            .map(|p| p.name.as_str())
                            .unwrap_or("?");
                        let _ = execute!(out, Print(format!("  Attached to: {}\n", att_name)));
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
            for (_i, entry) in log[start..].iter().enumerate() {
                let _ = execute!(out, SetAttribute(Attribute::Dim),
                    Print(format!("  {}\n", entry)), SetAttribute(Attribute::Reset));
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
                &format!(" YOUR DECK ({} cards)", total_cards));
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
                let cost = data.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
                let pt = match (data.power, data.toughness) {
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
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
                if h > 0 { locs.push(format!("{}hand", h)); }
                if b > 0 { locs.push(format!("{}board", b)); }
                if g > 0 { locs.push(format!("{}gy", g)); }
                if e > 0 { locs.push(format!("{}exile", e)); }
                if lib > 0 { locs.push(format!("{}lib", lib)); }
                let loc_str = if locs.is_empty() { String::new() } else { format!(" ({})", locs.join(", ")) };

                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {:>2}", i)),
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
                    let cost = data.cost.as_ref().map(|c| format!("{}", c)).unwrap_or_else(|| "(none)".into());
                    let _ = execute!(out, Print(format!("  Mana cost: {}\n", cost)));
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
                        let _ = execute!(out, Print(format!("  Power/Toughness: {}/{}\n", p, t)));
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
                            Print(format!("  Flashback: {}\n", fb)), ResetColor);
                    }
                    let _ = execute!(out, Print("\n  Press enter to return to list..."));
                    let _ = out.flush();
                    let _ = Self::read_line("");
                }
            }
        }
    }

    // ── Combat ─────────────────────────────────────────────────────

    fn choose_attackers(&self, view: &GameView, prompt: &CombatPrompt) -> Action {
        let (eligible, defending) = match prompt {
            CombatPrompt::ChooseAttackers { eligible, defending_player } => (eligible, *defending_player),
            _ => unreachable!(),
        };

        if eligible.is_empty() {
            return Action::DeclareAttackers { attackers: vec![] };
        }

        Self::render(view, None, Some("DECLARE ATTACKERS"), &view.display_log, "");

        // Get mid_col for positioning
        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = (side + 1) as u16;
        let cur_row = cursor::position().unwrap_or((0, 20)).1;

        let mut out = stdout();
        let mut r = cur_row;
        let _ = execute!(out, cursor::MoveTo(col, r),
            SetForegroundColor(Color::Yellow), SetAttribute(Attribute::Bold),
            Print(" Eligible attackers:"), SetAttribute(Attribute::Reset), ResetColor);
        r += 1;
        for (i, &id) in eligible.iter().enumerate() {
            let _ = execute!(out, cursor::MoveTo(col, r),
                SetAttribute(Attribute::Bold), Print(format!("  {}", i)),
                SetAttribute(Attribute::Reset), Print(format!(": {}", Self::perm_name(view, id))));
            r += 1;
        }
        let _ = execute!(out, cursor::MoveTo(col, r));
        let _ = out.flush();

        loop {
            let _ = execute!(stdout(), cursor::MoveTo(col, r));
            let input = Self::read_line("  Attack (numbers/all/enter=none)> ");

            if input.is_empty() {
                return Action::DeclareAttackers { attackers: vec![] };
            }
            if input == "all" {
                return Action::DeclareAttackers {
                    attackers: eligible.iter().map(|&id| (id, defending)).collect(),
                };
            }

            let indices: Vec<usize> = input.split_whitespace()
                .filter_map(|s| s.parse().ok()).collect();
            if indices.iter().all(|&i| i < eligible.len()) {
                return Action::DeclareAttackers {
                    attackers: indices.iter().map(|&i| (eligible[i], defending)).collect(),
                };
            }
            println!("  Invalid. Enter numbers like '0 2', 'all', or press enter.");
        }
    }

    fn choose_blockers(&self, view: &GameView, prompt: &CombatPrompt) -> Action {
        let (eligible_blockers, attacker_ids) = match prompt {
            CombatPrompt::ChooseBlockers { eligible_blockers, attackers } => (eligible_blockers, attackers),
            _ => unreachable!(),
        };

        if eligible_blockers.is_empty() {
            return Action::DeclareBlockers { assignments: vec![] };
        }

        Self::render(view, None, Some("DECLARE BLOCKERS"), &view.display_log, "");

        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = (side + 1) as u16;
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
            let _ = execute!(out, cursor::MoveTo(col, r),
                Print(format!("  {}: {}", i, Self::perm_name(view, id))));
            r += 1;
        }
        let _ = execute!(out, cursor::MoveTo(col, r));
        let _ = out.flush();

        loop {
            let _ = execute!(stdout(), cursor::MoveTo(col, r));
            let input = Self::read_line("  Block (blocker->attacker / enter=none)> ");

            if input.is_empty() {
                return Action::DeclareBlockers { assignments: vec![] };
            }

            let mut assignments = Vec::new();
            let mut valid = true;
            for pair in input.split_whitespace() {
                let parts: Vec<&str> = pair.split("->").collect();
                if parts.len() != 2 { valid = false; break; }
                match (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    (Ok(b), Ok(a)) if b < eligible_blockers.len() && a < attacker_ids.len() => {
                        assignments.push((eligible_blockers[b], attacker_ids[a]));
                    }
                    _ => { valid = false; break; }
                }
            }

            if valid {
                return Action::DeclareBlockers { assignments };
            }
            println!("  Invalid. Use '0->0 1->1' format.");
        }
    }
}

impl Player for CliPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, view: &GameView, legal_actions: &[Action]) -> Action {
        let has_pass = legal_actions.iter().any(|a| matches!(a, Action::PassPriority));

        // Auto-pass when the only options are Pass and Concede.
        let only_pass_concede = legal_actions.iter().all(|a| matches!(a,
            Action::PassPriority | Action::Concede
        ));
        if only_pass_concede && has_pass {
            return Action::PassPriority;
        }

        // "Pass until my turn" mode (F6-like).
        if let Some(activated_turn) = self.pass_until_turn_after {
            if has_pass {
                // Break if it's our turn AND we're on a later turn than when we pressed 'f'.
                let is_new_turn = view.active_player == view.you
                    && view.turn_number > activated_turn;
                // Break if something is on the stack (opponent cast a spell we can respond to).
                let stack_has_spell = !view.stack.is_empty();

                if is_new_turn || stack_has_spell {
                    self.pass_until_turn_after = None;
                } else {
                    return Action::PassPriority;
                }
            }
        }

        loop {
            Self::render(view, Some(legal_actions), None, &view.display_log, &self.card_filter);

            // Read input: render already positioned cursor after "  > " prompt.
            // Use raw mode to detect '/' immediately for card search.
            let (term_w, _) = terminal::size().unwrap_or((100, 30));
            let side = term_w as usize / 5;
            let col = (side + 1) as u16;
            let input = Self::read_line_with_search(col);

            // '/' triggers card search immediately (returns None to re-render)
            if input.is_none() {
                self.run_card_search(view, legal_actions);
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
                                let cost = card.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
                                let pt = match (card.power, card.toughness) {
                                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                                    _ => String::new(),
                                };
                                let _ = execute!(out, Print(format!("   {}{}{}\n", card.name, cost, pt)));
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
                            let cost = card.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
                            let pt = match (card.power, card.toughness) {
                                (Some(p), Some(t)) => format!(" {}/{}", p, t),
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
                            let cost = card.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
                            let pt = match (card.power, card.toughness) {
                                (Some(p), Some(t)) => format!(" {}/{}", p, t),
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
                    // Pass until my next turn (F6-like).
                    if has_pass {
                        self.pass_until_turn_after = Some(view.turn_number);
                        return Action::PassPriority;
                    }
                    continue;
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
                if idx < legal_actions.len() {
                    if matches!(legal_actions[idx], Action::Concede) {
                        let _ = execute!(stdout(), cursor::MoveTo(col, cursor::position().unwrap_or((0, 24)).1));
                        let confirm = Self::read_line("  Are you sure you want to concede? (y/n)> ");
                        if confirm.to_lowercase() != "y" {
                            continue;
                        }
                    }
                    return legal_actions[idx].clone();
                }
            }
            // Invalid input — just re-render
        }
    }

    fn choose_cards_to_bottom(
        &mut self,
        _view: &GameView,
        hand: &[CardView],
        count: usize,
    ) -> Vec<ObjectId> {
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
        Self::print_colored(&mut out, Color::Yellow,
            &format!(" Choose {} card(s) to put on bottom:", count));
        for (i, card) in hand.iter().enumerate() {
            let _ = execute!(out, Print(format!("  {}: {}\n", i, card.name)));
        }
        let _ = out.flush();

        loop {
            let input = Self::read_line(&format!("  Enter {} numbers> ", count));
            let indices: Vec<usize> = input.split_whitespace()
                .filter_map(|s| s.parse().ok()).collect();
            if indices.len() == count && indices.iter().all(|&i| i < hand.len()) {
                return indices.iter().map(|&i| hand[i].object_id).collect();
            }
            println!("  Invalid selection.");
        }
    }
}

impl CliPlayer {
    /// Show the game state with a spinning indicator on the opponent's
    /// caret while the AI thinks. Drop the returned handle to stop.
    pub fn start_thinking(view: &GameView) -> SpinnerHandle {
        Self::render(view, None, None, &view.display_log, "");

        let (term_w, _) = terminal::size().unwrap_or((100, 30));
        let side = term_w as usize / 5;
        let col = (side + 1) as u16;
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
            CombatPrompt::ChooseAttackers { .. } => {
                if self.pass_until_turn_after.is_some() {
                    return Action::DeclareAttackers { attackers: vec![] };
                }
                self.choose_attackers(view, prompt)
            }
            CombatPrompt::ChooseBlockers { .. } => {
                self.pass_until_turn_after = None;
                self.choose_blockers(view, prompt)
            }
        }
    }
}

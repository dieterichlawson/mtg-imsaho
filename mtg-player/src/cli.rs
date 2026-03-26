use std::collections::HashMap;
use std::io::{self, Write, stdout};

use crossterm::{
    cursor, execute,
    style::{Color, SetForegroundColor, SetAttribute, Attribute, ResetColor, Print},
    terminal::{self, Clear, ClearType},
};

use mtg_engine::actions::{Action, CombatPrompt, Target};
use mtg_engine::ids::ObjectId;
use mtg_engine::types::CardType;
use mtg_engine::view::{GameView, CardView, PermanentView};

use crate::Player;

/// A player that interacts via a terminal UI.
pub struct CliPlayer {
    name: String,
    /// When set, auto-pass priority until our next turn.
    /// Stores the turn number when 'f' was pressed.
    pass_until_turn_after: Option<u32>,
    /// Rolling game log of significant events.
    log: Vec<String>,
    /// Previous view for diffing.
    last_view: Option<GameView>,
    /// Scroll offset for log viewer.
    log_scroll: usize,
}

impl CliPlayer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pass_until_turn_after: None,
            log: Vec::new(),
            last_view: None,
            log_scroll: 0,
        }
    }

    /// Compare current view to previous view and log significant changes.
    fn update_log(&mut self, view: &GameView) {
        if let Some(prev) = &self.last_view {
            // Turn changes (log first so they appear as headers)
            if view.turn_number != prev.turn_number {
                let whose = if view.active_player == view.you { "Your" } else { "Opp's" };
                self.log.push(format!("── Turn {} ({}) ──", view.turn_number, whose));
            }

            // Life changes
            if view.your_life != prev.your_life {
                let diff = view.your_life - prev.your_life;
                if diff > 0 {
                    self.log.push(format!("You gained {} life ({})", diff, view.your_life));
                } else {
                    self.log.push(format!("You took {} damage ({})", -diff, view.your_life));
                }
            }
            for (opp, prev_opp) in view.opponents.iter().zip(prev.opponents.iter()) {
                if opp.life != prev_opp.life {
                    let diff = opp.life - prev_opp.life;
                    if diff > 0 {
                        self.log.push(format!("Opp gained {} life ({})", diff, opp.life));
                    } else {
                        self.log.push(format!("Opp took {} damage ({})", -diff, opp.life));
                    }
                }
            }

            // New items on stack (cast)
            for item in &view.stack {
                if !prev.stack.iter().any(|s| s.object_id == item.object_id) {
                    let who = if item.controller == view.you { "You" } else { "Opp" };
                    self.log.push(format!("{} cast {}", who, item.name));
                }
            }

            // Items resolved from stack
            for prev_item in &prev.stack {
                if !view.stack.iter().any(|s| s.object_id == prev_item.object_id) {
                    self.log.push(format!("{} resolved", prev_item.name));
                }
            }

            // New permanents — only log non-creatures (lands, enchantments, artifacts).
            // Creatures are logged via cast + resolved above.
            for perm in &view.battlefield {
                if !prev.battlefield.iter().any(|p| p.object_id == perm.object_id) {
                    if perm.power.is_none() {
                        let who = if perm.controller == view.you { "You" } else { "Opp" };
                        self.log.push(format!("{} played {}", who, perm.name));
                    }
                }
            }

            // Permanents that left the battlefield
            for prev_perm in &prev.battlefield {
                if !view.battlefield.iter().any(|p| p.object_id == prev_perm.object_id) {
                    self.log.push(format!("{} left battlefield", prev_perm.name));
                }
            }
        }
        self.last_view = Some(view.clone());
    }

    // ── Rendering ──────────────────────────────────────────────────

    fn render(view: &GameView, actions: Option<&[Action]>, message: Option<&str>, log: &[String]) {
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

        let (term_w, term_h) = terminal::size().unwrap_or((100, 30));
        let w = term_w as usize;
        let h = term_h as usize;

        // Panel widths — stack and log are the same width
        let side_w: usize = w / 5;
        let mid_w = w.saturating_sub(side_w * 2 + 2); // middle, -2 for separators
        let mid_col = (side_w + 1) as u16;
        let log_col = (side_w + 1 + mid_w + 1) as u16;

        // ── Draw vertical separators ──
        for row in 0..h {
            let _ = execute!(out, cursor::MoveTo(side_w as u16, row as u16),
                SetAttribute(Attribute::Dim), Print("│"), SetAttribute(Attribute::Reset));
            let _ = execute!(out, cursor::MoveTo(log_col - 1, row as u16),
                SetAttribute(Attribute::Dim), Print("│"), SetAttribute(Attribute::Reset));
        }

        // ── Left panel: STACK ──
        let _ = execute!(out, cursor::MoveTo(0, 0),
            SetForegroundColor(Color::Cyan), SetAttribute(Attribute::Bold),
            Print(" STACK"), SetAttribute(Attribute::Reset), ResetColor);
        if view.stack.is_empty() {
            let _ = execute!(out, cursor::MoveTo(1, 1),
                SetAttribute(Attribute::Dim), Print("(empty)"), SetAttribute(Attribute::Reset));
        } else {
            let mut srow: u16 = 1;
            for item in view.stack.iter() {
                if srow >= term_h - 1 { break; }
                let who = if item.controller == view.you { "you" } else { "opp" };
                let text = format!("{} ({})", item.name, who);
                let truncated: String = text.chars().take(side_w.saturating_sub(1)).collect();
                let _ = execute!(out, cursor::MoveTo(1, srow),
                    SetForegroundColor(Color::Cyan), Print(&truncated), ResetColor);
                srow += 1;
                // Show targets
                for target in &item.targets {
                    if srow >= term_h - 1 { break; }
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
                    let truncated: String = target_name.chars().take(side_w.saturating_sub(1)).collect();
                    let _ = execute!(out, cursor::MoveTo(1, srow),
                        SetForegroundColor(Color::DarkCyan), Print(&truncated), ResetColor);
                    srow += 1;
                }
            }
        }

        // ── Right panel: LOG ──
        let _ = execute!(out, cursor::MoveTo(log_col, 0),
            SetForegroundColor(Color::DarkYellow), SetAttribute(Attribute::Bold),
            Print("LOG"), SetAttribute(Attribute::Reset), ResetColor);
        if !log.is_empty() {
            let visible = h.saturating_sub(2);
            let start = if log.len() > visible { log.len() - visible } else { 0 };
            for (i, entry) in log[start..].iter().enumerate() {
                let row = (i + 1) as u16;
                if row >= term_h - 1 { break; }
                let truncated: String = entry.chars().take(side_w.saturating_sub(1)).collect();
                let _ = execute!(out, cursor::MoveTo(log_col + 1, row),
                    SetAttribute(Attribute::Dim), Print(&truncated), SetAttribute(Attribute::Reset));
            }
        }

        // ── Middle panel: main game ──
        let mut row: u16 = 0;
        let mid_sep: String = "─".repeat(mid_w);

        // Turn/phase at top
        let step_name = format!("{:?}", view.step);
        let whose_turn = if view.active_player == view.you { "Your turn" } else { "Opp's turn" };
        let mut status = format!(" T{} {} | {}", view.turn_number, step_name, whose_turn);
        if !view.your_mana_pool.is_empty() {
            let mana_str: Vec<String> = view.your_mana_pool.mana.iter()
                .filter(|(_, &v)| v > 0)
                .map(|(t, v)| format!("{:?}:{}", t, v))
                .collect();
            status.push_str(&format!("  Pool: {}", mana_str.join(" ")));
        }
        let _ = execute!(out, cursor::MoveTo(mid_col, row),
            SetAttribute(Attribute::Bold), Print(&status), SetAttribute(Attribute::Reset));
        row += 1;

        // Opponent info
        let opp_gy: usize = view.graveyards.iter()
            .filter(|(pid, _)| *pid != view.you)
            .map(|(_, cards)| cards.len()).sum();
        let opp_exile: usize = view.exile.iter()
            .filter(|c| c.owner != view.you).count();
        Self::mid_print(&mut out, mid_col, &mut row, mid_w, " OPPONENT", Some(Color::Red), true);
        for opp in &view.opponents {
            Self::mid_print(&mut out, mid_col, &mut row, mid_w,
                &format!("  Life: {}  Hand: {}  Library: {}  Grave: {}  Exile: {}",
                    opp.life, opp.hand_size, opp.library_size, opp_gy, opp_exile),
                None, false);
        }

        // Opponent battlefield
        let opp_perms: Vec<&PermanentView> = view.battlefield.iter()
            .filter(|p| p.controller != view.you).collect();
        row = Self::render_battlefield_at(&mut out, &opp_perms, Color::Red, mid_col, row, mid_w, &view.battlefield);

        // Battlefield separator
        Self::mid_print(&mut out, mid_col, &mut row, mid_w, &format!("─── BATTLEFIELD {}", "─".repeat(mid_w.saturating_sub(16))), None, false);

        // Your battlefield
        let your_perms: Vec<&PermanentView> = view.battlefield.iter()
            .filter(|p| p.controller == view.you).collect();
        row = Self::render_battlefield_at(&mut out, &your_perms, Color::Green, mid_col, row, mid_w, &view.battlefield);

        // Your info (above hand)
        let _ = execute!(out, cursor::MoveTo(mid_col, row),
            SetAttribute(Attribute::Dim), Print(&mid_sep), SetAttribute(Attribute::Reset));
        row += 1;

        let your_gy: usize = view.graveyards.iter()
            .filter(|(pid, _)| *pid == view.you)
            .map(|(_, cards)| cards.len()).sum();
        let your_exile: usize = view.exile.iter()
            .filter(|c| c.owner == view.you).count();
        let _ = execute!(out, cursor::MoveTo(mid_col, row),
            SetForegroundColor(Color::Green), SetAttribute(Attribute::Bold),
            Print(format!(" Life: {}  Library: {}  Grave: {}  Exile: {}",
                view.your_life, view.your_library_size, your_gy, your_exile)),
            SetAttribute(Attribute::Reset), ResetColor);
        row += 1;

        // Hand
        Self::mid_print(&mut out, mid_col, &mut row, mid_w, " HAND", Some(Color::Green), true);
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

        // Message
        if let Some(msg) = message {
            Self::mid_print(&mut out, mid_col, &mut row, mid_w, &format!(" {}", msg), Some(Color::Yellow), true);
        }

        // Actions
        if let Some(actions) = actions {
            let _ = execute!(out, cursor::MoveTo(mid_col, row),
                SetAttribute(Attribute::Dim), Print(&mid_sep), SetAttribute(Attribute::Reset));
            row += 1;
            for (i, action) in actions.iter().enumerate() {
                let desc = Self::format_action(view, action);
                let _ = execute!(out, cursor::MoveTo(mid_col, row),
                    SetAttribute(Attribute::Bold), Print(format!("  {}", i)),
                    SetAttribute(Attribute::Reset), Print(format!(": {}", desc)));
                row += 1;
            }
            let has_pass = actions.first().map(|a| matches!(a, Action::PassPriority)).unwrap_or(false);
            let hints = if has_pass {
                "  [enter=pass] [f=pass turn] [l=log] [g=graveyard] [e=exile] [d=deck]"
            } else {
                "  [l=log] [g=graveyard] [e=exile] [d=deck]"
            };
            let _ = execute!(out, cursor::MoveTo(mid_col, row),
                SetAttribute(Attribute::Dim), Print(hints), SetAttribute(Attribute::Reset));
            row += 1;
        }

        // Move cursor to input area in middle panel
        let _ = execute!(out, cursor::MoveTo(mid_col, row));
        let _ = out.flush();
    }

    /// Render battlefield permanents at a specific column/row, return next row.
    fn render_battlefield_at(out: &mut io::Stdout, perms: &[&PermanentView], color: Color,
                              col: u16, mut row: u16, max_w: usize,
                              all_perms: &[PermanentView]) -> u16 {
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

        // Lands
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
            let _ = execute!(out, cursor::MoveTo(col, row),
                SetForegroundColor(color), Print(&truncated), ResetColor);
            row += 1;
        }

        // Creatures
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
            let _ = execute!(out, cursor::MoveTo(col, row),
                SetForegroundColor(color), Print(&truncated), ResetColor);
            row += 1;
        }

        // Non-aura enchantments
        for e in &enchantments {
            if e.attached_to.is_some() { continue; }
            let _ = execute!(out, cursor::MoveTo(col, row),
                SetForegroundColor(Color::Magenta), Print(format!("  {} #{}", e.name, e.object_id.0)), ResetColor);
            row += 1;
        }

        // Artifacts
        for a in &artifacts {
            let t = if a.tapped { " [T]" } else { "" };
            let _ = execute!(out, cursor::MoveTo(col, row), Print(format!("  {}{} #{}", a.name, t, a.object_id.0)));
            row += 1;
        }

        row
    }

    fn render_battlefield(out: &mut impl Write, perms: &[&PermanentView], color: Color) {
        let has_type = |p: &&PermanentView, t: CardType| p.card_types.contains(&t);
        let lands: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Land)).collect();
        let creatures: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Creature)).collect();
        let enchantments: Vec<_> = perms.iter().filter(|p|
            has_type(p, CardType::Enchantment) && !has_type(p, CardType::Creature)).collect();
        let artifacts: Vec<_> = perms.iter().filter(|p|
            has_type(p, CardType::Artifact) && !has_type(p, CardType::Creature) && !has_type(p, CardType::Land)).collect();

        // Aura map
        let mut aura_map: HashMap<ObjectId, Vec<String>> = HashMap::new();
        for e in &enchantments {
            if let Some(target_id) = e.attached_to {
                aura_map.entry(target_id).or_default().push(e.name.clone());
            }
        }

        // Lands
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
            let _ = execute!(out, SetForegroundColor(color));
            let _ = execute!(out, Print("  Lands: "));
            let _ = execute!(out, ResetColor);
            let parts: Vec<String> = summary.iter().map(|(name, untapped, tapped)| {
                let total = untapped + tapped;
                if *tapped == 0 { format!("{}x {}", total, name) }
                else if *untapped == 0 { format!("{}x {} (tapped)", total, name) }
                else { format!("{}x {} ({} tapped)", total, name, tapped) }
            }).collect();
            let _ = execute!(out, Print(format!("{}\n", parts.join(", "))));
        }

        // Creatures
        for c in &creatures {
            let _ = execute!(out, SetForegroundColor(color));
            let _ = execute!(out, Print("  "));

            // Name
            let _ = execute!(out, Print(&c.name));

            // P/T
            let pt = match (c.effective_power, c.effective_toughness) {
                (Some(p), Some(t)) => format!(" {}/{}", p, t),
                _ => match (c.power, c.toughness) {
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                    _ => String::new(),
                },
            };
            let _ = execute!(out, Print(&pt));
            let _ = execute!(out, ResetColor);

            // Auras
            if let Some(names) = aura_map.get(&c.object_id) {
                let _ = execute!(out, SetForegroundColor(Color::Magenta),
                    Print(format!(" [{}]", names.join(", "))), ResetColor);
            }

            // Damage
            if c.damage_marked > 0 {
                let _ = execute!(out, SetForegroundColor(Color::Red),
                    Print(format!(" ({}dmg)", c.damage_marked)), ResetColor);
            }

            // Tapped
            if c.tapped {
                let _ = execute!(out, SetForegroundColor(Color::Yellow),
                    Print(" [T]"), ResetColor);
            }

            // Sick
            if c.summoning_sick {
                let _ = execute!(out, SetAttribute(Attribute::Dim),
                    Print(" [S]"), SetAttribute(Attribute::Reset));
            }

            let _ = execute!(out, Print("\n"));
        }

        // Non-aura enchantments
        for e in &enchantments {
            if e.attached_to.is_some() { continue; }
            let _ = execute!(out, SetForegroundColor(Color::Magenta),
                Print(format!("  {}\n", e.name)), ResetColor);
        }

        // Artifacts
        for a in &artifacts {
            let tapped = if a.tapped { " [T]" } else { "" };
            let _ = execute!(out, Print(format!("  {}{}\n", a.name, tapped)));
        }
    }

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

    fn print_dim(out: &mut impl Write, text: &str) {
        let _ = execute!(out, SetAttribute(Attribute::Dim),
            Print(format!("{}\n", text)), SetAttribute(Attribute::Reset));
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

    fn show_deck_browser() {
        let registry = mtg_engine::cards::CardRegistry::with_all_cards();
        let mut out = stdout();

        loop {
            let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
            Self::print_colored(&mut out, Color::Cyan, " CARD BROWSER");
            let _ = execute!(out, Print("\n"));

            let mut cards: Vec<mtg_engine::cards::CardData> = Vec::new();
            for i in 1..100u32 {
                let id = mtg_engine::ids::CardId(i);
                if let Some(data) = registry.card_data(id) {
                    if !cards.iter().any(|c| c.name == data.name) {
                        cards.push(data);
                    }
                }
            }
            cards.sort_by(|a, b| a.name.cmp(&b.name));

            for (i, data) in cards.iter().enumerate() {
                let cost = data.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
                let types: Vec<&str> = data.card_types.iter().map(|t| match t {
                    CardType::Land => "Land",
                    CardType::Creature => "Creature",
                    CardType::Instant => "Instant",
                    CardType::Sorcery => "Sorcery",
                    CardType::Enchantment => "Enchantment",
                    CardType::Artifact => "Artifact",
                    CardType::Planeswalker => "Planeswalker",
                }).collect();
                let pt = match (data.power, data.toughness) {
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                    _ => String::new(),
                };
                let _ = execute!(out,
                    SetAttribute(Attribute::Bold), Print(format!("  {:>2}", i)),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}{} [{}]{}\n", data.name, cost, types.join(" "), pt)));
            }

            let _ = execute!(out, Print("\n  Enter number for details, or press enter to return: "));
            let _ = out.flush();
            let input = Self::read_line("");

            if input.is_empty() { return; }

            if let Ok(idx) = input.parse::<usize>() {
                if idx < cards.len() {
                    let data = &cards[idx];
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
                    if !data.oracle_text.is_empty() {
                        let _ = execute!(out, SetForegroundColor(Color::Yellow),
                            Print(format!("\n  {}\n", data.oracle_text)), ResetColor);
                    }
                    let _ = execute!(out, Print("\n  Press enter to return to list..."));
                    let _ = out.flush();
                    let _ = Self::read_line("");
                }
            }
        }
    }

    fn show_zone(_view: &GameView, title: &str, cards: &[CardView]) {
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
        Self::print_colored(&mut out, Color::Cyan, &format!(" {}", title));
        if cards.is_empty() {
            let _ = execute!(out, Print("  (empty)\n"));
        } else {
            for card in cards {
                let cost = card.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
                let pt = match (card.power, card.toughness) {
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                    _ => String::new(),
                };
                let _ = execute!(out, Print(format!("  {}{}{}\n", card.name, cost, pt)));
            }
        }
        let _ = execute!(out, Print("\n  Press enter to return..."));
        let _ = out.flush();
        let _ = Self::read_line("");
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

        Self::render(view, None, Some("DECLARE ATTACKERS"), &self.log);

        let mut out = stdout();
        let _ = execute!(out, Print("\n"));
        Self::print_colored(&mut out, Color::Yellow, " Eligible attackers:");
        for (i, &id) in eligible.iter().enumerate() {
            let _ = execute!(out,
                SetAttribute(Attribute::Bold), Print(format!("  {}", i)),
                SetAttribute(Attribute::Reset), Print(format!(": {}\n", Self::perm_name(view, id))),
            );
        }
        let _ = execute!(out, Print("\n"));
        let _ = out.flush();

        loop {
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

        Self::render(view, None, Some("DECLARE BLOCKERS"), &self.log);

        let mut out = stdout();
        let _ = execute!(out, Print("\n"));
        Self::print_colored(&mut out, Color::Red, " Attackers:");
        for (i, &id) in attacker_ids.iter().enumerate() {
            let _ = execute!(out, Print(format!("  {}: {}\n", i, Self::perm_name(view, id))));
        }
        Self::print_colored(&mut out, Color::Green, " Your blockers:");
        for (i, &id) in eligible_blockers.iter().enumerate() {
            let _ = execute!(out, Print(format!("  {}: {}\n", i, Self::perm_name(view, id))));
        }
        let _ = execute!(out, Print("\n"));
        let _ = out.flush();

        loop {
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
            self.update_log(view);
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
                    self.update_log(view);
                    return Action::PassPriority;
                }
            }
        }

        self.update_log(view);

        loop {
            Self::render(view, Some(legal_actions), None, &self.log);

            let input = Self::read_line("\n  > ");

            // Keyboard shortcuts
            match input.as_str() {
                "g" => {
                    // Show all graveyards
                    let mut all_gy: Vec<CardView> = Vec::new();
                    for (_pid, cards) in &view.graveyards {
                        for card in cards {
                            all_gy.push(card.clone());
                        }
                    }
                    Self::show_zone(view, "GRAVEYARD", &all_gy);
                    continue;
                }
                "e" => {
                    Self::show_zone(view, "EXILE", &view.exile);
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
                    Self::show_log(&self.log);
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
                Self::show_deck_browser();
                continue;
            }

            if let Ok(idx) = input.parse::<usize>() {
                if idx < legal_actions.len() {
                    if matches!(legal_actions[idx], Action::Concede) {
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
    pub fn choose_combat(&mut self, view: &GameView, prompt: &CombatPrompt) -> Action {
        self.update_log(view);
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

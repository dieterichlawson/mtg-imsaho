use std::collections::HashMap;
use std::io::{self, Write, stdout};

use crossterm::{
    cursor, execute, queue,
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
            // Life changes
            if view.your_life != prev.your_life {
                let diff = view.your_life - prev.your_life;
                if diff > 0 {
                    self.log.push(format!("T{} You gained {} life ({})", view.turn_number, diff, view.your_life));
                } else {
                    self.log.push(format!("T{} You took {} damage ({})", view.turn_number, -diff, view.your_life));
                }
            }
            for (opp, prev_opp) in view.opponents.iter().zip(prev.opponents.iter()) {
                if opp.life != prev_opp.life {
                    let diff = opp.life - prev_opp.life;
                    if diff > 0 {
                        self.log.push(format!("T{} Opponent gained {} life ({})", view.turn_number, diff, opp.life));
                    } else {
                        self.log.push(format!("T{} Opponent took {} damage ({})", view.turn_number, -diff, opp.life));
                    }
                }
            }

            // New permanents on battlefield
            for perm in &view.battlefield {
                if !prev.battlefield.iter().any(|p| p.object_id == perm.object_id) {
                    let who = if perm.controller == view.you { "You" } else { "Opponent" };
                    let pt = match (perm.effective_power, perm.effective_toughness) {
                        (Some(p), Some(t)) => format!(" {}/{}", p, t),
                        _ => String::new(),
                    };
                    self.log.push(format!("T{} {} played {}{}", view.turn_number, who, perm.name, pt));
                }
            }

            // Permanents that left the battlefield
            for prev_perm in &prev.battlefield {
                if !view.battlefield.iter().any(|p| p.object_id == prev_perm.object_id) {
                    let who = if prev_perm.controller == view.you { "Your" } else { "Opponent's" };
                    self.log.push(format!("T{} {} {} left the battlefield", view.turn_number, who, prev_perm.name));
                }
            }

            // New items on stack
            for item in &view.stack {
                if !prev.stack.iter().any(|s| s.object_id == item.object_id) {
                    let who = if item.controller == view.you { "You" } else { "Opponent" };
                    self.log.push(format!("T{} {} cast {}", view.turn_number, who, item.name));
                }
            }

            // Items resolved from stack
            for prev_item in &prev.stack {
                if !view.stack.iter().any(|s| s.object_id == prev_item.object_id) {
                    self.log.push(format!("T{} {} resolved", view.turn_number, prev_item.name));
                }
            }

            // Turn changes
            if view.turn_number != prev.turn_number {
                let whose = if view.active_player == view.you { "Your" } else { "Opponent's" };
                self.log.push(format!("── Turn {} ({}) ──", view.turn_number, whose));
            }
        }
        self.last_view = Some(view.clone());
    }

    // ── Rendering ──────────────────────────────────────────────────

    fn render(view: &GameView, actions: Option<&[Action]>, message: Option<&str>, log: &[String]) {
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        let w = term_w as usize;
        let log_w = w / 3;          // right 1/3 for log
        let main_w = w - log_w - 1; // left 2/3 for game, -1 for separator

        // Build left-side lines as plain strings (we'll colorize when rendering).
        // For now, just render normally on the left and overlay the log on the right.

        // ── Opponent info ──
        Self::print_colored(&mut out, Color::Red, " OPPONENT");
        for opp in &view.opponents {
            let _ = execute!(out,
                Print(format!("  Life: {}  Hand: {}  Library: {}\n", opp.life, opp.hand_size, opp.library_size))
            );
        }

        // ── Opponent battlefield ──
        let opp_perms: Vec<&PermanentView> = view.battlefield.iter()
            .filter(|p| p.controller != view.you).collect();
        if !opp_perms.is_empty() {
            Self::render_battlefield(&mut out, &opp_perms, Color::Red);
        }

        let sep = "─".repeat(main_w);
        Self::print_dim(&mut out, &sep);

        // ── Your battlefield ──
        let your_perms: Vec<&PermanentView> = view.battlefield.iter()
            .filter(|p| p.controller == view.you).collect();
        if !your_perms.is_empty() {
            Self::render_battlefield(&mut out, &your_perms, Color::Green);
        }

        // ── Stack ──
        if !view.stack.is_empty() {
            Self::print_colored(&mut out, Color::Cyan, " STACK");
            for item in &view.stack {
                let who = if item.controller == view.you { "you" } else { "opp" };
                let _ = execute!(out, Print(format!("  {} ({})\n", item.name, who)));
            }
        }

        // ── Status bar ──
        Self::print_dim(&mut out, &sep);
        let step_name = format!("{:?}", view.step);
        let whose_turn = if view.active_player == view.you { "Your turn" } else { "Opp's turn" };
        let _ = execute!(out,
            SetAttribute(Attribute::Bold),
            Print(format!(" T{} {} | {}", view.turn_number, step_name, whose_turn)),
            SetAttribute(Attribute::Reset),
        );

        if !view.your_mana_pool.is_empty() {
            let mana_str: Vec<String> = view.your_mana_pool.mana.iter()
                .filter(|(_, &v)| v > 0)
                .map(|(t, v)| format!("{:?}:{}", t, v))
                .collect();
            let _ = execute!(out, Print(format!("  Pool: {}", mana_str.join(" "))));
        }
        let _ = execute!(out, Print("\n"));

        // ── Hand ──
        Self::print_dim(&mut out, &sep);
        Self::print_colored(&mut out, Color::Green, " HAND");
        if view.your_hand.is_empty() {
            let _ = execute!(out, Print("  (empty)\n"));
        } else {
            for card in &view.your_hand {
                let cost = card.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
                let pt = match (card.power, card.toughness) {
                    (Some(p), Some(t)) => format!(" {}/{}", p, t),
                    _ => String::new(),
                };
                let _ = execute!(out, Print(format!("  {}{}{}\n", card.name, cost, pt)));
            }
        }

        // ── Your info bar ──
        let _ = execute!(out,
            SetForegroundColor(Color::Green),
            SetAttribute(Attribute::Bold),
            Print(format!(" Life: {}  Library: {}\n", view.your_life, view.your_library_size)),
            SetAttribute(Attribute::Reset),
            ResetColor,
        );

        // ── Message ──
        if let Some(msg) = message {
            Self::print_colored(&mut out, Color::Yellow, &format!(" {}", msg));
        }

        // ── Actions ──
        if let Some(actions) = actions {
            Self::print_dim(&mut out, &sep);
            for (i, action) in actions.iter().enumerate() {
                let desc = Self::format_action(view, action);
                let _ = execute!(out,
                    SetAttribute(Attribute::Bold),
                    Print(format!("  {}", i)),
                    SetAttribute(Attribute::Reset),
                    Print(format!(": {}\n", desc)),
                );
            }
            let has_pass = actions.first().map(|a| matches!(a, Action::PassPriority)).unwrap_or(false);
            if has_pass {
                Self::print_dim(&mut out, "  [enter=pass] [f=pass turn] [l=log] [g=graveyard] [e=exile] [?N=card info]");
            } else {
                Self::print_dim(&mut out, "  [l=log] [g=graveyard] [e=exile] [?N=card info]");
            }
        }

        // ── Log on the right side ──
        if !log.is_empty() {
            let h = term_h as usize;
            let log_col = (main_w + 1) as u16;
            let visible = h.saturating_sub(2);
            let start = if log.len() > visible { log.len() - visible } else { 0 };

            // Draw separator line
            for row in 0..h {
                let _ = execute!(out, cursor::MoveTo(main_w as u16, row as u16),
                    SetAttribute(Attribute::Dim), Print("│"), SetAttribute(Attribute::Reset));
            }

            // Draw log header
            let _ = execute!(out, cursor::MoveTo(log_col, 0),
                SetForegroundColor(Color::Cyan), SetAttribute(Attribute::Bold),
                Print("LOG"), SetAttribute(Attribute::Reset), ResetColor);

            // Draw log entries
            for (i, entry) in log[start..].iter().enumerate() {
                let row = (i + 1) as u16;
                if row >= term_h { break; }
                let truncated: String = entry.chars().take(log_w.saturating_sub(1)).collect();
                let _ = execute!(out, cursor::MoveTo(log_col + 1, row),
                    SetAttribute(Attribute::Dim),
                    Print(&truncated),
                    SetAttribute(Attribute::Reset));
            }
        }

        // Move cursor back to input area
        let _ = execute!(out, cursor::MoveTo(0, term_h.saturating_sub(1)));
        let _ = out.flush();
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
            for (i, entry) in log[start..].iter().enumerate() {
                let _ = execute!(out, SetAttribute(Attribute::Dim),
                    Print(format!("  {}\n", entry)), SetAttribute(Attribute::Reset));
            }
        }
        let _ = execute!(out, Print("\n  Press enter to return..."));
        let _ = out.flush();
        let _ = Self::read_line("");
    }

    fn show_zone(view: &GameView, title: &str, cards: &[CardView]) {
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
                    for (pid, cards) in &view.graveyards {
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

            // Card info: ?0, ?1, etc. — show details of a battlefield permanent
            if input.starts_with('?') {
                if let Ok(idx) = input[1..].parse::<usize>() {
                    // Find the idx-th permanent on the battlefield
                    if idx < view.battlefield.len() {
                        let perm = &view.battlefield[idx];
                        let mut out = stdout();
                        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
                        Self::print_colored(&mut out, Color::Cyan,
                            &format!(" CARD: {}", perm.name));
                        let types: Vec<&str> = perm.card_types.iter().map(|t| match t {
                            CardType::Land => "Land",
                            CardType::Creature => "Creature",
                            CardType::Instant => "Instant",
                            CardType::Sorcery => "Sorcery",
                            CardType::Enchantment => "Enchantment",
                            CardType::Artifact => "Artifact",
                            CardType::Planeswalker => "Planeswalker",
                        }).collect();
                        let _ = execute!(out, Print(format!("  Types: {}\n", types.join(" "))));
                        if let (Some(p), Some(t)) = (perm.power, perm.toughness) {
                            let _ = execute!(out, Print(format!("  Base P/T: {}/{}\n", p, t)));
                        }
                        if let (Some(p), Some(t)) = (perm.effective_power, perm.effective_toughness) {
                            let _ = execute!(out, Print(format!("  Effective P/T: {}/{}\n", p, t)));
                        }
                        if perm.damage_marked > 0 {
                            let _ = execute!(out, Print(format!("  Damage: {}\n", perm.damage_marked)));
                        }
                        let controller = if perm.controller == view.you { "You" } else { "Opponent" };
                        let _ = execute!(out, Print(format!("  Controller: {}\n", controller)));
                        let _ = execute!(out, Print(format!("  Tapped: {}\n", perm.tapped)));
                        let _ = execute!(out, Print(format!("  Summoning sick: {}\n", perm.summoning_sick)));
                        if let Some(att) = perm.attached_to {
                            let att_name = view.battlefield.iter()
                                .find(|p| p.object_id == att)
                                .map(|p| p.name.as_str())
                                .unwrap_or("?");
                            let _ = execute!(out, Print(format!("  Attached to: {}\n", att_name)));
                        }
                        let _ = execute!(out, Print("\n  Press enter to return..."));
                        let _ = out.flush();
                        let _ = Self::read_line("");
                    }
                }
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

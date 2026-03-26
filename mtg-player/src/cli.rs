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
    /// When true, auto-pass priority until it's our turn again
    /// or the opponent puts something on the stack we can respond to.
    pass_until_my_turn: bool,
}

impl CliPlayer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pass_until_my_turn: false,
        }
    }

    // ── Rendering ──────────────────────────────────────────────────

    fn render(view: &GameView, actions: Option<&[Action]>, message: Option<&str>) {
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

        let w = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
        let bar = "─".repeat(w);

        // ── Opponent info ──
        Self::print_colored(&mut out, Color::Red, &format!(" OPPONENT"));
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

        Self::print_dim(&mut out, &format!("{}",  bar));

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
        Self::print_dim(&mut out, &format!("{}", bar));
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
        Self::print_dim(&mut out, &format!("{}", bar));
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
            Self::print_dim(&mut out, &format!("{}", bar));
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
                Self::print_dim(&mut out, "  [enter=pass]  [f=pass until my turn]  [g=graveyard]  [e=exile]  [?N=card info]");
            } else {
                Self::print_dim(&mut out, "  [g=graveyard]  [e=exile]  [?N=card info]");
            }
        }

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

        Self::render(view, None, Some("DECLARE ATTACKERS"));

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

        Self::render(view, None, Some("DECLARE BLOCKERS"));

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
            return Action::PassPriority;
        }

        // "Pass until my turn" mode (F6-like).
        if self.pass_until_my_turn && has_pass {
            // Break if it's our turn.
            if view.active_player == view.you {
                self.pass_until_my_turn = false;
            }
            // Break if opponent put something on the stack we can respond to.
            else if !view.stack.is_empty() {
                self.pass_until_my_turn = false;
            }
            // Otherwise, auto-pass.
            else {
                return Action::PassPriority;
            }
        }

        loop {
            Self::render(view, Some(legal_actions), None);

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
                        self.pass_until_my_turn = true;
                        return Action::PassPriority;
                    }
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
        match prompt {
            CombatPrompt::ChooseAttackers { .. } => {
                // If in pass mode, don't attack.
                if self.pass_until_my_turn {
                    return Action::DeclareAttackers { attackers: vec![] };
                }
                self.choose_attackers(view, prompt)
            }
            CombatPrompt::ChooseBlockers { .. } => {
                // Always prompt for blockers — blocking is too important to skip.
                // Break pass mode so player sees the board state.
                self.pass_until_my_turn = false;
                self.choose_blockers(view, prompt)
            }
        }
    }
}

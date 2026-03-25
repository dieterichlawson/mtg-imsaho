use std::io::{self, Write};
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::ids::ObjectId;
use mtg_engine::view::{GameView, CardView, PermanentView};

use crate::Player;

/// A player that interacts via the command line.
pub struct CliPlayer {
    name: String,
}

impl CliPlayer {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    fn perm_name(view: &GameView, id: ObjectId) -> String {
        view.battlefield.iter()
            .find(|p| p.object_id == id)
            .map(|p| {
                let pt = match (p.power, p.toughness) {
                    (Some(pw), Some(t)) => format!(" {}/{}", pw, t),
                    _ => String::new(),
                };
                format!("{}{} ({})", p.name, pt, p.object_id)
            })
            .or_else(|| view.your_hand.iter()
                .find(|c| c.object_id == id)
                .map(|c| format!("{} ({})", c.name, c.object_id)))
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
                        mtg_engine::actions::Target::Object(id) => Self::perm_name(view, *id),
                        mtg_engine::actions::Target::Player(pid) => {
                            if *pid == view.you { "you".into() } else { format!("opponent (p#{})", pid.0) }
                        }
                    }).collect();
                    format!("Cast {} → {}", name, target_names.join(", "))
                }
            }
            Action::ActivateManaAbility { object_id, .. } =>
                format!("Tap {} for mana", Self::perm_name(view, *object_id)),
            Action::DeclareAttackers { attackers } => {
                if attackers.is_empty() {
                    "Don't attack".into()
                } else {
                    let names: Vec<String> = attackers.iter()
                        .map(|(id, _)| Self::perm_name(view, *id))
                        .collect();
                    format!("Attack with {}", names.join(", "))
                }
            }
            Action::DeclareBlockers { assignments } => {
                if assignments.is_empty() {
                    "Don't block".into()
                } else {
                    let descs: Vec<String> = assignments.iter()
                        .map(|(blocker, attacker)|
                            format!("{} blocks {}", Self::perm_name(view, *blocker), Self::perm_name(view, *attacker)))
                        .collect();
                    format!("Block: {}", descs.join(", "))
                }
            }
            Action::DiscardCards { cards } => {
                let names: Vec<String> = cards.iter()
                    .map(|id| Self::perm_name(view, *id))
                    .collect();
                format!("Discard {}", names.join(", "))
            }
            Action::Concede => "Concede".into(),
        }
    }

    fn print_game_state(&self, view: &GameView) {
        println!("\n{}", "=".repeat(60));
        println!("Turn {} | Step: {:?} | Active: player#{}",
            view.turn_number, view.step, view.active_player.0);
        println!("{}", "=".repeat(60));

        for opp in &view.opponents {
            println!("Opponent (player#{}): Life={}, Hand={} cards, Library={} cards",
                opp.id.0, opp.life, opp.hand_size, opp.library_size);
        }

        if !view.battlefield.is_empty() {
            use mtg_engine::types::CardType;

            let mut players: Vec<mtg_engine::ids::PlayerId> = view.battlefield.iter()
                .map(|p| p.controller)
                .collect();
            players.sort_by_key(|p| p.0);
            players.dedup();

            for player in &players {
                let is_you = *player == view.you;
                let label = if is_you { "You".to_string() } else { format!("Opponent (p#{})", player.0) };

                let perms: Vec<&PermanentView> = view.battlefield.iter()
                    .filter(|p| p.controller == *player)
                    .collect();

                // Categorize permanents.
                let has_type = |p: &&PermanentView, t: CardType| p.card_types.contains(&t);
                let lands: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Land)).collect();
                let creatures: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Creature)).collect();
                let artifacts: Vec<_> = perms.iter().filter(|p|
                    has_type(p, CardType::Artifact) && !has_type(p, CardType::Creature) && !has_type(p, CardType::Land)
                ).collect();
                let enchantments: Vec<_> = perms.iter().filter(|p|
                    has_type(p, CardType::Enchantment) && !has_type(p, CardType::Creature)
                ).collect();
                let planeswalkers: Vec<_> = perms.iter().filter(|p| has_type(p, CardType::Planeswalker)).collect();
                let other: Vec<_> = perms.iter().filter(|p|
                    !has_type(p, CardType::Land) && !has_type(p, CardType::Creature) &&
                    !has_type(p, CardType::Artifact) && !has_type(p, CardType::Enchantment) &&
                    !has_type(p, CardType::Planeswalker)
                ).collect();

                println!("\n{}'s battlefield:", label);

                // Lands: summarized on one line.
                if !lands.is_empty() {
                    let mut land_summary: Vec<(String, usize, usize)> = Vec::new();
                    for land in &lands {
                        if let Some(entry) = land_summary.iter_mut().find(|(n, _, _)| *n == land.name) {
                            if land.tapped { entry.2 += 1; } else { entry.1 += 1; }
                        } else {
                            let (u, t) = if land.tapped { (0, 1) } else { (1, 0) };
                            land_summary.push((land.name.clone(), u, t));
                        }
                    }
                    let parts: Vec<String> = land_summary.iter().map(|(name, untapped, tapped)| {
                        let total = untapped + tapped;
                        if *tapped == 0 {
                            format!("{}x {}", total, name)
                        } else if *untapped == 0 {
                            format!("{}x {} [all tapped]", total, name)
                        } else {
                            format!("{}x {} ({} tapped)", total, name, tapped)
                        }
                    }).collect();
                    println!("  Lands: {}", parts.join(", "));
                }

                // Collect aura names by what they're attached to.
                let mut aura_map: std::collections::HashMap<mtg_engine::ids::ObjectId, Vec<String>> = std::collections::HashMap::new();
                for e in &enchantments {
                    if let Some(target_id) = e.attached_to {
                        aura_map.entry(target_id).or_default().push(e.name.clone());
                    }
                }

                // Creatures: listed individually with full status and attached auras.
                if !creatures.is_empty() {
                    println!("  Creatures:");
                    for c in &creatures {
                        let tapped = if c.tapped { " [TAPPED]" } else { "" };
                        let sick = if c.summoning_sick { " [SICK]" } else { "" };
                        let pt = match (c.effective_power, c.effective_toughness) {
                            (Some(p), Some(t)) => format!(" {}/{}", p, t),
                            _ => match (c.power, c.toughness) {
                                (Some(p), Some(t)) => format!(" {}/{}", p, t),
                                _ => String::new(),
                            },
                        };
                        let dmg = if c.damage_marked > 0 {
                            format!(" ({}dmg)", c.damage_marked)
                        } else {
                            String::new()
                        };
                        let auras = aura_map.get(&c.object_id)
                            .map(|names| format!(" [{}]", names.join(", ")))
                            .unwrap_or_default();
                        println!("    {}{}{}{}{}{} ({})", c.name, pt, auras, dmg, tapped, sick, c.object_id);
                    }
                }

                // Artifacts.
                if !artifacts.is_empty() {
                    println!("  Artifacts:");
                    for a in &artifacts {
                        let tapped = if a.tapped { " [TAPPED]" } else { "" };
                        println!("    {}{} ({})", a.name, tapped, a.object_id);
                    }
                }

                // Enchantments: skip auras shown with creatures.
                let non_aura_enchantments: Vec<_> = enchantments.iter()
                    .filter(|e| e.attached_to.is_none())
                    .collect();
                if !non_aura_enchantments.is_empty() {
                    println!("  Enchantments:");
                    for e in &non_aura_enchantments {
                        println!("    {} ({})", e.name, e.object_id);
                    }
                }

                // Planeswalkers.
                if !planeswalkers.is_empty() {
                    println!("  Planeswalkers:");
                    for pw in &planeswalkers {
                        println!("    {} ({})", pw.name, pw.object_id);
                    }
                }

                // Other.
                if !other.is_empty() {
                    println!("  Other:");
                    for o in &other {
                        let tapped = if o.tapped { " [TAPPED]" } else { "" };
                        println!("    {}{} ({})", o.name, tapped, o.object_id);
                    }
                }
            }
        }

        if !view.stack.is_empty() {
            println!("\nStack:");
            for item in &view.stack {
                println!("  {} (controller: player#{}, {})", item.name, item.controller.0, item.object_id);
            }
        }

        println!("\nYou (player#{}): Life={}, Library={} cards",
            view.you.0, view.your_life, view.your_library_size);

        if !view.your_mana_pool.is_empty() {
            println!("Mana pool: {:?}", view.your_mana_pool.mana);
        }

        println!("Hand:");
        for card in &view.your_hand {
            let cost_str = card.cost.as_ref()
                .map(|c| format!(" {}", c))
                .unwrap_or_default();
            let pt = match (card.power, card.toughness) {
                (Some(p), Some(t)) => format!(" {}/{}", p, t),
                _ => String::new(),
            };
            println!("  {}{}{} ({})", card.name, cost_str, pt, card.object_id);
        }
    }

    fn choose_attackers(&self, view: &GameView, prompt: &CombatPrompt) -> Action {
        let (eligible, defending) = match prompt {
            CombatPrompt::ChooseAttackers { eligible, defending_player } => (eligible, *defending_player),
            _ => unreachable!(),
        };

        if eligible.is_empty() {
            println!("\nNo creatures can attack.");
            return Action::DeclareAttackers { attackers: vec![] };
        }

        println!("\nEligible attackers:");
        for (i, &id) in eligible.iter().enumerate() {
            println!("  {}: {}", i, Self::perm_name(view, id));
        }
        println!("\nEnter attacker numbers separated by spaces (enter=don't attack):");

        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input.is_empty() {
                return Action::DeclareAttackers { attackers: vec![] };
            }

            if input == "all" {
                let attackers = eligible.iter().map(|&id| (id, defending)).collect();
                return Action::DeclareAttackers { attackers };
            }

            let indices: Vec<usize> = input.split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            if indices.iter().all(|&i| i < eligible.len()) {
                let attackers = indices.iter()
                    .map(|&i| (eligible[i], defending))
                    .collect();
                return Action::DeclareAttackers { attackers };
            }

            println!("Invalid selection. Enter numbers like '0 2' or 'all'.");
        }
    }

    fn choose_blockers(&self, view: &GameView, prompt: &CombatPrompt) -> Action {
        let (eligible_blockers, attacker_ids) = match prompt {
            CombatPrompt::ChooseBlockers { eligible_blockers, attackers } => (eligible_blockers, attackers),
            _ => unreachable!(),
        };

        if eligible_blockers.is_empty() {
            println!("\nNo creatures can block.");
            return Action::DeclareBlockers { assignments: vec![] };
        }

        println!("\nAttackers:");
        for (i, &id) in attacker_ids.iter().enumerate() {
            println!("  {}: {}", i, Self::perm_name(view, id));
        }
        println!("\nYour eligible blockers:");
        for (i, &id) in eligible_blockers.iter().enumerate() {
            println!("  {}: {}", i, Self::perm_name(view, id));
        }
        println!("\nAssign blockers as 'blocker->attacker' pairs (e.g., '0->0 1->0'). Enter=don't block:");

        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input.is_empty() {
                return Action::DeclareBlockers { assignments: vec![] };
            }

            let mut assignments = Vec::new();
            let mut valid = true;

            for pair in input.split_whitespace() {
                let parts: Vec<&str> = pair.split("->").collect();
                if parts.len() != 2 {
                    valid = false;
                    break;
                }
                let blocker_idx: usize = match parts[0].parse() {
                    Ok(v) => v,
                    Err(_) => { valid = false; break; }
                };
                let attacker_idx: usize = match parts[1].parse() {
                    Ok(v) => v,
                    Err(_) => { valid = false; break; }
                };
                if blocker_idx >= eligible_blockers.len() || attacker_idx >= attacker_ids.len() {
                    valid = false;
                    break;
                }
                assignments.push((eligible_blockers[blocker_idx], attacker_ids[attacker_idx]));
            }

            if valid {
                return Action::DeclareBlockers { assignments };
            }
            println!("Invalid. Use format '0->0 1->0' (blocker->attacker).");
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

        self.print_game_state(view);

        println!("\nActions:");
        for (i, action) in legal_actions.iter().enumerate() {
            println!("  {}: {}", i, Self::format_action(view, action));
        }

        // Default action.
        let default_idx = if !legal_actions.is_empty() && matches!(legal_actions[0], Action::PassPriority) {
            Some(0)
        } else {
            None
        };

        loop {
            if let Some(_) = default_idx {
                print!("Choose action (enter=pass)> ");
            } else {
                print!("Choose action> ");
            }
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input.is_empty() {
                if let Some(idx) = default_idx {
                    return legal_actions[idx].clone();
                }
            }

            if let Ok(idx) = input.parse::<usize>() {
                if idx < legal_actions.len() {
                    if matches!(legal_actions[idx], Action::Concede) {
                        print!("Are you sure you want to concede? (y/n)> ");
                        io::stdout().flush().unwrap();
                        let mut confirm = String::new();
                        io::stdin().read_line(&mut confirm).unwrap();
                        if confirm.trim().to_lowercase() != "y" {
                            continue;
                        }
                    }
                    return legal_actions[idx].clone();
                }
            }
            println!("Invalid choice, try again.");
        }
    }

    fn choose_cards_to_bottom(
        &mut self,
        _view: &GameView,
        hand: &[CardView],
        count: usize,
    ) -> Vec<ObjectId> {
        println!("\nChoose {} card(s) to put on the bottom of your library:", count);
        for (i, card) in hand.iter().enumerate() {
            println!("  {}: {}", i, card.name);
        }

        loop {
            print!("Enter {} card numbers (space-separated)> ", count);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            let indices: Vec<usize> = input.trim().split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            if indices.len() == count && indices.iter().all(|&i| i < hand.len()) {
                return indices.iter().map(|&i| hand[i].object_id).collect();
            }
            println!("Invalid selection, try again.");
        }
    }
}

impl CliPlayer {
    /// Handle combat prompts.
    pub fn choose_combat(&mut self, view: &GameView, prompt: &CombatPrompt) -> Action {
        self.print_game_state(view);
        match prompt {
            CombatPrompt::ChooseAttackers { .. } => self.choose_attackers(view, prompt),
            CombatPrompt::ChooseBlockers { .. } => self.choose_blockers(view, prompt),
        }
    }
}

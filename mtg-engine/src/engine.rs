use rand::seq::SliceRandom;

use crate::actions::Action;
use crate::cards::CardRegistry;
use crate::combat;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::mana;
use crate::sba::check_state_based_actions_with_registry;
use crate::stack;
use crate::state::{AwaitingAction, GameState, LogLevel};
use crate::types::*;

/// A decklist: card name -> count.
pub struct Decklist {
    pub entries: Vec<(String, u32)>,
}

/// Configuration for setting up a game.
pub struct GameConfig {
    pub player_names: Vec<String>,
    pub decklists: Vec<Decklist>,
    pub starting_life: i32,
}

/// Result of legal_actions: a list of actions plus an optional combat prompt.
/// When a combat prompt is present, the player should construct a
/// DeclareAttackers/DeclareBlockers action from it (not pick from the actions list).
pub struct LegalActions {
    pub actions: Vec<Action>,
    pub combat_prompt: Option<crate::actions::CombatPrompt>,
}

/// Compute all legal actions for the player who currently needs to act.
pub fn legal_actions(state: &GameState, registry: &CardRegistry) -> LegalActions {
    if state.is_game_over() {
        return LegalActions { actions: vec![], combat_prompt: None };
    }

    // If we're waiting for a specific action (attackers, blockers, discard).
    if let Some(awaiting) = &state.awaiting_action {
        return match awaiting {
            AwaitingAction::DeclareAttackers => {
                let eligible = combat::eligible_attackers_with_registry(state, state.active_player, registry);
                let defending = state.opponent(state.active_player);
                LegalActions {
                    actions: vec![],
                    combat_prompt: Some(crate::actions::CombatPrompt::ChooseAttackers {
                        eligible,
                        defending_player: defending,
                    }),
                }
            }
            AwaitingAction::DeclareBlockers { defending_player } => {
                let eligible_blockers = combat::eligible_blockers_with_registry(state, *defending_player, registry);
                let attacker_ids = state.combat.as_ref()
                    .map(|c| c.attackers.keys().copied().collect())
                    .unwrap_or_default();
                LegalActions {
                    actions: vec![],
                    combat_prompt: Some(crate::actions::CombatPrompt::ChooseBlockers {
                        eligible_blockers,
                        attackers: attacker_ids,
                    }),
                }
            }
            AwaitingAction::DiscardToHandSize { player, discard_count } => {
                LegalActions {
                    actions: legal_discard_actions(state, *player, *discard_count),
                    combat_prompt: None,
                }
            }
        };
    }

    let player = match state.priority_player {
        Some(p) => p,
        None => return LegalActions { actions: vec![], combat_prompt: None },
    };

    let mut actions = Vec::new();

    // PassPriority is always available when you have priority.
    actions.push(Action::PassPriority);

    // Mana abilities: can activate anytime you have priority.
    // Deduplicate by card_id — if you have 5 untapped Forests, only show one "Tap Forest".
    let mut seen_mana_abilities: Vec<(CardId, usize)> = Vec::new();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            let mana_abs = behavior.mana_abilities(state, obj.id);
            for ma in mana_abs {
                let key = (obj.card_id, ma.ability_index);
                if !seen_mana_abilities.contains(&key) {
                    seen_mana_abilities.push(key);
                    actions.push(Action::ActivateManaAbility {
                        object_id: obj.id,
                        ability_index: ma.ability_index,
                    });
                }
            }
        }
    }

    // Sorcery-speed window: your main phase, stack empty, your turn.
    let is_sorcery_speed = state.step.is_main_phase()
        && state.stack.is_empty()
        && state.active_player == player;

    // Instant-speed window: anytime you have priority (which is already true here).
    let player_state = state.get_player(player);

    if is_sorcery_speed {
        // Play a land (if land plays remaining > 0).
        // Deduplicate — only show one "Play Forest" even if you have 3 in hand.
        if player_state.land_plays_remaining > 0 {
            let mut seen_lands: Vec<CardId> = Vec::new();
            for obj in state.objects_in_zone(Zone::Hand, player) {
                if let Some(behavior) = registry.get(obj.card_id) {
                    let data = behavior.card_data();
                    if data.card_types.contains(&CardType::Land) && !seen_lands.contains(&obj.card_id) {
                        seen_lands.push(obj.card_id);
                        actions.push(Action::PlayLand { object_id: obj.id });
                    }
                }
            }
        }
    }

    // Cast spells from hand.
    // Deduplicate untargeted spells — only show one "Cast Kalonian Tusker" even if you have 3.
    // Targeted spells still get one entry per valid target.
    let mut seen_untargeted_casts: Vec<CardId> = Vec::new();
    for obj in state.objects_in_zone(Zone::Hand, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            let data = behavior.card_data();

            // Determine if this spell can be cast right now.
            let is_instant = data.card_types.contains(&CardType::Instant);
            let is_sorcery_type = data.card_types.contains(&CardType::Sorcery)
                || data.card_types.contains(&CardType::Creature)
                || data.card_types.contains(&CardType::Enchantment)
                || data.card_types.contains(&CardType::Artifact)
                || data.card_types.contains(&CardType::Planeswalker);

            let can_cast_timing = if is_instant {
                true // Instants can be cast anytime you have priority
            } else if is_sorcery_type {
                is_sorcery_speed
            } else {
                false
            };

            if !can_cast_timing {
                continue;
            }

            // Check mana.
            if let Some(cost) = &data.cost {
                if !mana::can_pay(&player_state.mana_pool, cost) {
                    continue;
                }
            }

            // Generate cast actions with valid targets.
            let target_req = behavior.target_requirement();

            // For untargeted spells, deduplicate by card_id.
            if matches!(target_req, crate::cards::TargetRequirement::None) {
                if seen_untargeted_casts.contains(&obj.card_id) {
                    continue;
                }
                seen_untargeted_casts.push(obj.card_id);
            }

            let cast_actions = generate_cast_actions_with_targets(
                state, player, obj.id, &target_req, behavior,
            );
            actions.extend(cast_actions);
        }
    }

    // Concede is always last.
    actions.push(Action::Concede);

    LegalActions { actions, combat_prompt: None }
}

/// Generate CastSpell actions with all valid target combinations.
fn generate_cast_actions_with_targets(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    target_req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
) -> Vec<Action> {
    use crate::actions::Target;
    use crate::cards::TargetRequirement;

    match target_req {
        TargetRequirement::None => {
            vec![Action::CastSpell { object_id: spell_id, targets: vec![] }]
        }
        TargetRequirement::AnyTarget => {
            // Can target any creature on the battlefield or any player.
            let mut actions = Vec::new();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                if obj.power.is_some() { // is a creature
                    let target = Target::Object(obj.id);
                    if behavior.is_valid_target(state, caster, &target) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                        });
                    }
                }
            }
            for player in &state.players {
                if !player.lost {
                    let target = Target::Player(player.id);
                    if behavior.is_valid_target(state, caster, &target) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                        });
                    }
                }
            }
            actions
        }
        TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_) => {
            let mut actions = Vec::new();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                if obj.power.is_some() { // is a creature
                    let target = Target::Object(obj.id);
                    if behavior.is_valid_target(state, caster, &target) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                        });
                    }
                }
            }
            actions
        }
        TargetRequirement::PlayerOnly => {
            let mut actions = Vec::new();
            for player in &state.players {
                if !player.lost {
                    let target = Target::Player(player.id);
                    if behavior.is_valid_target(state, caster, &target) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                        });
                    }
                }
            }
            actions
        }
    }
}

// Attacker/blocker enumeration removed — players now construct combat
// actions from CombatPrompt data. The engine validates on submission.

/// Generate legal discard actions for hand size.
fn legal_discard_actions(state: &GameState, player: PlayerId, discard_count: usize) -> Vec<Action> {
    let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, player)
        .iter().map(|o| o.id).collect();

    if hand.len() <= discard_count {
        // Must discard all.
        return vec![Action::DiscardCards { cards: hand }];
    }

    // Enumerate all combinations of `discard_count` cards from hand.
    let combos = combinations(&hand, discard_count);
    combos.into_iter()
        .map(|cards| Action::DiscardCards { cards })
        .collect()
}

fn combinations(items: &[ObjectId], k: usize) -> Vec<Vec<ObjectId>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    let mut result = Vec::new();
    for i in 0..=items.len() - k {
        let rest = combinations(&items[i + 1..], k - 1);
        for mut combo in rest {
            combo.insert(0, items[i]);
            result.push(combo);
        }
    }
    result
}

fn card_name(state: &GameState, registry: &CardRegistry, obj_id: ObjectId) -> String {
    state.get_object(obj_id)
        .and_then(|o| registry.card_data(o.card_id))
        .map(|d| d.name)
        .unwrap_or_else(|| "?".into())
}

/// Apply an action to the game state and return the new state.
pub fn submit_action(state: &GameState, action: &Action, registry: &CardRegistry) -> GameState {
    let mut new_state = state.clone();
    new_state.events.clear();

    match action {
        Action::PassPriority => {
            let player = new_state.priority_player.unwrap();
            new_state.events.push(GameEvent::PriorityPassed { player });
            new_state.log(LogLevel::Debug, format!("p{} passes priority", player.0));
            new_state.consecutive_passes += 1;
        }

        Action::PlayLand { object_id } => {
            let player = new_state.priority_player.unwrap();
            new_state.move_object(*object_id, Zone::Battlefield);
            // Remove from library order if somehow there (shouldn't be, it's in hand).
            new_state.get_player_mut(player).land_plays_remaining -= 1;
            new_state.events.push(GameEvent::LandPlayed {
                player,
                object: *object_id,
            });
            new_state.events.push(GameEvent::EnteredBattlefield {
                object: *object_id,
                controller: player,
            });
            // Lands don't have summoning sickness (only creatures care).
            if let Some(obj) = new_state.get_object_mut(*object_id) {
                obj.summoning_sick = false;
            }
            let name = card_name(&new_state, registry, *object_id);
            new_state.log(LogLevel::Info, format!("p{} played {}", player.0, name));
            new_state.consecutive_passes = 0;
        }

        Action::CastSpell { object_id, targets } => {
            let player = new_state.priority_player.unwrap();

            // Pay mana cost.
            let card_id = new_state.get_object(*object_id).unwrap().card_id;
            let cost = registry.get(card_id).unwrap().card_data().cost.unwrap();
            mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &cost)
                .expect("legal_actions should have verified mana availability");

            // Move to stack and store targets.
            new_state.move_object(*object_id, Zone::Stack);
            new_state.get_object_mut(*object_id).unwrap().targets = targets.clone();
            new_state.stack.push(*object_id);

            new_state.events.push(GameEvent::SpellCast {
                player,
                object: *object_id,
            });

            let name = card_name(&new_state, registry, *object_id);
            new_state.log(LogLevel::Event, format!("p{} cast {}", player.0, name));
            new_state.consecutive_passes = 0;
        }

        Action::ActivateManaAbility { object_id, ability_index } => {
            let obj = new_state.get_object(*object_id).unwrap();
            let card_id = obj.card_id;
            let controller = obj.controller;

            if let Some(behavior) = registry.get(card_id) {
                let abilities = behavior.mana_abilities(&new_state, *object_id);
                if let Some(ability) = abilities.get(*ability_index) {
                    if ability.requires_tap {
                        new_state.get_object_mut(*object_id).unwrap().tapped = true;
                        new_state.events.push(GameEvent::Tapped { object: *object_id });
                    }
                    for &(mana_type, amount) in &ability.produced {
                        new_state.get_player_mut(controller).mana_pool.add(mana_type, amount);
                        new_state.events.push(GameEvent::ManaAdded {
                            player: controller,
                            mana_type,
                            amount,
                        });
                    }
                }
            }

            let name = card_name(&new_state, registry, *object_id);
            new_state.log(LogLevel::Debug, format!("p{} tapped {} for mana", controller.0, name));
        }

        Action::DeclareAttackers { attackers } => {
            if attackers.is_empty() {
                new_state.log(LogLevel::Debug, "No attackers declared".into());
            } else {
                let names: Vec<String> = attackers.iter()
                    .map(|(id, _)| card_name(state, registry, *id))
                    .collect();
                new_state.log(LogLevel::Event, format!("p{} declared attackers: {}", new_state.active_player.0, names.join(", ")));
            }
            combat::declare_attackers(&mut new_state, attackers);
            new_state.awaiting_action = None;
            new_state.consecutive_passes = 0;
        }

        Action::DeclareBlockers { assignments } => {
            // The defending player is the opponent of the active player.
            let defender = new_state.opponent(new_state.active_player);
            if assignments.is_empty() {
                new_state.log(LogLevel::Info, format!("p{} declared no blockers", defender.0));
            } else {
                let descs: Vec<String> = assignments.iter()
                    .map(|(b, a)| format!("{} blocks {}", card_name(state, registry, *b), card_name(state, registry, *a)))
                    .collect();
                new_state.log(LogLevel::Event, format!("p{} declared blockers: {}", defender.0, descs.join(", ")));
            }
            combat::declare_blockers(&mut new_state, assignments);
            new_state.awaiting_action = None;
            new_state.consecutive_passes = 0;
        }

        Action::DiscardCards { cards } => {
            let player = match &new_state.awaiting_action {
                Some(AwaitingAction::DiscardToHandSize { player, .. }) => *player,
                _ => new_state.active_player,
            };
            for &card_id in cards {
                new_state.events.push(GameEvent::Discarded { player, object: card_id });
                new_state.move_object(card_id, Zone::Graveyard);
            }
            new_state.awaiting_action = None;
        }

        Action::Concede => {
            if let Some(player) = new_state.priority_player {
                new_state.log(LogLevel::Milestone, format!("p{} concedes", player.0));
                new_state.get_player_mut(player).lost = true;
                new_state.events.push(GameEvent::PlayerLost {
                    player,
                    reason: crate::events::LossReason::Conceded,
                });
            }
        }
    }

    new_state
}

/// Set up a new game: create objects, shuffle libraries, draw opening hands.
pub fn setup_game(config: &GameConfig, registry: &CardRegistry) -> GameState {
    let num_players = config.player_names.len() as u8;
    let mut state = GameState::new(num_players);

    // Set starting life.
    for p in &mut state.players {
        p.life = config.starting_life;
    }

    // Create card objects for each player's deck.
    let mut rng = rand::thread_rng();
    for (player_idx, decklist) in config.decklists.iter().enumerate() {
        let player_id = PlayerId(player_idx as u8);
        let mut library_ids = Vec::new();

        for (card_name, count) in &decklist.entries {
            let card_id = registry.get_id_by_name(card_name)
                .unwrap_or_else(|| panic!("Unknown card: {}", card_name));

            let card_data = registry.card_data(card_id).unwrap();

            // Derive colors from mana cost.
            let colors: Vec<Color> = card_data.cost.as_ref()
                .map(|cost| {
                    let mut c = Vec::new();
                    for sym in &cost.symbols {
                        if let ManaSymbol::Colored(color) = sym {
                            if !c.contains(color) {
                                c.push(*color);
                            }
                        }
                    }
                    c
                })
                .unwrap_or_default();

            for _ in 0..*count {
                let obj_id = state.create_object(
                    card_id,
                    player_id,
                    Zone::Library,
                    card_data.power,
                    card_data.toughness,
                );
                state.get_object_mut(obj_id).unwrap().colors = colors.clone();
                library_ids.push(obj_id);
            }
        }

        // Shuffle the library.
        library_ids.shuffle(&mut rng);
        state.get_player_mut(player_id).library_order = library_ids;
    }

    // Draw opening hands (7 cards each).
    for player_idx in 0..num_players {
        let player_id = PlayerId(player_idx);
        draw_cards(&mut state, player_id, 7);
    }

    state.events.push(GameEvent::GameStarted);
    state.log(LogLevel::Milestone, format!("── Turn 1 (p0) ──"));
    state
}

/// Draw N cards for a player. Logs a single summary entry.
pub fn draw_cards(state: &mut GameState, player: PlayerId, count: usize) {
    let mut drawn = 0;
    for _ in 0..count {
        let card_id = {
            let player_state = state.get_player_mut(player);
            player_state.draw_top_card()
        };
        match card_id {
            Some(id) => {
                state.move_object(id, Zone::Hand);
                state.events.push(GameEvent::CardDrawn { player, object: id });
                drawn += 1;
            }
            None => {
                // Drew from empty library — SBA will catch it.
                break;
            }
        }
    }
    if drawn > 0 {
        if drawn == 1 {
            state.log(LogLevel::Info, format!("p{} drew a card", player.0));
        } else {
            state.log(LogLevel::Info, format!("p{} drew {} cards", player.0, drawn));
        }
    }
}

/// Advance the game by one step. Performs turn-based actions for the new step.
/// Returns the updated state.
pub fn advance_step(state: &mut GameState, registry: &CardRegistry) {
    // Empty mana pools between steps.
    for player in &mut state.players {
        if !player.mana_pool.is_empty() {
            state.events.push(GameEvent::ManaPoolEmptied { player: player.id });
            player.mana_pool.empty();
        }
    }

    let next = state.step.next();
    match next {
        Some(next_step) => {
            state.step = next_step;
        }
        None => {
            // End of turn: advance to next player's turn.
            let next_player = state.next_player(state.active_player);
            state.active_player = next_player;
            state.turn_number += 1;
            state.step = Step::Untap;
            state.is_first_turn = false;

            state.events.push(GameEvent::TurnStarted {
                player: next_player,
                turn: state.turn_number,
            });
            state.log(LogLevel::Milestone, format!("── Turn {} (p{}) ──", state.turn_number, next_player.0));
        }
    }

    state.events.push(GameEvent::StepStarted { step: state.step });
    state.log(LogLevel::Debug, format!("Step: {:?}", state.step));
    state.consecutive_passes = 0;

    // Perform turn-based actions for this step.
    perform_turn_based_actions(state, registry);
}

/// Perform automatic actions when entering a step.
fn perform_turn_based_actions(state: &mut GameState, _registry: &CardRegistry) {
    let active = state.active_player;

    match state.step {
        Step::Untap => {
            // Untap all permanents the active player controls.
            let to_untap: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, active)
                .iter()
                .filter(|o| o.tapped)
                .map(|o| o.id)
                .collect();

            for id in to_untap {
                state.get_object_mut(id).unwrap().tapped = false;
                state.events.push(GameEvent::Untapped { object: id });
            }

            // Clear summoning sickness for creatures the active player controls.
            let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, active)
                .iter()
                .filter(|o| o.summoning_sick)
                .map(|o| o.id)
                .collect();

            for id in creatures {
                state.get_object_mut(id).unwrap().summoning_sick = false;
            }

            // Reset land plays.
            state.get_player_mut(active).land_plays_remaining = 1;

            // No priority during untap step.
            state.priority_player = None;
        }

        Step::Draw => {
            // Active player draws a card (skip on the very first turn).
            if !state.is_first_turn {
                draw_cards(state, active, 1);
            }
            state.priority_player = Some(active);
        }

        Step::DeclareAttackers => {
            // Set up the awaiting action for attacker declaration.
            state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
            state.priority_player = Some(active);
        }

        Step::DeclareBlockers => {
            // Check if there are any attackers.
            let has_attackers = state.combat.as_ref()
                .map(|c| !c.attackers.is_empty())
                .unwrap_or(false);

            if has_attackers {
                let defending = state.opponent(active);
                state.awaiting_action = Some(AwaitingAction::DeclareBlockers {
                    defending_player: defending,
                });
                state.priority_player = Some(defending);
            } else {
                // No attackers, skip.
                state.priority_player = Some(active);
            }
        }

        Step::CombatDamage => {
            let has_attackers = state.combat.as_ref()
                .map(|c| !c.attackers.is_empty())
                .unwrap_or(false);

            if has_attackers {
                combat::deal_combat_damage(state);
            }
            // No priority in combat damage step for Phase 1.
            // (Technically there should be, but no instants yet.)
            state.priority_player = Some(active);
        }

        Step::EndCombat => {
            combat::end_combat(state);
            state.priority_player = Some(active);
        }

        Step::Cleanup => {
            // Remove damage from all creatures.
            let damaged: Vec<ObjectId> = state.all_objects_in_zone(Zone::Battlefield)
                .iter()
                .filter(|o| o.damage_marked > 0)
                .map(|o| o.id)
                .collect();

            for id in damaged {
                state.get_object_mut(id).unwrap().damage_marked = 0;
            }

            // Remove "until end of turn" effects.
            state.until_end_of_turn_effects.clear();

            // Empty mana pools.
            for player in &mut state.players {
                player.mana_pool.empty();
            }

            // Check hand size: max 7 cards.
            let hand_size = state.objects_in_zone(Zone::Hand, active).len();
            if hand_size > 7 {
                state.awaiting_action = Some(AwaitingAction::DiscardToHandSize {
                    player: active,
                    discard_count: hand_size - 7,
                });
                state.priority_player = Some(active);
            } else {
                state.priority_player = None; // No priority in cleanup normally.
            }
        }

        // All other steps: just give priority to active player.
        _ => {
            state.priority_player = Some(active);
        }
    }
}

/// The main game loop. Takes a mutable game state and player callbacks.
/// `choose_action` receives the game state, acting player, legal actions,
/// and an optional combat prompt. It returns the chosen Action.
pub fn run_game_loop<F>(
    state: &mut GameState,
    registry: &CardRegistry,
    mut choose_action: F,
) where
    F: FnMut(&GameState, PlayerId, &LegalActions) -> Action,
{
    // Start with the first turn's turn-based actions.
    state.events.push(GameEvent::TurnStarted {
        player: state.active_player,
        turn: state.turn_number,
    });
    perform_turn_based_actions(state, registry);

    let num_players = state.players.len() as u32;

    loop {
        if state.is_game_over() {
            break;
        }

        // Check SBAs before giving priority.
        while check_state_based_actions_with_registry(state, Some(registry)) {}
        if state.is_game_over() {
            break;
        }

        // If no one has priority and there's no awaiting action, advance step.
        if state.priority_player.is_none() && state.awaiting_action.is_none() {
            advance_step(state, registry);
            continue;
        }

        // Determine who needs to act.
        let acting_player = if let Some(AwaitingAction::DeclareBlockers { defending_player }) = &state.awaiting_action {
            *defending_player
        } else if let Some(AwaitingAction::DiscardToHandSize { player, .. }) = &state.awaiting_action {
            *player
        } else {
            match state.priority_player {
                Some(p) => p,
                None => {
                    advance_step(state, registry);
                    continue;
                }
            }
        };

        let legal = legal_actions(state, registry);
        if legal.actions.is_empty() && legal.combat_prompt.is_none() {
            advance_step(state, registry);
            continue;
        }

        let action = choose_action(state, acting_player, &legal);

        *state = submit_action(state, &action, registry);

        // After submitting, handle priority flow.
        match &action {
            Action::PassPriority => {
                if state.consecutive_passes >= num_players {
                    // All players passed in succession.
                    if !state.stack.is_empty() {
                        stack::resolve_top_of_stack(&mut state.clone(), registry);
                        let mut new_state = state.clone();
                        stack::resolve_top_of_stack(&mut new_state, registry);
                        *state = new_state;
                        state.consecutive_passes = 0;
                        state.priority_player = Some(state.active_player);
                    } else {
                        // Stack empty, advance step.
                        state.priority_player = None;
                        advance_step(state, registry);
                    }
                } else {
                    // Pass to next player.
                    let current = state.priority_player.unwrap();
                    state.priority_player = Some(state.next_player(current));
                }
            }

            Action::DeclareAttackers { attackers } => {
                if attackers.is_empty() {
                    // No attackers: skip to end of combat.
                    state.step = Step::EndCombat;
                    state.priority_player = Some(state.active_player);
                    combat::end_combat(state);
                } else {
                    // After declaring attackers, give priority to active player.
                    state.priority_player = Some(state.active_player);
                }
            }

            Action::DeclareBlockers { .. } => {
                // After declaring blockers, give priority to active player.
                state.priority_player = Some(state.active_player);
            }

            Action::DiscardCards { .. } => {
                // After discarding, cleanup continues (no priority).
                state.priority_player = None;
            }

            Action::ActivateManaAbility { .. } => {
                // Player retains priority. Don't change anything.
            }

            Action::Concede => {
                // SBAs will handle the game ending.
            }

            Action::PlayLand { .. } | Action::CastSpell { .. } => {
                // Player retains priority after these actions.
            }
        }
    }
}

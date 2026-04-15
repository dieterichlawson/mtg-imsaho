//! Shared test helpers.
//!
//! Each integration test file compiles as a separate binary, so helpers
//! only used by *other* test files appear dead in each binary. This is a
//! known Rust issue (rust-lang/rust#46379) with no upstream fix.
#![allow(dead_code, unused_imports)]

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::{CardId, ObjectId, PlayerId};
use mtg_engine::state::GameState;
use mtg_engine::types::*;

pub const P0: PlayerId = PlayerId(0);
pub const P1: PlayerId = PlayerId(1);

/// Set up a game state at a specific step with the given player as active and having priority.
pub fn game_at_step(step: Step, active: PlayerId) -> GameState {
    let mut state = GameState::new(2);
    state.step = step;
    state.active_player = active;
    state.priority_player = Some(active);
    state.is_first_turn = false;
    state.players[0].life = 20;
    state.players[1].life = 20;
    state
}

/// Place a creature on the battlefield that is ready to act (no summoning sickness).
pub fn ready_creature(state: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    let id = state.create_object(CardId(9999), owner, Zone::Battlefield, Some(power), Some(toughness));
    state.get_object_mut(id).unwrap().summoning_sick = false;
    id
}

/// Place a creature on the battlefield with summoning sickness.
pub fn sick_creature(state: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    state.create_object(CardId(9999), owner, Zone::Battlefield, Some(power), Some(toughness))
}

/// Put a named card into a player's hand. Returns the object ID.
pub fn spell_in_hand(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) -> ObjectId {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let data = registry.card_data(card_id);
    let power = data.as_ref().and_then(|d| d.power);
    let toughness = data.as_ref().and_then(|d| d.toughness);
    let id = state.create_object(card_id, player, Zone::Hand, power, toughness);
    state.get_object_mut(id).unwrap().name = name.into();
    id
}

/// Add exactly enough mana to a player's pool to pay for a card by name.
pub fn add_mana_for(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let data = registry.card_data(card_id)
        .unwrap_or_else(|| panic!("No card data for: {name}"));
    if let Some(ref cost) = data.cost {
        for sym in &cost.symbols {
            match sym {
                ManaSymbol::Colored(c) => {
                    let mana_type = match c {
                        Color::White => ManaType::White,
                        Color::Blue => ManaType::Blue,
                        Color::Black => ManaType::Black,
                        Color::Red => ManaType::Red,
                        Color::Green => ManaType::Green,
                    };
                    state.get_player_mut(player).mana_pool.add(mana_type, 1);
                }
                ManaSymbol::Generic(n) => {
                    state.get_player_mut(player).mana_pool.add(ManaType::Colorless, *n);
                }
                _ => {}
            }
        }
    }
}

/// Put a named card in hand and add mana to cast it. Returns the object ID.
pub fn castable_spell(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) -> ObjectId {
    let id = spell_in_hand(state, registry, name, player);
    add_mana_for(state, registry, name, player);
    id
}

/// Cast a spell targeting something, then resolve the top of the stack.
/// Returns the new state after resolution. For X-cost spells, funds X to
/// the max (taps every offered source + drains pool). Tests wanting a
/// specific X value should construct the `FundingResponse` themselves
/// instead of calling this helper.
pub fn cast_and_resolve(
    state: &GameState,
    registry: &CardRegistry,
    spell_id: ObjectId,
    targets: Vec<Target>,
) -> GameState {
    let new_state = mtg_engine::engine::submit_action(
        state,
        &Action::CastSpell { object_id: spell_id, targets, sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        registry,
    );
    // Resolve any follow-up casting prompts (exile-choice, then X-funding).
    // Order matters: exile-choice comes first since the engine sets up
    // ChooseExileFromGraveyard from the CastSpell handler, and only after
    // the cast recurses (via ChosenExileSet) might a ChooseXFunding
    // appear. No current card needs both chained, but the helper handles
    // the sequence defensively.
    let new_state = resolve_exile_choice_max_power(&new_state, registry);
    let mut new_state = resolve_funding_max(&new_state, registry);
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, registry);
    new_state
}

/// Resolve a pending `ChooseExileFromGraveyard` prompt (if any) by picking
/// the `max` highest-`effective_power` subset. For fixed-count costs this
/// is the count; for variable-count (Harvest Pyre) it's the maximum
/// possible X. Matches legacy "auto-pick max" behavior so tests that
/// previously relied on engine-side auto-pick keep working.
pub fn resolve_exile_choice_max_power(
    state: &GameState,
    registry: &CardRegistry,
) -> GameState {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseExileFromGraveyard { options, max, .. },
        ..
    }) = state.awaiting_action.clone() else {
        return state.clone();
    };
    // Sort by effective_power desc so Corpse Lunge et al. pick the big creature.
    let mut ranked: Vec<(ObjectId, i32)> = options.iter()
        .map(|&id| (id, state.effective_power(id, registry).unwrap_or(0)))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    let chosen: Vec<ObjectId> = ranked.into_iter().take(max).map(|(id, _)| id).collect();
    let action = Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(chosen) };
    mtg_engine::engine::submit_action(state, &action, registry)
}

/// Resolve a pending `ChooseXFunding` prompt (if any) by maxing out every
/// offered source and draining the pool. No-op when no such prompt is
/// pending. Used by `cast_and_resolve` and by tests that activate X-cost
/// abilities and don't care about a specific X value.
pub fn resolve_funding_max(state: &GameState, registry: &CardRegistry) -> GameState {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::funding::FundingResponse;
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { options, .. },
        ..
    }) = state.awaiting_action.clone() else {
        return state.clone();
    };
    let mut response = FundingResponse::default();
    for (mt, amt) in &options.pool {
        response.pool.insert(*mt, *amt);
    }
    for g in &options.groups {
        response.taps.insert(g.name.clone(), g.max_contribution());
    }
    let action = Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) };
    mtg_engine::engine::submit_action(state, &action, registry)
}

/// Place a named card on the battlefield, ready to act. Returns the object ID.
pub fn named_creature(
    state: &mut GameState,
    registry: &CardRegistry,
    name: &str,
    owner: PlayerId,
) -> ObjectId {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let data = registry.card_data(card_id)
        .unwrap_or_else(|| panic!("No card data for: {name}"));
    let is_legendary = data.supertypes.contains(&Supertype::Legendary);
    let id = state.create_object(card_id, owner, Zone::Battlefield, data.power, data.toughness);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.summoning_sick = false;
    obj.is_legendary = is_legendary;
    id
}

/// Process triggers, auto-resolving any "choose target player" choices by picking the opponent.
/// Repeats until all triggers and their choices are fully resolved.
pub fn process_triggers_auto_target_opponent(state: &mut GameState, registry: &CardRegistry) {
    for _ in 0..50 {
        mtg_engine::triggers::process_triggers(state, registry);
        if state.awaiting_action.is_none() {
            break;
        }
        // Auto-resolve: choose the opponent as target.
        let resolved = match &state.awaiting_action {
            Some(mtg_engine::state::AwaitingAction::ResolutionChoice { player, choice, .. }) => {
                let controller = *player;
                match choice {
                    mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. } => {
                        let opponent = state.opponent(controller);
                        options.iter()
                            .find(|t| matches!(t, Target::Player(p) if *p == opponent))
                            .or_else(|| options.first())
                            .cloned()
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(target) = resolved {
            let new_state = mtg_engine::engine::submit_action(
                state,
                &Action::ResolveChoice {
                    choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(target)),
                },
                registry,
            );
            *state = new_state;
        } else {
            break;
        }
    }
}

/// Count counters of a given type on `obj`. Returns 0 if the object has
/// no counters of that type (or doesn't exist). Prefer this over
/// reaching into `state.get_object(x).unwrap().counters.get(...)` — it
/// makes the intent clear and survives if the internal representation
/// of counters changes.
pub fn counters_of(state: &GameState, obj: ObjectId, kind: CounterType) -> u32 {
    state.get_object(obj)
        .and_then(|o| o.counters.get(&kind).copied())
        .unwrap_or(0)
}

/// Drive `Action::DeclareAttackers` through `engine::submit_action` (the
/// player-facing API). Preferred over `combat::declare_attackers` in tests
/// that exercise end-to-end gameplay, because it sets up the
/// `AwaitingAction`, fires events, and runs the forced-attacker pass.
pub fn submit_declare_attackers(
    state: &mut GameState,
    attackers: &[(ObjectId, PlayerId)],
    registry: &CardRegistry,
) {
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);
    *state = mtg_engine::engine::submit_action(
        state,
        &Action::DeclareAttackers { attackers: attackers.to_vec() },
        registry,
    );
}

/// Drive `Action::DeclareBlockers` through `engine::submit_action` (the
/// player-facing API). `defender` is the defending player (typically
/// `state.opponent(state.active_player)`).
pub fn submit_declare_blockers(
    state: &mut GameState,
    defender: PlayerId,
    assignments: &[(ObjectId, ObjectId)],
    registry: &CardRegistry,
) {
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareBlockers {
        defending_player: defender,
    });
    *state = mtg_engine::engine::submit_action(
        state,
        &Action::DeclareBlockers { assignments: assignments.to_vec() },
        registry,
    );
}

/// Place a named equipment card on the battlefield (unattached). Returns the object ID.
pub fn named_equipment(
    state: &mut GameState,
    registry: &CardRegistry,
    name: &str,
    owner: PlayerId,
) -> ObjectId {
    let id = named_creature(state, registry, name, owner);
    let obj = state.get_object_mut(id).unwrap();
    obj.is_equipment = true;
    id
}

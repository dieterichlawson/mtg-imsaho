//! Two independent gaps this file pins.
//!
//! A spell whose second target slot is "up to N" produced no cast action at
//! all — `valid_targets_for_req` had no `UpToTargets` branch, so the Cartesian
//! product with the first slot was always empty and Memory's Journey could
//! never be cast. And "from THEIR graveyard" was not enforced at announcement:
//! every graveyard was offered, so a player could declare targets from a third
//! party's and have them silently discarded at resolution (CR 601.2c).
//!
//! Separately, `CreatureCardMilled` is what Undead Alchemist watches, and only
//! `mill_cards` emitted it — so cards that moved library cards to the graveyard
//! by hand were invisible to it.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::GameState;
use mtg_engine::types::*;
/// Put a card into a player's library and return its id.
fn card_in_library(state: &mut GameState, reg: &CardRegistry, name: &str, owner: mtg_engine::ids::PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown {name}"));
    let data = reg.card_data(card_id).unwrap();
    let id = state.create_object(card_id, owner, Zone::Library, data.power, data.toughness);
    state.get_object_mut(id).unwrap().name = name.into();
    state.get_player_mut(owner).library_order.push(id);
    id
}

fn cast_actions_for(state: &GameState, reg: &CardRegistry, spell: ObjectId) -> Vec<Vec<Target>> {
    mtg_engine::engine::legal_actions(state, reg).actions.iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if *object_id == spell => Some(targets.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn memorys_journey_is_castable() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let spell = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    let actions = cast_actions_for(&state, &reg, spell);
    assert!(!actions.is_empty(),
        "Memory's Journey has an 'up to three' second target slot; with no \
         UpToTargets branch the Cartesian product was empty and the card could \
         never be cast at all");
}

/// "Up to three" includes zero — targeting just the player is a legal cast.
#[test]
fn memorys_journey_can_be_cast_with_no_card_targets() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let spell = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    let actions = cast_actions_for(&state, &reg, spell);
    assert!(actions.iter().any(|t| t.len() == 1 && matches!(t[0], Target::Player(_))),
        "'up to three' allows zero, so player-only is a legal announcement; \
         got {actions:?}");
}

/// "from THEIR graveyard": a card in someone else's graveyard is never offered
/// alongside a given player target.
#[test]
fn memorys_journey_only_offers_the_targeted_players_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mine = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let theirs = named_card_in_graveyard(&mut state, &reg, "Avacyn's Pilgrim", P1);
    let spell = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    let actions = cast_actions_for(&state, &reg, spell);
    // Guard against passing vacuously: there must actually BE announcements
    // that include a card, or the loop below proves nothing.
    let with_cards = actions.iter().filter(|t| t.len() > 1).count();
    assert!(with_cards > 0, "expected announcements that include card targets; got {actions:?}");

    for targets in &actions {
        let Some(Target::Player(pid)) = targets.first() else { continue };
        let wrong_owner = if *pid == P0 { theirs } else { mine };
        assert!(!targets.contains(&Target::Object(wrong_owner)),
            "targeting p{} must not offer a card from the other player's \
             graveyard; got {targets:?}", pid.0);
    }
}

// ── CreatureCardMilled from bespoke mill paths ───────────────────

fn milled_creature_events(state: &GameState) -> usize {
    state.events.iter()
        .filter(|e| matches!(e, GameEvent::CreatureCardMilled { .. }))
        .count()
}

/// Mulch puts the non-lands it reveals into the graveyard from the library —
/// that is a mill, and a creature among them must be visible to watchers.
#[test]
fn mulch_emits_creature_card_milled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..4 {
        card_in_library(&mut state, &reg, "Walking Corpse", P0);
    }
    let spell = castable_spell(&mut state, &reg, "Mulch", P0);

    state.events.clear();
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    assert!(milled_creature_events(&state) > 0,
        "Mulch put creature cards into the graveyard from the library, so \
         Undead Alchemist's watcher must see it");
}

/// Cellar Door mills from the BOTTOM, so it cannot use `mill_cards` — but it
/// is still a mill.
#[test]
fn cellar_door_emits_creature_card_milled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let door = named_creature(&mut state, &reg, "Cellar Door", P0);
    card_in_library(&mut state, &reg, "Walking Corpse", P1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    state.events.clear();
    let behavior = reg.get(state.get_object(door).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, door, 0, &[Target::Player(P1)], &reg);

    assert!(milled_creature_events(&state) > 0,
        "Cellar Door milled a creature card from the bottom of a library; \
         milling from the bottom is still milling");
}

//! End-to-end tests for the `ChooseXFunding` casting flow.
//!
//! Exercises the full path: cast an X-cost spell → engine puts up a
//! `ChooseXFunding` prompt → test submits a `FundingResponse` →
//! engine taps the chosen sources, drains the pool, sets `x_value`,
//! and fires `SpellCast`. Also verifies the rules-ordering fix:
//! `SpellCast` must not fire until funding completes.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::events::GameEvent;
use mtg_engine::funding::FundingResponse;
use mtg_engine::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use mtg_engine::types::*;

fn place_land(
    state: &mut GameState,
    registry: &CardRegistry,
    name: &str,
    owner: mtg_engine::ids::PlayerId,
) -> mtg_engine::ids::ObjectId {
    let card_id = registry
        .get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let id = state.create_object(card_id, owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.summoning_sick = false;
    id
}

fn cast_devils_play(state: &GameState, registry: &CardRegistry, dp: mtg_engine::ids::ObjectId) -> GameState {
    let legal = engine::legal_actions(state, registry);
    let cast_action = legal
        .actions
        .iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == dp))
        .expect("Devil's Play should be castable");
    engine::submit_action(state, cast_action, registry)
}

fn extract_funding(state: &GameState) -> &mtg_engine::funding::FundingOptions {
    match state.awaiting_action.as_ref() {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseXFunding { options, .. },
            ..
        }) => options,
        other => panic!("expected ChooseXFunding awaiting_action, got {other:?}"),
    }
}

#[test]
fn funding_prompt_is_presented_after_non_x_payment() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    for _ in 0..3 {
        place_land(&mut state, &registry, "Mountain", P0);
    }
    let _ = ready_creature(&mut state, P1, 2, 2);

    let post_cast = cast_devils_play(&state, &registry, dp);
    let options = extract_funding(&post_cast);
    // One Mountain was tapped for the {R} non-X pip. 2 remain.
    let mountain = options.groups.iter().find(|g| g.name == "Mountain").unwrap();
    assert_eq!(mountain.source_ids.len(), 2);
    assert_eq!(options.max_x, 2);
}

#[test]
fn x_equals_sum_of_funding_allocations() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    for _ in 0..5 {
        place_land(&mut state, &registry, "Mountain", P0);
    }
    let _ = ready_creature(&mut state, P1, 5, 5);

    let post_cast = cast_devils_play(&state, &registry, dp);
    let options = extract_funding(&post_cast);
    assert_eq!(options.max_x, 4);

    // Fund X = 3 by tapping 3 of the 4 remaining Mountains.
    let mut response = FundingResponse::default();
    response.taps.insert("Mountain".into(), 3);
    let action = Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) };
    let final_state = engine::submit_action(&post_cast, &action, &registry);

    let spell = final_state
        .objects_in_zone(Zone::Stack, P0)
        .into_iter()
        .find(|o| o.name == "Devil's Play")
        .expect("Devil's Play still on stack");
    assert_eq!(spell.x_value, Some(3));
}

#[test]
fn funding_prefers_pool_then_taps() {
    // Player has {R}{R} floating + 1 untapped Mountain. Cast Devil's Play.
    // Non-X uses {R} from pool. For X, prefer remaining pool over tapping.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 2);
    let mountain = place_land(&mut state, &registry, "Mountain", P0);
    let _ = ready_creature(&mut state, P1, 5, 5);

    let post_cast = cast_devils_play(&state, &registry, dp);
    let options = extract_funding(&post_cast);
    // Non-X {R} came from the pool. 1 R still floating. Mountain untapped.
    assert_eq!(options.max_x, 2);
    assert_eq!(options.pool.get(&ManaType::Red).copied(), Some(1));
    let g = options.groups.iter().find(|g| g.name == "Mountain").unwrap();
    assert_eq!(g.source_ids.len(), 1);

    // Fund X = 1 from the pool only; leave Mountain untapped.
    let mut response = FundingResponse::default();
    response.pool.insert(ManaType::Red, 1);
    let action = Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) };
    let final_state = engine::submit_action(&post_cast, &action, &registry);

    // Mountain should still be untapped; spell has X = 1.
    let m_obj = final_state.get_object(mountain).unwrap();
    assert!(!m_obj.tapped, "Mountain should remain untapped when pool covers X");
    let spell = final_state
        .objects_in_zone(Zone::Stack, P0)
        .into_iter()
        .find(|o| o.name == "Devil's Play")
        .unwrap();
    assert_eq!(spell.x_value, Some(1));
}

#[test]
fn x_equals_zero_is_legal_and_pool_unchanged() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    for _ in 0..3 {
        place_land(&mut state, &registry, "Mountain", P0);
    }
    let _ = ready_creature(&mut state, P1, 2, 2);

    let post_cast = cast_devils_play(&state, &registry, dp);

    // Submit an all-zero funding: X = 0.
    let action = Action::ResolveChoice { choice: ResolvedChoice::XFunding(FundingResponse::default()) };
    let final_state = engine::submit_action(&post_cast, &action, &registry);

    let spell = final_state
        .objects_in_zone(Zone::Stack, P0)
        .into_iter()
        .find(|o| o.name == "Devil's Play")
        .unwrap();
    assert_eq!(spell.x_value, Some(0));
    assert!(final_state.awaiting_action.is_none(), "awaiting_action should be cleared after funding");
}

#[test]
fn spellcast_event_fires_after_funding_not_before() {
    // Rules-ordering regression: the `SpellCast` event must not be emitted
    // until funding completes (CR 601.2b announce X → 601.2i spell becomes
    // cast). Before the refactor, `SpellCast` fired with x_value = None,
    // meaning any "on cast" trigger that looked at x_value saw 0.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    for _ in 0..3 {
        place_land(&mut state, &registry, "Mountain", P0);
    }
    let _ = ready_creature(&mut state, P1, 2, 2);

    let post_cast = cast_devils_play(&state, &registry, dp);
    let spellcast_before_funding = post_cast.events.iter().any(|e| matches!(e,
        GameEvent::SpellCast { object, .. } if *object == dp));
    assert!(
        !spellcast_before_funding,
        "SpellCast should not fire before funding completes"
    );

    let mut response = FundingResponse::default();
    response.taps.insert("Mountain".into(), 1);
    let action = Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) };
    let final_state = engine::submit_action(&post_cast, &action, &registry);

    let spellcast_after_funding = final_state.events.iter().any(|e| matches!(e,
        GameEvent::SpellCast { object, .. } if *object == dp));
    assert!(
        spellcast_after_funding,
        "SpellCast must fire after funding completes so on-cast triggers see the real X"
    );
    // Confirm x_value is set at the point SpellCast fires (not None).
    let spell = final_state
        .objects_in_zone(Zone::Stack, P0)
        .into_iter()
        .find(|o| o.name == "Devil's Play")
        .unwrap();
    assert_eq!(spell.x_value, Some(1));
}

#[test]
fn spells_cast_counter_increments_after_funding() {
    // The per-turn counter used by werewolf transforms etc. should only
    // increment once the spell formally becomes cast.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    for _ in 0..2 {
        place_land(&mut state, &registry, "Mountain", P0);
    }
    let _ = ready_creature(&mut state, P1, 2, 2);

    let post_cast = cast_devils_play(&state, &registry, dp);
    assert_eq!(
        post_cast.num_spells_cast_this_turn.get(&P0).copied().unwrap_or(0),
        0,
        "counter should not increment until funding completes"
    );

    let action = Action::ResolveChoice { choice: ResolvedChoice::XFunding(FundingResponse::default()) };
    let final_state = engine::submit_action(&post_cast, &action, &registry);
    assert_eq!(
        final_state.num_spells_cast_this_turn.get(&P0).copied().unwrap_or(0),
        1,
        "counter should increment exactly once after funding completes"
    );
}

#[test]
fn floating_plus_swamp_and_sac_cost_would_cast_if_supported() {
    // Regression scenario the user explicitly worried about:
    //   "1B floating, 1 tapped Swamp, 1 untapped Swamp, legal sac target,
    //    spell with cost {1}{B} and a sacrifice additional cost"
    // The autotap planner already handles this correctly via Phase 0
    // (deduct pool before planning). This test validates using a simpler
    // non-X stand-in: Altar's Reap ({1}{B}, sac a creature).
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let reap = spell_in_hand(&mut state, &registry, "Altar's Reap", P0);

    // Exactly one untapped Swamp + {B} floating.
    place_land(&mut state, &registry, "Swamp", P0);
    let tapped_sw = place_land(&mut state, &registry, "Swamp", P0);
    state.get_object_mut(tapped_sw).unwrap().tapped = true;
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);

    let _sac_target = ready_creature(&mut state, P0, 1, 1);

    let legal = engine::legal_actions(&state, &registry);
    let cast_action = legal
        .actions
        .iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == reap))
        .expect(
            "Altar's Reap ({1}{B}, sac) should be castable with {B} floating + 1 untapped Swamp + sac target",
        );
    let _new_state = engine::submit_action(&state, cast_action, &registry);
}

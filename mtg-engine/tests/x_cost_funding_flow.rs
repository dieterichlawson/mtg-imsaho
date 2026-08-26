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

fn cast_devils_play(state: &GameState, registry: &CardRegistry, dp: ObjectId) -> GameState {
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
        named_permanent(&mut state, &registry, "Mountain", P0);
    }
    let _ = ready_creature(&mut state, P1, 2, 2);

    let post_cast = cast_devils_play(&state, &registry, dp);
    let options = extract_funding(&post_cast);
    // The funding options reflect what's available for X *after* the
    // non-X portion ({R}) is paid — one Mountain's worth is reserved for
    // that, leaving 2 for X. Note: under rules-strict casting, no mana is
    // actually paid yet (see rules_strict_spell_stays_in_hand below); the
    // prompt just shows the post-payment availability the agent should
    // reason about.
    let mountain = options.groups.iter().find(|g| g.name == "Mountain").unwrap();
    assert_eq!(mountain.source_ids.len(), 2);
    assert_eq!(options.max_x, 2);
}

#[test]
fn rules_strict_spell_stays_in_hand_until_funding_completes() {
    // CR 601.2h → 601.2i: additional costs and mana are paid BEFORE the
    // spell becomes cast. While the ChooseXFunding prompt is pending,
    // the spell must still be in hand (not on stack), no mana should be
    // tapped, and no SpellCast event should have fired.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    let mountains: Vec<_> = (0..3)
        .map(|_| named_permanent(&mut state, &registry, "Mountain", P0))
        .collect();
    let _ = ready_creature(&mut state, P1, 2, 2);

    let post_cast = cast_devils_play(&state, &registry, dp);

    // Spell still in hand — not on stack.
    assert_eq!(
        post_cast.get_object(dp).map(|o| o.zone),
        Some(Zone::Hand),
        "spell should stay in hand until funding completes"
    );
    assert!(
        post_cast.stack.is_empty(),
        "stack should be empty while ChooseXFunding is pending"
    );
    // Mountains still untapped — tap_plan hasn't executed.
    for m in &mountains {
        assert!(
            !post_cast.get_object(*m).unwrap().tapped,
            "Mountain should remain untapped during the funding prompt"
        );
    }
    // No mana paid yet.
    assert_eq!(post_cast.get_player(P0).mana_pool.total(), 0);
    // SpellCast must not have fired.
    assert!(!post_cast.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::SpellCast { object, .. } if *object == dp)));
    // pending_spell_cast should be set.
    assert!(post_cast.pending_spell_cast.is_some());

    // After funding: spell on stack, mountains tapped, SpellCast fired.
    let mut response = FundingResponse::default();
    response.taps.insert("Mountain".into(), 2);
    let action = Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) };
    let final_state = engine::submit_action(&post_cast, &action, &registry);

    assert_eq!(
        final_state.get_object(dp).map(|o| o.zone),
        Some(Zone::Stack),
        "spell should be on stack after funding completes"
    );
    // All 3 Mountains tapped (1 for {R}, 2 for X=2).
    for m in &mountains {
        assert!(
            final_state.get_object(*m).unwrap().tapped,
            "all Mountains should be tapped after funding resolves"
        );
    }
    assert!(final_state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::SpellCast { object, .. } if *object == dp)));
    assert!(final_state.pending_spell_cast.is_none());
}

#[test]
fn x_equals_sum_of_funding_allocations() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    for _ in 0..5 {
        named_permanent(&mut state, &registry, "Mountain", P0);
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
    let mountain = named_permanent(&mut state, &registry, "Mountain", P0);
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
        named_permanent(&mut state, &registry, "Mountain", P0);
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
        named_permanent(&mut state, &registry, "Mountain", P0);
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
        named_permanent(&mut state, &registry, "Mountain", P0);
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

/// The autotap planner has to spend floating mana before it decides which
/// lands to tap: with {B} in the pool, one untapped Swamp and one tapped one,
/// Altar's Reap ({1}{B}, sacrifice a creature) is payable — and paying it must
/// leave the pool empty, the untapped Swamp tapped, and the creature
/// sacrificed.
///
/// This used to be named `..._would_cast_if_supported`, and asserted only that
/// the cast was offered before dropping the resulting state on the floor.
#[test]
fn floating_mana_is_spent_before_lands_are_tapped() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let reap = spell_in_hand(&mut state, &registry, "Altar's Reap", P0);

    let untapped = named_permanent(&mut state, &registry, "Swamp", P0);
    let already_tapped = named_permanent(&mut state, &registry, "Swamp", P0);
    state.get_object_mut(already_tapped).unwrap().tapped = true;
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);

    let sac_target = ready_creature(&mut state, P0, 1, 1);

    assert!(can_cast(&state, &registry, reap),
        "{{B}} floating plus one untapped Swamp covers {{1}}{{B}}");

    let legal = engine::legal_actions(&state, &registry);
    let cast = legal.actions.iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == reap))
        .expect("the cast the assertion above just found")
        .clone();
    let state = engine::submit_action(&state, &cast, &registry);

    assert_eq!(state.get_object(reap).unwrap().zone, Zone::Stack, "the spell was cast");
    assert!(state.get_object(untapped).unwrap().tapped,
        "the untapped Swamp paid the generic half");
    assert_eq!(state.get_player(P0).mana_pool.total(), 0,
        "and the floating {{B}} paid the coloured half rather than being left behind");
    assert_eq!(state.get_object(sac_target).unwrap().zone, Zone::Graveyard,
        "the additional cost was paid too (CR 601.2h)");
}

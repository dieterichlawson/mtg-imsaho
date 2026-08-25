//! Copy effects: what gets copied, what may be chosen, and which abilities
//! trigger when the copy arrives.
//!
//! CR 707.2 defines the copiable values; CR 614.12 says a permanent that
//! "enters as a copy" enters already bearing them, so the abilities that
//! trigger on it entering are the COPIED creature's; and CR 115.1 says an
//! effect targets only where the word "target" appears — so "a copy of any
//! creature on the battlefield" is a choice, not a target.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::{GameState, PendingEffect};
use mtg_engine::types::*;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};


fn copy_onto(state: &mut GameState, reg: &CardRegistry, copier: ObjectId, victim: ObjectId) {
    mtg_engine::engine::apply_pending_effect(
        state, &Target::Object(victim),
        &PendingEffect::CopyCreature { source_id: copier }, reg);
}

// ── Evil Twin: choosing is not targeting ─────────────────────────

/// CR 115.1: hexproof and protection restrict TARGETING. Evil Twin's copy is
/// a choice (CR 614.12b), so an opponent's hexproof creature is fair game.
#[test]
fn evil_twin_may_copy_a_hexproof_creature_it_could_not_target() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_creature(&mut state, &reg, "Evil Twin", P0);
    let hexproof = named_creature(&mut state, &reg, "Walking Corpse", P1);
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: hexproof, keyword: Keyword::Hexproof,
    });
    assert!(state.has_keyword(hexproof, Keyword::Hexproof, &reg), "test precondition");

    // Drive Evil Twin's own enters-the-battlefield handler, so this covers the
    // card's candidate list rather than the helper in isolation.
    let behavior = reg.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &reg);

    let options = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => options.clone(),
        other => panic!("Evil Twin should offer a copy choice, got {other:?}"),
    };

    assert!(options.contains(&Target::Object(hexproof)),
        "an opponent's hexproof creature can't be TARGETED but can be chosen \
         to copy — 'any creature on the battlefield' has no 'target' in it; \
         offered {options:?}");
}

/// A generic token's printed keywords live on the object, not in the
/// registry, so copying one used to drop them.
#[test]
fn copying_a_token_preserves_its_keywords() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_creature(&mut state, &reg, "Evil Twin", P0);
    let token = *state.create_token_with_subtypes(
        "Spirit", P1, 1, 1, vec![Color::White], vec![CardType::Creature],
        vec![Keyword::Flying], vec!["Spirit".into()], &reg)
        .first().expect("token created");
    assert!(state.has_keyword(token, Keyword::Flying, &reg), "test precondition");

    copy_onto(&mut state, &reg, twin, token);

    assert!(state.has_keyword(twin, Keyword::Flying, &reg),
        "the copy has the token's flying — a generic token has no registry \
         entry, so its printed keywords are the ones on the object");
}

/// CR 614.12: the permanent enters as the copy, so the copied creature's
/// enters-the-battlefield ability is the one that triggers.
#[test]
fn a_copy_fires_the_copied_creatures_etb_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_creature(&mut state, &reg, "Evil Twin", P0);
    // Fiend Hunter's ETB exiles a creature — an unmistakable ability.
    let hunter = named_creature(&mut state, &reg, "Fiend Hunter", P1);

    state.pending_triggers.clear();
    copy_onto(&mut state, &reg, twin, hunter);

    let queued = state.pending_triggers.iter().any(|t| matches!(t,
        PendingTrigger { source: TriggerSource { id: object_id, .. }, event: TriggerEvent::SelfEntered }
        if *object_id == twin));
    assert!(queued,
        "copying a creature with an enters-the-battlefield ability must raise \
         that ability for the copy (CR 614.12); it was silently lost");
}

/// Copying something with no ETB ability raises nothing.
#[test]
fn a_copy_of_a_vanilla_creature_raises_no_etb_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_creature(&mut state, &reg, "Evil Twin", P0);
    let vanilla = named_creature(&mut state, &reg, "Walking Corpse", P1);

    state.pending_triggers.clear();
    copy_onto(&mut state, &reg, twin, vanilla);

    assert!(state.pending_triggers.is_empty(),
        "Walking Corpse has no enters-the-battlefield ability, so nothing \
         should be queued; got {:?}", state.pending_triggers);
}

// ── Cackling Counterpart ─────────────────────────────────────────

/// The basic case, which had no coverage at all: a token copy of a creature
/// you control, with the copied creature's characteristics.
#[test]
fn cackling_counterpart_creates_a_token_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let original = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let spell = castable_spell(&mut state, &reg, "Cackling Counterpart", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(original)]);

    let copies: Vec<&mtg_engine::state::GameObject> = state.objects.values()
        .filter(|o| o.is_token && o.zone == Zone::Battlefield)
        .collect();
    assert_eq!(copies.len(), 1, "exactly one token copy should exist");

    let token = copies[0].id;
    assert_eq!(state.name_of(token, &reg), "Avacyn's Pilgrim",
        "the token copies the creature's name");
    assert!(state.has_subtype(token, "Human", &reg),
        "and its subtypes; subtypes_of = {:?}", state.subtypes_of(token, &reg));
    assert_eq!(state.effective_power(token, &reg), state.effective_power(original, &reg),
        "and its power");
}

/// The token is a copy of the card, so the copied creature's ETB ability
/// triggers when the token enters.
#[test]
fn a_token_copy_fires_the_copied_creatures_etb_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_creature(&mut state, &reg, "Mentor of the Meek", P0);
    let original = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);

    // A 1/1 token entering under P0's control satisfies Mentor's condition.
    state.stack.clear();
    let token = state.create_token_copy(original, P0, &reg);
    assert_eq!(state.name_of(token, &reg), "Avacyn's Pilgrim", "test precondition");
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    let mentor_triggered = state.stack.iter().any(|e| matches!(e,
        mtg_engine::state::StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: watcher_id, .. },
                event: TriggerEvent::CreatureEntered { .. },
            })
        if *watcher_id == mentor));
    assert!(mentor_triggered,
        "a token copy entering the battlefield is a creature entering, and \
         watchers must see it");
}

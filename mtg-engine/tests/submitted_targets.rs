//! CR 601.2c on the way *in*: the targets a player hands the engine.
//!
//! `legal_actions` enumerates only legal target sets, and for a long time that
//! was the whole of the enforcement — every submit path took the list it was
//! given. That is not a theoretical gap: neither client picks a whole offered
//! action. Both `mtg-player`'s CLI and its LLM driver assemble their own
//! action from per-slot choices, so the list the engine receives is one it
//! never built.
//!
//! Five card audits each met a different face of it — Corpse Lunge, Unburial
//! Rites, Purify the Grave, Travel Preparations, Rage Thrower — and each was
//! patched where it was found. These are the three submit paths, checked as
//! one rule: casting a spell, activating an ability, and answering a
//! resolution-time target prompt.
//!
//! The refusal is a no-op, not a partial action: it happens before any cost is
//! paid, because an illegal choice means the thing did not happen rather than
//! that it happened for nothing.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::types::*;

/// Casting: a spell whose targets were never legal does not go on the stack,
/// and nothing is paid for it.
///
/// Bump in the Night is "target opponent loses 3 life"; the opponent here has
/// hexproof from Witchbane Orb, so they were never an offerable target.
#[test]
fn a_cast_with_a_target_that_was_never_legal_does_not_happen() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Witchbane Orb", P1);

    let bump = castable_spell(&mut state, &reg, "Bump in the Night", P0);
    let mana_before = state.get_player(P0).mana_pool.clone();
    let their_life = state.get_player(P1).life;

    let state = cast_onto_stack(&state, &reg, bump, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(bump).unwrap().zone, Zone::Hand,
        "the spell never left the hand");
    assert!(state.stack.is_empty(), "and nothing went on the stack");
    assert_eq!(state.get_player(P0).mana_pool, mana_before,
        "and no mana was paid — an illegal choice means the cast did not \
         happen, not that it happened for nothing");
    assert_eq!(state.get_player(P1).life, their_life);
}

/// Activating: an ability whose target was never legal is refused, and the
/// activation cost is not paid.
///
/// Avacynian Priest is "{1}, {T}: Tap target non-Human creature", declared as
/// `TargetRequirement::Creature` with the non-Human half in the card's own
/// `is_valid_target`. A land fails the first half, which is the generic one.
#[test]
fn an_activation_with_a_target_that_was_never_legal_does_not_happen() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    let land = named_permanent(&mut state, &reg, "Forest", P1);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 1)]);
    let mana_before = state.get_player(P0).mana_pool.clone();

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: priest,
            ability_index: 0,
            targets: vec![Target::Object(land)],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
            source_card_id: None,
        },
        &reg,
    );

    assert!(!state.get_object(priest).unwrap().tapped,
        "the tap cost was not paid");
    assert_eq!(state.get_player(P0).mana_pool, mana_before,
        "and neither was the mana");
    assert!(state.stack.is_empty(), "and nothing went on the stack");
}

/// Answering a prompt: a target that was not among the options offered is not
/// an answer, and the prompt stays open.
///
/// Rage Thrower's "target player or planeswalker" is the one this was found
/// on — a creature is not either, and submitting one used to be taken at face
/// value.
#[test]
fn a_choice_answered_with_something_never_offered_is_refused() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _thrower = named_permanent(&mut state, &reg, "Rage Thrower", P0);
    let bystander = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let victim = ready_creature(&mut state, P1, 1, 1);

    kill_by_damage(&mut state, &reg, victim);
    mtg_engine::triggers::process_triggers(&mut state, &reg);
    assert!(state.awaiting_action.is_some(), "test setup: the trigger asks for a target");

    let life_before = state.get_player(P1).life;
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(bystander))),
        },
        &reg,
    );

    assert!(state.awaiting_action.is_some(),
        "a creature was never offered, so the prompt is still waiting");
    assert_eq!(state.get_object(bystander).unwrap().damage_marked, 0,
        "and nothing was damaged");
    assert_eq!(state.get_player(P1).life, life_before);
}

/// And an answer of the wrong *shape* is refused the same way: the question
/// asked for a target, so a yes/no does not answer it.
///
/// This used to fall through the match silently — the prompt was taken off
/// the state and dropped, which resumed a resolution that never got its
/// choice.
#[test]
fn a_choice_answered_in_the_wrong_shape_is_refused() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _thrower = named_permanent(&mut state, &reg, "Rage Thrower", P0);
    let victim = ready_creature(&mut state, P1, 1, 1);

    kill_by_damage(&mut state, &reg, victim);
    mtg_engine::triggers::process_triggers(&mut state, &reg);
    assert!(state.awaiting_action.is_some(), "test setup: the trigger asks for a target");

    let life_before = state.get_player(P1).life;
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    assert!(state.awaiting_action.is_some(),
        "the target question is still unanswered");
    assert_eq!(state.get_player(P1).life, life_before);
}

/// A player is not a creature, and "target creature" has to say so.
///
/// The requirement decides what *kind* of thing a target can be, and the
/// player arm of the re-check used to ask only whether the player could be
/// targeted at all — so every creature-, permanent-, card- and spell-shaped
/// requirement accepted a player. Nothing offers such a target, which is
/// exactly why it went unnoticed: it is only reachable through a submitted
/// one, and both clients submit their own.
///
/// Rebuke is "Destroy target attacking creature", declared `CreatureWithFilter`.
#[test]
fn a_player_is_not_a_legal_target_for_a_spell_that_wants_a_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P1);
    state.priority_player = Some(P1);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    attacks_unblocked(&mut state, attacker, P1);
    let rebuke = castable_spell(&mut state, &reg, "Rebuke", P1);
    let life_before = state.get_player(P0).life;

    let state = cast_onto_stack(&state, &reg, rebuke, vec![Target::Player(P0)]);

    assert_eq!(state.get_object(rebuke).unwrap().zone, Zone::Hand,
        "the spell never left the hand");
    assert!(state.stack.is_empty());
    assert_eq!(state.get_player(P0).life, life_before);
    assert_eq!(state.get_object(attacker).unwrap().zone, Zone::Battlefield,
        "and nothing else was destroyed in its place");
}

/// The same rule for the rest of the resolution choices, which ask for a card
/// or an index rather than a target.
///
/// `ChooseTarget` was the arm the audits kept arriving at, and it was the only
/// one checked. Its siblings each carry the set they offered — the cards
/// revealed, the player whose hand it is, the list of names — and each took
/// the answer on trust.
mod other_choices {
    use super::*;

    /// Forbidden Alchemy: "Look at the top four cards of your library. Put one
    /// **of them** into your hand." Answering with a fifth card was a tutor —
    /// name anything in your library and it came to hand.
    #[test]
    fn a_card_that_was_not_revealed_cannot_be_the_one_you_keep() {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let library = stock_library(&mut state, &reg, P0, 4);
        // A fifth card, under the four the spell will look at.
        let buried = spell_in_hand(&mut state, &reg, "Brimstone Volley", P0);
        state.move_object(buried, Zone::Library, &reg);
        state.get_player_mut(P0).library_order.push(buried);

        let alchemy = castable_spell(&mut state, &reg, "Forbidden Alchemy", P0);
        let state = cast_and_resolve(&state, &reg, alchemy, vec![]);
        assert!(state.awaiting_action.is_some(), "test setup: it asks which to keep");

        let state = mtg_engine::engine::submit_action(
            &state,
            &Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::ChosenCard(buried),
            },
            &reg,
        );

        assert_eq!(state.get_object(buried).unwrap().zone, Zone::Library,
            "the fifth card was never looked at, so it is not one to keep");
        assert!(state.awaiting_action.is_some(), "and the question still stands");
        for id in &library {
            assert_eq!(state.get_object(*id).unwrap().zone, Zone::Library,
                "nor did the rest go to the graveyard on a refused answer");
        }
    }

    /// "That player discards a card" — theirs, out of their hand. Not a card
    /// from their library, and not one of yours.
    ///
    /// The prompt is raised through `engine::discard_cards`, which is the one
    /// place every "discards N cards" in the set goes through, rather than
    /// through a particular card: the rule under test is the answer, not the
    /// asker.
    #[test]
    fn a_card_outside_the_hand_cannot_be_the_one_discarded() {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        // Two cards in hand, so there is a real choice and a real prompt.
        let _a = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
        let _b = spell_in_hand(&mut state, &reg, "Doomed Traveler", P1);
        let in_their_library = spell_in_hand(&mut state, &reg, "Brimstone Volley", P1);
        state.move_object(in_their_library, Zone::Library, &reg);
        state.get_player_mut(P1).library_order.push(in_their_library);
        let source = named_permanent(&mut state, &reg, "Brain Weevil", P0);

        mtg_engine::engine::discard_cards(&mut state, P1, 1, source, "test", &reg);
        assert!(state.awaiting_action.is_some(), "test setup: it asks them to discard");

        let state = mtg_engine::engine::submit_action(
            &state,
            &Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::ChosenCard(in_their_library),
            },
            &reg,
        );

        assert_eq!(state.get_object(in_their_library).unwrap().zone, Zone::Library,
            "a card in the library is not a card in hand");
        assert!(state.awaiting_action.is_some(), "and the question still stands");
    }
}

/// The same rule for a mana ability: a submitted activation for a source that
/// cannot pay {T} — tapped, or a summoning-sick creature — produces nothing.
///
/// Offer and submit share `available_mana_abilities`, so this holds by
/// construction; the test pins the sharing, because a submit path that read
/// the card's raw ability list instead would make phantom mana.
#[test]
fn a_mana_ability_submitted_for_a_source_that_cannot_pay_produces_nothing() {
    let reg = registry();

    // Summoning-sick Pilgrim.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let card_id = reg.get_id_by_name("Avacyn's Pilgrim").unwrap();
    let sick = state.create_object(card_id, P0, Zone::Battlefield, Some(1), Some(1));
    let state2 = mtg_engine::engine::submit_action(&state,
        &Action::ActivateManaAbility { object_id: sick, ability_index: 0 }, &reg);
    assert_eq!(state2.get_player(P0).mana_pool.get(ManaType::White), 0,
        "a summoning-sick creature cannot pay {{T}} (CR 302.6)");
    assert!(!state2.get_object(sick).unwrap().tapped, "and it was not tapped either");

    // Tapped Pilgrim.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
    state.tap(pilgrim);
    let state3 = mtg_engine::engine::submit_action(&state,
        &Action::ActivateManaAbility { object_id: pilgrim, ability_index: 0 }, &reg);
    assert_eq!(state3.get_player(P0).mana_pool.get(ManaType::White), 0,
        "an already-tapped source cannot pay {{T}} again");
}

/// The same rule for an additional cost: what the caster names to pay with
/// must really be theirs to pay (CR 601.2h, CR 701.17a).
///
/// Found at Altar's Reap: the offer path never proposes an opponent's
/// creature, but the submit path sacrificed whatever id it was handed —
/// removal stapled to a draw spell.
#[test]
fn a_sacrifice_cost_cannot_be_paid_with_an_opponents_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 2, 2);
    let theirs = ready_creature(&mut state, P1, 2, 2);
    let spell = castable_spell(&mut state, &reg, "Altar's Reap", P0);
    let mana_before = state.get_player(P0).mana_pool.clone();

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: spell,
            targets: vec![],
            sacrifice: Some(theirs),
            exile_count: None,
            exile_ids: vec![],
            alternative_cost: None,
            tap_plan: vec![],
        },
        &reg,
    );

    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Battlefield,
        "their creature is not yours to sacrifice");
    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Battlefield,
        "and nothing was taken in its place");
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Hand,
        "the cast did not happen");
    assert_eq!(state.get_player(P0).mana_pool, mana_before, "and nothing was paid");
}

/// And for an exile cost: Corpse Lunge's "exile a creature card from your
/// graveyard" cannot be paid out of the opponent's.
#[test]
fn an_exile_cost_cannot_be_paid_from_an_opponents_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _mine = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);
    let target = ready_creature(&mut state, P1, 1, 4);
    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: spell,
            targets: vec![Target::Object(target)],
            sacrifice: None,
            exile_count: None,
            exile_ids: vec![theirs],
            alternative_cost: None,
            tap_plan: vec![],
        },
        &reg,
    );

    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Graveyard,
        "their card stays in their graveyard");
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Hand,
        "the cast did not happen");
}

/// CR 601.2h: a cast whose submitted funding (pool + tap plan) cannot pay the
/// mana cost is refused with the state untouched. The engine used to panic
/// here ("legal_actions should have verified mana availability") — but neither
/// client submits a whole offered action, so the submit path must speak for
/// itself.
#[test]
fn a_cast_submitted_without_funding_is_refused() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let bears = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0, "precondition: broke");

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: bears, targets: vec![], sacrifice: None,
            exile_count: None, exile_ids: vec![], alternative_cost: None,
            tap_plan: vec![],
        },
        &reg,
    );

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Hand,
        "an unfunded cast is refused, not executed (and not a panic)");
    assert_eq!(state.get_player(P0).mana_pool.total(), 0, "nothing was drained");
}

/// The activation twin: an ability whose mana cost the pool cannot cover is
/// refused before anything is deducted or tapped (CR 601.2h via 602.2b).
#[test]
fn an_activation_submitted_without_funding_is_refused() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Gavony Township's second ability costs {2}{G}{W}, {T}.
    let township = named_permanent(&mut state, &reg, "Gavony Township", P0);
    let creature = ready_creature(&mut state, P0, 1, 1);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0, "precondition: broke");

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: township, ability_index: 1, targets: vec![],
            tap_plan: vec![], sacrifice: None, x_value: None, source_card_id: None,
        },
        &reg,
    );

    assert!(!state.get_object(township).unwrap().tapped,
        "the refused activation must not tap the Township");
    assert!(state.get_object(creature).unwrap().counters.is_empty(),
        "and no counters were handed out");
}

/// Stony Silence: "Activated abilities of artifacts can't be activated" —
/// mana abilities included (2017-03-14 ruling). `legal_actions` never offers
/// them, but a submitted action must be refused on its own merits.
mod stony_silence_submits {
    use super::*;

    #[test]
    fn a_submitted_artifact_mana_ability_is_refused() {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.priority_player = Some(P0);

        let ring = named_permanent(&mut state, &reg, "Sol Ring", P0);
        named_permanent(&mut state, &reg, "Stony Silence", P1);

        let state = mtg_engine::engine::submit_action(
            &state,
            &Action::ActivateManaAbility { object_id: ring, ability_index: 0 },
            &reg,
        );

        assert_eq!(state.get_player(P0).mana_pool.total(), 0,
            "no mana from an artifact ability under Stony Silence");
        assert!(!state.get_object(ring).unwrap().tapped, "and the Ring is not tapped");
    }

    #[test]
    fn a_submitted_tap_plan_naming_an_artifact_source_cannot_fund_a_cast() {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.priority_player = Some(P0);

        let ring = named_permanent(&mut state, &reg, "Sol Ring", P0);
        named_permanent(&mut state, &reg, "Stony Silence", P1);
        let bears = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

        let state = mtg_engine::engine::submit_action(
            &state,
            &Action::CastSpell {
                object_id: bears, targets: vec![], sacrifice: None,
                exile_count: None, exile_ids: vec![], alternative_cost: None,
                tap_plan: vec![(ring, 0)],
            },
            &reg,
        );

        assert_eq!(state.get_object(bears).unwrap().zone, Zone::Hand,
            "the silenced Ring produces nothing, so the cast is refused whole");
        assert!(!state.get_object(ring).unwrap().tapped, "the Ring was not tapped");
    }

    /// Equip has a colon: it is an activated ability of an artifact, and the
    /// same ruling shuts it off.
    #[test]
    fn a_submitted_equip_is_refused() {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.priority_player = Some(P0);

        let cleaver = named_permanent(&mut state, &reg, "Butcher's Cleaver", P0);
        let creature = ready_creature(&mut state, P0, 2, 2);
        named_permanent(&mut state, &reg, "Stony Silence", P1);
        state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

        let state = mtg_engine::engine::submit_action(
            &state,
            &Action::ActivateAbility {
                object_id: cleaver, ability_index: 0, targets: vec![Target::Object(creature)],
                tap_plan: vec![], sacrifice: None, x_value: None, source_card_id: None,
            },
            &reg,
        );

        assert_eq!(state.get_object(cleaver).unwrap().attached_to, None,
            "equip is an activated ability of an artifact — refused");
        assert_eq!(state.get_player(P0).mana_pool.total(), 3, "no mana was charged");
    }
}

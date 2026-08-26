//! Double-faced cards: which face is being read.
//!
//! CR 711.5: transforming does not create a new object, so everything about the
//! permanent stays put — its counters, its attachments, whether it is tapped —
//! while every printed characteristic comes from the other face. The failures
//! in this file were all the same shape underneath: something read the front
//! face, or read *a* Ranger rather than *this* one.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::types::*;

/// Every double-faced card in the set: the keywords the game reports are the
/// active face's, on both sides.
///
/// Derived from the registry rather than hand-listed, and it checks the
/// *absence* of the other face's keywords as well — the failure it protects
/// against is a front-face keyword surviving a transform, which only shows up
/// as an absence.
#[test]
fn a_transformed_card_reports_its_back_faces_keywords_and_not_its_front_faces() {
    let reg = registry();
    let mut checked = 0;

    let mut names: Vec<String> = reg.all_names().iter().map(|s| (*s).to_string()).collect();
    names.sort();
    for name in names {
        let card_id = reg.get_id_by_name(&name).expect("named card has an id");
        let Some(behavior) = reg.get(card_id) else { continue };
        let Some(back) = behavior.back_face_data() else { continue };
        let front = behavior.card_data();
        if front.keywords == back.keywords {
            continue; // nothing to tell apart
        }

        let mut state = game_at_step(Step::PrecombatMain, P0);
        let id = named_permanent(&mut state, &reg, &name, P0);

        for kw in &front.keywords {
            assert!(state.has_keyword(id, *kw, &reg),
                "{name} (front) should have its own {kw:?}");
        }
        for kw in back.keywords.iter().filter(|k| !front.keywords.contains(k)) {
            assert!(!state.has_keyword(id, *kw, &reg),
                "{name} (front): {kw:?} is printed on the back face only");
        }

        mtg_engine::cards::helpers::apply_transform(&mut state, id, &reg);
        assert!(state.get_object(id).unwrap().is_transformed, "{name}: transformed");

        for kw in &back.keywords {
            assert!(state.has_keyword(id, *kw, &reg),
                "{name} (back, {}) should have its own {kw:?}", back.name);
        }
        for kw in front.keywords.iter().filter(|k| !back.keywords.contains(k)) {
            assert!(!state.has_keyword(id, *kw, &reg),
                "{name}: {kw:?} is printed on the front face only, so it must not \
                 survive the transform into {}", back.name);
        }
        checked += 1;
    }

    assert!(checked >= 3,
        "expected the set's double-faced cards with differing keywords to be \
         found and checked, got {checked}");
}

/// A card's ability has to read *its own* face, not that of another copy.
///
/// Daybreak Ranger's two faces have different target filters — front is "target
/// creature with flying", back ("Nightfall Predator") is any creature — and the
/// front-face handler used to find the controller's Rangers with `.find()` and
/// read whichever one came back first. With two out in different states, the
/// answer depended on hash order, so this runs the check repeatedly: a correct
/// implementation gives the same answer every time.
#[test]
fn one_daybreak_rangers_transform_state_does_not_leak_into_anothers_targeting() {
    let reg = registry();

    for run in 0..30 {
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let front_face = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);
        let transformed = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);
        mtg_engine::cards::helpers::apply_transform(&mut state, transformed, &reg);

        let non_flying = ready_creature(&mut state, P1, 2, 2);

        let offered = engine::legal_actions(&state, &reg).actions.iter().any(|a| matches!(
            a, Action::ActivateAbility { object_id, ability_index: 0, targets, .. }
            if *object_id == front_face
                && targets.iter().any(|t| matches!(t, Target::Object(id) if *id == non_flying))));

        assert!(!offered,
            "run {run}: the front face's '{{T}}: deals 2 damage to target creature \
             with flying' must not offer a non-flying creature, whatever state a \
             second Ranger happens to be in");
    }
}

/// "When Garruk Relentless has two or fewer loyalty counters on him, transform
/// him" is a state-triggered ability (CR 603.8). Damage that takes him from 3
/// to 0 makes the condition true on the way past, so the transform preempts the
/// zero-loyalty state-based action that would otherwise bury him (CR 704.5i).
#[test]
fn garruk_transforms_rather_than_dying_when_loyalty_drops_straight_to_zero() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    assert_eq!(counters_of(&state, garruk, CounterType::Loyalty), 3, "test setup");

    set_loyalty(&mut state, garruk, 0);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(garruk).map(|o| o.zone), Some(Zone::Battlefield),
        "the state-triggered transform is queued before the zero-loyalty SBA \
         could bury him, so he is still here");

    // The trigger then goes on the stack and resolves like any other.
    mtg_engine::triggers::process_triggers(&mut state, &reg);
    assert!(state.get_object(garruk).unwrap().is_transformed,
        "and he is Garruk, the Veil-Cursed now");
}

/// "At the beginning of your upkeep, look at the top card of your library. You
/// may reveal that card. If an instant or sorcery card is revealed this way,
/// transform Delver of Secrets."
///
/// Ruling: "You may reveal the card even if it's not an instant or sorcery."
/// The choice is offered on what the card says, not on whether taking it would
/// achieve anything.
#[test]
fn delver_of_secrets_offers_the_reveal_whatever_is_on_top() {
    let reg = registry();
    // (card on top of the library, does revealing it transform the Delver)
    const TOPS: &[(&str, bool)] = &[("Think Twice", true), ("Grizzly Bears", false)];

    for &(top, transforms) in TOPS {
        let mut state = game_at_step(Step::Upkeep, P0);
        let delver = named_permanent(&mut state, &reg, "Delver of Secrets", P0);

        let card_id = reg.get_id_by_name(top).unwrap();
        let id = state.create_object(card_id, P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.insert(0, id);

        let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
        behavior.on_upkeep(&mut state, delver, &[], &reg);

        assert!(state.awaiting_action.is_some(),
            "with {top} on top the reveal is still offered — the ruling says you \
             may reveal whatever is there");

        behavior.on_yes_no_choice(&mut state, delver, true, &reg);
        assert_eq!(state.get_object(delver).unwrap().is_transformed, transforms,
            "{top}: revealing it transforms the Delver only if it is an instant \
             or sorcery");
    }
}

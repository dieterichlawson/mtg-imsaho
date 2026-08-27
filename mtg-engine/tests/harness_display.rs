//! What the player is shown, as opposed to what the game knows.
//!
//! An LLM player sees the `GameView` and the labels on the actions offered to
//! it, and can only reason about what is in them. Three ways that has gone
//! wrong: printed P/T shown for a creature whose P/T is a
//! characteristic-defining ability (CR 208.2 — a CDA works in every zone),
//! the front-face name shown for a transformed card, and internal object
//! handles rendered into a label with `{:?}`.

mod common;
use common::*;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

/// The view reports effective P/T wherever the card is.
///
/// Geist-Honored Monk's "power and toughness are each equal to the number of
/// creatures you control" is a CDA, so it has a real size in the graveyard and
/// in hand as well as on the battlefield — and that is the number the player
/// needs in order to decide whether reanimating it is worth anything.
#[test]
fn the_view_shows_effective_power_in_every_zone() {
    let reg = registry();

    for zone in [Zone::Battlefield, Zone::Graveyard, Zone::Hand] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        // Two other creatures out, so the Monk's count differs from its
        // printed 0/0 in every zone.
        named_permanent(&mut state, &reg, "Grizzly Bears", P0);
        named_permanent(&mut state, &reg, "Grizzly Bears", P0);

        let card_id = reg.get_id_by_name("Geist-Honored Monk").unwrap();
        let monk = state.create_object(card_id, P0, zone, Some(0), Some(0));
        state.get_object_mut(monk).unwrap().name = "Geist-Honored Monk".into();
        state.get_object_mut(monk).unwrap().summoning_sick = false;

        let expected = state.effective_power(monk, &reg).expect("a CDA has a value");
        assert!(expected >= 2,
            "test precondition: in {zone:?} the value is {expected}, not the printed 0");

        let view = mtg_engine::view::GameView::for_player(&state, P0, &reg);
        let shown = match zone {
            Zone::Battlefield => view.battlefield.iter()
                .find(|c| c.object_id == monk).and_then(|c| c.effective_power),
            Zone::Graveyard => view.graveyards.iter()
                .find(|(pid, _)| *pid == P0)
                .and_then(|(_, cards)| cards.iter().find(|c| c.object_id == monk))
                .and_then(|c| c.power),
            _ => view.your_hand.iter().find(|c| c.object_id == monk).and_then(|c| c.power),
        };

        assert_eq!(shown, Some(expected),
            "in {zone:?} the view must show the Monk's effective power, not the \
             printed 0 — a CDA works in every zone (CR 208.2)");
    }
}

/// A transformed card's trigger is labelled with the face that is showing.
/// The battlefield says "Rampaging Werewolf"; a stack entry saying "Tormented
/// Pariah" describes a permanent the player cannot see.
#[test]
fn a_transformed_cards_trigger_label_names_the_face_that_is_showing() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let pariah = named_permanent(&mut state, &reg, "Tormented Pariah", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, pariah, &reg);
    let card_id = state.get_object(pariah).unwrap().card_id;

    let trigger = PendingTrigger {
        source: TriggerSource::new(pariah, card_id, P0, "transform back if 2+ spells cast"),
        event: TriggerEvent::Upkeep,
    };
    let label = trigger.display_name_with_state(&reg, Some(&state));

    assert!(label.contains("Rampaging Werewolf"),
        "the label names the face that is on the battlefield; label = {label:?}");
    assert!(!label.contains("Tormented Pariah"),
        "and not the front face, which names a permanent that is not there; \
         label = {label:?}");
}

/// No ability label anywhere in the set renders an internal handle.
///
/// `ObjectId(5)` means nothing to a player — there is no way to map it back to
/// a creature. Skirsdag High Priest's tap-pair labels used to be built with
/// `{:?}`; this checks every card rather than that one, over a board with
/// enough going on for the enumerating abilities to enumerate something.
#[test]
fn no_ability_label_renders_an_internal_object_id() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;

    let mut names: Vec<String> = reg.all_names().iter().map(|s| (*s).to_string()).collect();
    names.sort();
    for name in names {
        let card_id = reg.get_id_by_name(&name).expect("named card has an id");
        let Some(behavior) = reg.get(card_id) else { continue };

        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.creature_died_this_turn = true; // unlock the morbid ones
        let id = named_permanent(&mut state, &reg, &name, P0);
        // Fodder for abilities that enumerate creatures or graveyard cards.
        for _ in 0..3 {
            ready_creature(&mut state, P0, 2, 2);
            named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
        }
        ready_creature(&mut state, P1, 2, 2);

        for ability in behavior.activated_abilities(&state, id, &reg) {
            checked += 1;
            if ability.description.contains("ObjectId(") {
                offenders.push(format!("{name}: {:?}", ability.description));
            }
        }
    }

    assert!(checked >= 20,
        "expected to have looked at a good number of ability labels, got {checked}");
    assert!(offenders.is_empty(),
        "{} ability label(s) render an internal handle the player cannot map to \
         anything:\n  {}", offenders.len(), offenders.join("\n  "));
}

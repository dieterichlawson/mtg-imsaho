//! Paying for things the mana pool can't see yet.
//!
//! Two places where the engine decided a cost was unpayable by looking at the
//! wrong thing: the X in an X-cost flashback, and the mana a player *could*
//! produce as opposed to what is floating right now (CR 106.4 — pools empty
//! between steps, so at an upkeep trigger the pool is always empty).
//!
//! A third lived here — Corpse Lunge exiling a Boneyard Wurm — until the cost
//! stopped recording a power at all. What it was about now sits with the rest
//! of that card in `cards_sacrifice_and_additional_costs.rs`.

mod common;
use common::*;

use mtg_engine::types::*;

/// "Flashback {X}{R}{R}{R}" (Devil's Play). The flashback path autotapped for
/// the whole printed cost including the {X}, which is not a number, so the
/// planner returned nothing and the cast was silently dropped from the action
/// list — with any amount of mana available.
#[test]
fn an_x_cost_flashback_is_offered_when_the_non_x_part_is_payable() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils = named_card_in_graveyard(&mut state, &reg, "Devil's Play", P0);
    ready_creature(&mut state, P1, 2, 2); // something for "any target"

    for n in 0..7 {
        assert_eq!(can_cast(&state, &reg, devils), n >= 3,
            "with {n} Mountain(s), the {{R}}{{R}}{{R}} of {{X}}{{R}}{{R}}{{R}} is \
             {} payable", if n >= 3 { "" } else { "not" });
        named_permanent(&mut state, &reg, "Mountain", P0);
    }
}

/// "At the beginning of your upkeep, you may pay {2}{B}{B}. If you do,
/// transform Screeching Bat."
///
/// CR 106.4: mana pools empty between steps, so at an upkeep trigger the pool
/// is always empty. Asking whether the player can pay by looking at the pool
/// therefore always answers no, and the option was never offered — however many
/// untapped Swamps they controlled.
#[test]
fn a_may_pay_prompt_counts_mana_the_player_could_produce() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);
    for _ in 0..4 {
        named_permanent(&mut state, &reg, "Swamp", P0);
    }
    assert_eq!(state.get_player(P0).mana_pool.total(), 0,
        "test precondition: nothing floating, which is the normal state at upkeep");

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);

    match state.awaiting_action.as_ref() {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::YesNo { description, source_card }, ..
        }) => {
            assert_eq!(*source_card, bat);
            assert!(description.contains("Swamp") || description.contains("tap"),
                "the prompt names the lands it would tap, so the player can weigh \
                 the opportunity cost; got {description:?}");
        }
        other => panic!("the may-pay prompt should be offered with four untapped \
                         Swamps out; got {other:?}"),
    }

    behavior.on_yes_no_choice(&mut state, bat, true, &reg);
    assert!(state.get_object(bat).unwrap().is_transformed, "saying yes transforms it");
    assert_eq!(state.objects.values().filter(|o| o.name == "Swamp" && o.tapped).count(), 4,
        "and taps the lands that paid for it");
}

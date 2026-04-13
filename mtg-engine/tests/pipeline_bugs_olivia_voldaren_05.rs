mod common;

use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use common::{game_at_step, named_creature, ready_creature, P0, P1};

/// Olivia Voldaren ruling (2017-03-14): "If you activate Olivia Voldaren's last
/// ability, and before that ability resolves you lose control of Olivia Voldaren,
/// the ability will resolve with no effect. You won't gain control of the targeted
/// Vampire."
///
/// When Olivia has left the battlefield before her {3}{B}{B} ability resolves,
/// the activating player no longer controls Olivia, so the ability should resolve
/// with no effect. The code in olivia_voldaren.rs:133-163 does not check whether
/// Olivia is still on the battlefield. It reads her controller (line 94) even from
/// the graveyard and proceeds to change the target Vampire's controller.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn olivia_control_steal_does_nothing_when_olivia_left_battlefield() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Olivia on the battlefield under P0's control.
    let olivia = named_creature(&mut state, &registry, "Olivia Voldaren", P0);

    // Place a 2/2 Vampire on the battlefield under P1's control.
    let vampire = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(vampire).unwrap().subtypes.push("Vampire".to_string());

    // Sanity: the Vampire starts under P1's control.
    assert_eq!(
        state.get_object(vampire).unwrap().controller, P1,
        "Sanity: vampire should start under P1's control"
    );

    // Olivia is destroyed — she moves to the graveyard. This simulates the
    // scenario where Olivia leaves the battlefield between activation and
    // resolution of her control-steal ability.
    state.move_object(olivia, Zone::Graveyard, &registry);
    assert_eq!(
        state.get_object(olivia).unwrap().zone, Zone::Graveyard,
        "Sanity: Olivia should be in the graveyard"
    );

    // Invoke Olivia's ability 1 resolution directly. In a correct implementation
    // this should check that the activating player still controls Olivia on the
    // battlefield, and do nothing if not.
    let card_id = registry.get_id_by_name("Olivia Voldaren").unwrap();
    let behavior = registry.get(card_id).unwrap();
    behavior.on_activate_ability(
        &mut state,
        olivia,
        1, // ability index 1 = {3}{B}{B} gain control of target Vampire
        &[Target::Object(vampire)],
        &registry,
    );

    // Per the 2017-03-14 ruling: since the activating player (P0) no longer
    // controls Olivia on the battlefield, the ability should resolve with no
    // effect. The Vampire must remain under P1's control.
    assert_eq!(
        state.get_object(vampire).unwrap().controller, P1,
        "Ruling: Olivia's control-steal ability should resolve with no effect \
         when Olivia is not on the battlefield — vampire must remain under P1's control"
    );
}

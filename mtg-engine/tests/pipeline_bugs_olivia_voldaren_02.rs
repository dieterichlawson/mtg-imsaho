mod common;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use common::{game_at_step, named_creature, P0, P1};

/// Olivia Voldaren's second ability: "{3}{B}{B}: Gain control of target Vampire
/// for as long as you control Olivia Voldaren."
///
/// Bug: At resolution time (olivia_voldaren.rs:136-138), the ability checks
/// `obj.subtypes.contains("Vampire")` to decide whether to steal the target.
/// For inherent Vampires like Stromkirk Noble, `obj.subtypes` is empty — their
/// Vampire subtype lives exclusively in `registry.card_data().subtypes`.
/// The targeting filter `HasSubtype` (engine.rs:1896-1900) correctly checks BOTH
/// sources, so inherent Vampires can be targeted. But the resolution check only
/// looks at `obj.subtypes`, so the ability resolves and does nothing.
///
/// Oracle says: if the target is a legal Vampire target, the ability should
/// gain control of it. Stromkirk Noble is a Vampire creature and should be
/// stolen.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn olivia_steal_fails_on_inherent_vampire() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Olivia on P0's battlefield.
    let olivia = named_creature(&mut state, &registry, "Olivia Voldaren", P0);

    // Place Stromkirk Noble (an inherent Vampire) on P1's battlefield.
    let noble = named_creature(&mut state, &registry, "Stromkirk Noble", P1);

    // Sanity: Stromkirk Noble's inherent subtype is Vampire per registry.
    let card_id = state.get_object(noble).unwrap().card_id;
    let card_data = registry.card_data(card_id).unwrap();
    assert!(
        card_data.subtypes.contains(&"Vampire".to_string()),
        "Sanity: Stromkirk Noble is a Vampire in registry card data"
    );

    // Sanity: obj.subtypes is empty (the root cause of the bug).
    assert!(
        state.get_object(noble).unwrap().subtypes.is_empty(),
        "Sanity: obj.subtypes is empty for inherent Vampires — subtypes come from registry"
    );

    // Add mana for Olivia's ability 1: {3}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    // Activate Olivia's ability 1 targeting Stromkirk Noble.
    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: olivia,
            ability_index: 1,
            targets: vec![Target::Object(noble)],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
        },
        &registry,
    );

    // Oracle text: "{3}{B}{B}: Gain control of target Vampire for as long as
    // you control Olivia Voldaren." Stromkirk Noble is a Vampire, so P0
    // should now control it.
    assert_eq!(
        new_state.get_object(noble).unwrap().controller, P0,
        "Olivia's steal ability should gain control of inherent Vampire Stromkirk Noble, \
         but resolution-time check only reads obj.subtypes (empty for real cards) \
         and misses the Vampire subtype in registry card data"
    );
}

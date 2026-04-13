mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Bug: When a transformed DFC leaves the battlefield, move_object (state.rs:494-505)
/// resets is_transformed to false but does NOT reset obj.name. Since obj_name() reads
/// obj.name directly with no registry-based fallback, the card retains its back-face
/// name in the graveyard.
///
/// MTG Rule 712.10: "A double-faced card not on the battlefield or on the stack has
/// only the characteristics of its front face."
///
/// After Garruk transforms into "Garruk, the Veil-Cursed" and then dies, he should
/// revert to "Garruk Relentless" in the graveyard. The engine currently keeps the
/// back-face name because on_state_trigger sets obj.name = "Garruk, the Veil-Cursed"
/// (garruk_relentless.rs:308) and move_object never clears it.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn transformed_garruk_should_revert_name_when_leaving_battlefield() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Garruk Relentless on the battlefield with 2 loyalty counters.
    let garruk_card_id = registry.get_id_by_name("Garruk Relentless").unwrap();
    let garruk = state.create_object(garruk_card_id, P0, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(garruk).unwrap();
        obj.name = "Garruk Relentless".into();
        obj.card_types = vec![CardType::Planeswalker];
        obj.is_legendary = true;
    }
    state.add_counters(garruk, CounterType::Loyalty, 2);

    // Simulate transformation: this is exactly what on_state_trigger does when
    // Garruk has 2 or fewer loyalty (garruk_relentless.rs:305-312).
    {
        let obj = state.get_object_mut(garruk).unwrap();
        obj.is_transformed = true;
        obj.name = "Garruk, the Veil-Cursed".into();
    }

    // Verify Garruk is transformed on the battlefield.
    let obj = state.get_object(garruk).unwrap();
    assert!(obj.is_transformed, "Garruk should be transformed");
    assert_eq!(obj.name, "Garruk, the Veil-Cursed");
    assert_eq!(obj.zone, Zone::Battlefield);

    // Garruk dies — move to graveyard (as happens when loyalty reaches 0 via SBA,
    // or when destroyed by an effect).
    state.move_object(garruk, Zone::Graveyard, &registry);

    // Verify Garruk is in the graveyard and is_transformed was cleared.
    let obj = state.get_object(garruk).unwrap();
    assert_eq!(obj.zone, Zone::Graveyard);
    assert!(!obj.is_transformed, "is_transformed should be cleared when leaving battlefield");

    // Rule 712.10: A DFC not on the battlefield has only its front-face characteristics.
    // Garruk's name in the graveyard should be "Garruk Relentless" (the front face),
    // not "Garruk, the Veil-Cursed" (the back face).
    assert_eq!(
        obj.name, "Garruk Relentless",
        "Rule 712.10: transformed DFC should revert to front-face name ('Garruk Relentless') \
         when leaving the battlefield, but move_object only clears is_transformed without \
         resetting obj.name — the back-face name 'Garruk, the Veil-Cursed' persists"
    );
}

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Bug: Garruk Relentless's 0-ability ("Garruk deals 3 damage to target creature")
/// applies damage inline via `obj.damage_marked += 3` instead of using the central
/// PendingEffect::DealDamage handler. The central handler checks for
/// PreventDamageRemoveCounter replacement effects (e.g., Unbreathing Horde) and
/// protection from source. The inline path bypasses both checks.
///
/// Oracle text: "0: Garruk deals 3 damage to target creature. That creature deals
/// damage equal to its power to him."
///
/// Per MTG rules, if a creature has "prevent damage, remove counter" replacement
/// (Unbreathing Horde), Garruk's 3 damage should be prevented and a +1/+1 counter
/// removed instead. The current code ignores this and applies 3 damage directly.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn garruk_relentless_damage_bypasses_prevent_damage_remove_counter() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Garruk Relentless on the battlefield for P0 with 3 loyalty.
    let garruk_card_id = registry.get_id_by_name("Garruk Relentless").unwrap();
    let garruk = state.create_object(garruk_card_id, P0, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(garruk).unwrap();
        obj.name = "Garruk Relentless".into();
        obj.card_types = vec![CardType::Planeswalker];
    }
    state.add_counters(garruk, CounterType::Loyalty, 3);

    // Create a creature for P1 with the PreventDamageRemoveCounter effect
    // (simulating Unbreathing Horde's static ability) and 3 +1/+1 counters.
    // When this creature would be dealt damage, the damage is prevented and a
    // +1/+1 counter is removed instead.
    let creature = ready_creature(&mut state, P1, 2, 2);
    {
        let obj = state.get_object_mut(creature).unwrap();
        obj.name = "Unbreathing Horde Stand-in".into();
        obj.instance_continuous_effects = Some(vec![
            ContinuousEffect::PreventDamageRemoveCounter {
                scope: EffectScope::OnSelf,
            },
        ]);
    }
    state.add_counters(creature, CounterType::PlusOnePlusOne, 3);

    let counters_before = state.get_counter_count(creature, CounterType::PlusOnePlusOne);
    assert_eq!(counters_before, 3, "Creature should start with 3 +1/+1 counters");

    // Activate Garruk's 0-ability targeting the creature.
    let new_state = engine::submit_action(
        &state,
        &Action::ActivateLoyaltyAbility {
            object_id: garruk,
            ability_index: 0,
            targets: vec![Target::Object(creature)],
        },
        &registry,
    );

    // Oracle-correct behavior: damage should be PREVENTED and a +1/+1 counter
    // removed instead. The creature should have 0 damage_marked.
    let creature_obj = new_state.get_object(creature).expect("Creature should still exist");
    assert_eq!(
        creature_obj.damage_marked, 0,
        "PreventDamageRemoveCounter should prevent Garruk's 3 damage, \
         but inline damage_marked += 3 bypasses the central DealDamage handler"
    );

    // A +1/+1 counter should have been removed (3 -> 2).
    let counters_after = new_state.get_counter_count(creature, CounterType::PlusOnePlusOne);
    assert_eq!(
        counters_after, 2,
        "PreventDamageRemoveCounter should remove a +1/+1 counter when preventing damage, \
         but the inline damage path never checks for this replacement effect"
    );
}

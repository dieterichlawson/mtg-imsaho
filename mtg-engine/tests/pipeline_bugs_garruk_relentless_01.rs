mod common;
use common::*;

use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// CR 702.16e: protection prevents all damage from sources with the matching quality.
// Garruk's ability 0 deals 3 damage inline (damage_marked += 3), skipping has_protection_from.
#[test]
fn garruk_damage_respects_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.get_object_mut(garruk).unwrap().card_types = vec![CardType::Planeswalker];
    state.add_counters(garruk, CounterType::Loyalty, 3);
    state.get_object_mut(garruk).unwrap().subtypes.push("Green".into());

    let creature = ready_creature(&mut state, P1, 2, 4);
    state.get_object_mut(creature).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Green".into(),
            scope: EffectScope::OnSelf,
        },
    ]);

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 0, &[Target::Object(creature)], &reg);

    assert_eq!(
        state.get_object(creature).unwrap().damage_marked, 0,
        "CR 702.16e: creature with protection from green takes 0 damage from Garruk"
    );
}

// CR 702.15b: a source with lifelink causes its controller to gain life
// equal to the damage dealt. Garruk's ability 0 has the creature deal power
// as damage to Garruk by directly decrementing loyalty counters, bypassing
// the lifelink check in apply_pending_effect.
#[test]
fn garruk_fight_creature_lifelink_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.get_object_mut(garruk).unwrap().card_types = vec![CardType::Planeswalker];
    state.add_counters(garruk, CounterType::Loyalty, 3);

    let creature = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(creature).unwrap().keywords.push(Keyword::Lifelink);

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 0, &[Target::Object(creature)], &reg);

    assert_eq!(
        state.players[1].life, 22,
        "CR 702.15b: opponent gains 2 life from lifelink creature dealing 2 damage to Garruk"
    );
}

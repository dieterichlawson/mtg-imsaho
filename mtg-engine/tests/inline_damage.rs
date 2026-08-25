mod common;
use common::*;

use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::{CardId, ObjectId, PlayerId};
use mtg_engine::state::GameState;
use mtg_engine::types::*;
fn make_planeswalker(state: &mut GameState, owner: PlayerId, name: &str, loyalty: u32) -> ObjectId {
    let id = state.create_object(CardId(9998), owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.card_types = vec![CardType::Planeswalker];
    obj.counters.insert(CounterType::Loyalty, loyalty);
    obj.summoning_sick = false;
    id
}

fn spell_on_stack(state: &mut GameState, registry: &CardRegistry, name: &str, owner: PlayerId) -> ObjectId {
    let card_id = registry.get_id_by_name(name).unwrap();
    let id = state.create_object(card_id, owner, Zone::Stack, None, None);
    state.get_object_mut(id).unwrap().name = name.into();
    id
}

// CR 702.16e: protection prevents all damage from sources with the matching quality.
// Balefire Dragon's on_combat_damage_to_player writes damage_marked inline, skipping has_protection_from.
#[test]
fn test_balefire_dragon_respects_protection_from_red() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let balefire = named_creature(&mut state, &reg, "Balefire Dragon", P0);

    let protected = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(protected).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Dragon".into(),
            scope: EffectScope::OnSelf,
        },
    ]);

    let behavior = reg.get(state.get_object(balefire).unwrap().card_id).unwrap();
    behavior.on_combat_damage_to_player(&mut state, balefire, P1, 6, &reg);

    assert_eq!(
        state.get_object(protected).unwrap().damage_marked, 0,
        "CR 702.16e: creature with protection from Dragons takes 0 damage from Balefire Dragon trigger"
    );
}

// CR 702.16e: Olivia's first ability deals 1 damage inline without checking protection.
// Protection from Vampire (Olivia's subtype) should prevent damage and block dependent effects.
#[test]
fn test_olivia_voldaren_first_ability_respects_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_creature(&mut state, &reg, "Olivia Voldaren", P0);

    let target = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(target).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Vampire".into(),
            scope: EffectScope::OnSelf,
        },
    ]);

    let behavior = reg.get(state.get_object(olivia).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, olivia, 0, &[Target::Object(target)], &reg);

    assert_eq!(
        state.get_object(target).unwrap().damage_marked, 0,
        "CR 702.16e: creature with protection from Vampire takes no damage from Olivia"
    );
    assert!(
        !state.get_object(target).unwrap().subtypes.contains(&"Vampire".to_string()),
        "protection from source should prevent Olivia from adding Vampire subtype"
    );
    assert_eq!(
        counters_of(&state, olivia, CounterType::PlusOnePlusOne), 0,
        "Olivia should not gain a +1/+1 counter when target has protection"
    );
}

// CR 702.16e: Daybreak Ranger deals 2 damage to a flying creature inline.
// A creature with protection from Human (Ranger's subtype) should take 0 damage.
#[test]
fn test_daybreak_ranger_ability_respects_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ranger = named_creature(&mut state, &reg, "Daybreak Ranger", P0);

    let target = ready_creature(&mut state, P1, 2, 4);
    {
        let obj = state.get_object_mut(target).unwrap();
        obj.keywords.push(Keyword::Flying);
        obj.instance_continuous_effects = Some(vec![
            ContinuousEffect::ProtectionFromSubtype {
                subtype: "Human".into(),
                scope: EffectScope::OnSelf,
            },
        ]);
    }

    let behavior = reg.get(state.get_object(ranger).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, ranger, 0, &[Target::Object(target)], &reg);

    assert_eq!(
        state.get_object(target).unwrap().damage_marked, 0,
        "CR 702.16e: creature with protection from Human takes no damage from Daybreak Ranger"
    );
}

// CR 120.3c: damage to a planeswalker removes loyalty counters, not damage_marked.
// Devil's Play uses resolve_damage which always writes damage_marked, even for planeswalkers.
#[test]
fn test_devils_play_removes_planeswalker_loyalty() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pw = make_planeswalker(&mut state, P1, "Test Planeswalker", 4);

    let spell = spell_on_stack(&mut state, &reg, "Devil's Play", P0);
    state.get_object_mut(spell).unwrap().x_value = Some(3);

    let behavior = reg.get(state.get_object(spell).unwrap().card_id).unwrap();
    behavior.on_resolve(&mut state, spell, &[Target::Object(pw)], &reg);

    assert_eq!(
        counters_of(&state, pw, CounterType::Loyalty), 1,
        "CR 120.3c: 3 damage to a 4-loyalty planeswalker should leave 1 loyalty counter"
    );
    assert_eq!(
        state.get_object(pw).unwrap().damage_marked, 0,
        "damage to planeswalkers removes loyalty counters, not damage_marked"
    );
}

// CR 702.16e: Into the Maw of Hell deals 13 damage inline, bypassing protection.
#[test]
fn test_into_the_maw_of_hell_respects_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = named_creature(&mut state, &reg, "Mountain", P1);

    let protected = ready_creature(&mut state, P1, 3, 15);
    state.get_object_mut(protected).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Red".into(),
            scope: EffectScope::OnSelf,
        },
    ]);

    let spell = spell_on_stack(&mut state, &reg, "Into the Maw of Hell", P0);
    state.get_object_mut(spell).unwrap().subtypes.push("Red".into());

    let behavior = reg.get(state.get_object(spell).unwrap().card_id).unwrap();
    behavior.on_resolve(
        &mut state, spell,
        &[Target::Object(land), Target::Object(protected)],
        &reg,
    );

    assert_eq!(
        state.get_object(protected).unwrap().damage_marked, 0,
        "CR 702.16e: creature with protection from red takes 0 damage from Into the Maw of Hell"
    );
}

// CR 702.16e: Blasphemous Act deals 13 to each creature inline.
// A creature with protection from the source should take 0 damage.
#[test]
fn blasphemous_act_protection_from_red_prevents_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let protected = ready_creature(&mut state, P1, 3, 15);
    state.get_object_mut(protected).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Red".into(),
            scope: EffectScope::OnSelf,
        },
    ]);

    let unprotected = ready_creature(&mut state, P1, 3, 15);

    let spell = spell_on_stack(&mut state, &reg, "Blasphemous Act", P0);
    state.get_object_mut(spell).unwrap().subtypes.push("Red".into());

    let behavior = reg.get(state.get_object(spell).unwrap().card_id).unwrap();
    behavior.on_resolve(&mut state, spell, &[], &reg);

    assert_eq!(
        state.get_object(protected).unwrap().damage_marked, 0,
        "CR 702.16e: creature with protection from red takes 0 damage from Blasphemous Act"
    );
    assert_eq!(
        state.get_object(unprotected).unwrap().damage_marked, 13,
        "unprotected creature takes full 13 damage from Blasphemous Act"
    );
}

// CR 614.1a: Unbreathing Horde's replacement effect prevents damage by removing a +1/+1 counter.
// Blasphemous Act writes damage_marked inline, bypassing PreventDamageRemoveCounter.
#[test]
fn blasphemous_act_damage_prevention_replacement_effect() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let horde = named_creature(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    let vanilla = ready_creature(&mut state, P1, 3, 15);

    let spell = spell_on_stack(&mut state, &reg, "Blasphemous Act", P0);

    let behavior = reg.get(state.get_object(spell).unwrap().card_id).unwrap();
    behavior.on_resolve(&mut state, spell, &[], &reg);

    assert_eq!(
        state.get_object(horde).unwrap().damage_marked, 0,
        "CR 614.1a: Unbreathing Horde damage should be prevented by removing a +1/+1 counter"
    );
    assert_eq!(
        counters_of(&state, horde, CounterType::PlusOnePlusOne), 2,
        "Unbreathing Horde should lose one +1/+1 counter to prevent damage"
    );
    assert_eq!(
        state.get_object(vanilla).unwrap().damage_marked, 13,
        "vanilla creature takes full 13 damage from Blasphemous Act"
    );
}

// CR 702.16e: Harvest Pyre deals X damage inline, bypassing protection from source.
#[test]
fn harvest_pyre_inline_damage_bypasses_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target = ready_creature(&mut state, P1, 3, 10);
    state.get_object_mut(target).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Red".into(),
            scope: EffectScope::OnSelf,
        },
    ]);

    let spell = spell_on_stack(&mut state, &reg, "Harvest Pyre", P0);
    {
        let obj = state.get_object_mut(spell).unwrap();
        obj.subtypes.push("Red".into());
        obj.card_state.insert("exile_count".into(), ObjectId(3));
    }

    let behavior = reg.get(state.get_object(spell).unwrap().card_id).unwrap();
    behavior.on_resolve(&mut state, spell, &[Target::Object(target)], &reg);

    assert_eq!(
        state.get_object(target).unwrap().damage_marked, 0,
        "CR 702.16e: creature with protection from red takes 0 damage from Harvest Pyre"
    );
}

// CR 614.1a: Harvest Pyre deals X damage inline, bypassing PreventDamageRemoveCounter.
#[test]
fn harvest_pyre_inline_damage_bypasses_prevention() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let horde = named_creature(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    let spell = spell_on_stack(&mut state, &reg, "Harvest Pyre", P0);
    state.get_object_mut(spell).unwrap().card_state.insert("exile_count".into(), ObjectId(3));

    let behavior = reg.get(state.get_object(spell).unwrap().card_id).unwrap();
    behavior.on_resolve(&mut state, spell, &[Target::Object(horde)], &reg);

    assert_eq!(
        state.get_object(horde).unwrap().damage_marked, 0,
        "CR 614.1a: Unbreathing Horde damage should be prevented by removing a counter"
    );
}

// CR 702.16e: Stensia Bloodhall directly decrements planeswalker loyalty,
// bypassing has_protection_from check.
#[test]
fn bloodhall_damage_respects_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bloodhall = named_creature(&mut state, &reg, "Stensia Bloodhall", P0);
    state.get_object_mut(bloodhall).unwrap().subtypes.push("Bloodhall".into());

    let pw = make_planeswalker(&mut state, P1, "Test Planeswalker", 4);
    state.get_object_mut(pw).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Bloodhall".into(),
            scope: EffectScope::OnSelf,
        },
    ]);

    let behavior = reg.get(state.get_object(bloodhall).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, bloodhall, 1, &[Target::Object(pw)], &reg);

    assert_eq!(
        counters_of(&state, pw, CounterType::Loyalty), 4,
        "CR 702.16e: protection prevents Stensia Bloodhall damage, loyalty should be unchanged"
    );
}

// CR 614.1a: Stensia Bloodhall directly decrements loyalty, bypassing
// PreventDamageRemoveCounter replacement effect.
#[test]
fn bloodhall_damage_respects_prevention() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bloodhall = named_creature(&mut state, &reg, "Stensia Bloodhall", P0);

    let pw = make_planeswalker(&mut state, P1, "Test Planeswalker", 4);
    state.get_object_mut(pw).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::PreventDamageRemoveCounter { scope: EffectScope::OnSelf },
    ]);
    state.add_counters(pw, CounterType::PlusOnePlusOne, 2);

    let behavior = reg.get(state.get_object(bloodhall).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, bloodhall, 1, &[Target::Object(pw)], &reg);

    assert_eq!(
        counters_of(&state, pw, CounterType::Loyalty), 4,
        "CR 614.1a: damage prevented by counter removal, loyalty should be unchanged"
    );
    assert_eq!(
        counters_of(&state, pw, CounterType::PlusOnePlusOne), 1,
        "PreventDamageRemoveCounter should consume one +1/+1 counter"
    );
}

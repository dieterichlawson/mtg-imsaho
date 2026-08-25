//! Regression tests for card- and condition-level code that read the raw
//! object-level characteristic vectors instead of the `GameState` accessors.
//!
//! `obj.card_types`, `obj.subtypes` and `obj.colors` are empty for every
//! non-token permanent — the printed characteristics live on the card's active
//! face in the registry, and those vectors carry only what an effect granted
//! at runtime. Reading them directly produces two opposite failures:
//!
//! - treating an empty vector as "has no types" silently excludes every real
//!   permanent (Garruk's -3, Curse of the Pierced Heart's planeswalker scan);
//! - treating a *non-empty* vector as the whole truth makes a creature stop
//!   being what it was printed as the moment anything is added to it — a Human
//!   that Olivia Voldaren turned into a Vampire stopped counting as a Human.
//!
//! The accessors (`has_card_type`, `is_creature`, `has_subtype`) union the
//! object's runtime grants with the active face, which is right in both
//! directions.

mod common;

use common::*;
use mtg_engine::cards::AttackInfo;
use mtg_engine::types::*;
/// Garruk Relentless' -3 ("Creatures you control get +X/+X and gain trample")
/// filtered on `o.card_types.contains(&Creature)`, which is false for every
/// non-token creature — so the buff hit nothing. (Ticket garruk_relentless-01.)
#[test]
fn garruk_ultimate_buffs_non_token_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bear = named_creature(&mut state, &reg, "Walking Corpse", P0);
    assert!(state.get_object(bear).unwrap().card_types.is_empty(),
        "test precondition: non-token permanents have empty object-level card_types");

    // The -3 lives on the back face, Garruk, the Veil-Cursed. X is the number
    // of creature cards in the graveyard, so put one there for a visible buff.
    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 3);
    mtg_engine::cards::helpers::apply_transform(&mut state, garruk, &reg);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    let before = state.effective_power(bear, &reg).unwrap();
    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    let minus_three = behavior.loyalty_abilities(&state, garruk).into_iter()
        .find(|a| a.loyalty_change == -3)
        .expect("Garruk, the Veil-Cursed should have a -3 ability");
    behavior.on_loyalty_ability(&mut state, garruk, minus_three.ability_index, &[], &reg);

    assert!(state.effective_power(bear, &reg).unwrap() > before,
        "a non-token creature must receive Garruk's +X/+X (was {before}, now {:?})",
        state.effective_power(bear, &reg));
    assert!(state.has_keyword(bear, Keyword::Trample, &reg),
        "a non-token creature must receive trample from Garruk's ultimate");
}

/// Curse of the Pierced Heart's "1 damage to enchanted player or a
/// planeswalker they control" scanned `o.card_types` for Planeswalker, so the
/// planeswalker half was dead code. (Ticket curse_of_the_pierced_heart-01.)
#[test]
fn curse_of_the_pierced_heart_sees_non_token_planeswalkers() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P1);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    assert!(state.get_object(liliana).unwrap().card_types.is_empty(),
        "test precondition: non-token permanents have empty object-level card_types");

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P0, P1);
    let life_before = state.get_player(P1).life;

    let behavior = reg.get(state.get_object(curse).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, curse, &[], &reg);

    // Either the damage was redirected to the planeswalker, or the controller
    // was asked to choose — both mean the planeswalker was seen. What must not
    // happen is silently hitting the player as if no planeswalker existed.
    let asked = state.awaiting_action.is_some();
    let hit_planeswalker = counters_of(&state, liliana, CounterType::Loyalty) < 3;
    assert!(asked || hit_planeswalker,
        "the Curse must see a non-token planeswalker; instead it silently hit \
         the player (life {life_before} -> {})", state.get_player(P1).life);
}

/// Silver-Inlaid Dagger / Butcher's Cleaver check "as long as equipped
/// creature is a Human" through `EffectCondition::AttachedHasSubtype`, which
/// treated a non-empty `obj.subtypes` as authoritative. Olivia Voldaren pushes
/// "Vampire" onto a creature she damages — which made a Human stop being a
/// Human. Subtypes are additive. (Tickets silver_inlaid_dagger-01,
/// butcher_s_cleaver-02.)
#[test]
fn human_equipment_bonus_survives_gaining_another_subtype() {
    let reg = registry();
    for gear_name in ["Silver-Inlaid Dagger", "Butcher's Cleaver"] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
        let gear = named_equipment(&mut state, &reg, gear_name, P0);
        state.get_object_mut(gear).unwrap().attached_to = Some(pilgrim);

        let human_power = state.effective_power(pilgrim, &reg);
        let human_lifelink = state.has_keyword(pilgrim, Keyword::Lifelink, &reg);

        // Olivia's ability: "That creature becomes a Vampire in addition to
        // its other types" — an addition, not a replacement.
        state.get_object_mut(pilgrim).unwrap().subtypes.push("Vampire".into());

        assert_eq!(state.effective_power(pilgrim, &reg), human_power,
            "{gear_name}: a Human that also became a Vampire is still a Human");
        assert_eq!(state.has_keyword(pilgrim, Keyword::Lifelink, &reg), human_lifelink,
            "{gear_name}: the Human-conditional keyword must survive gaining a subtype");
    }
}

/// Hamlet Captain buffs "other Human creatures you control". Its subtype check
/// fell back to `registry.card_data`, which always returns FRONT-face data, so
/// a transformed werewolf still looked Human. CR 712.8d: a permanent has only
/// its current face's characteristics. (Ticket hamlet_captain-01.)
#[test]
fn hamlet_captain_does_not_buff_transformed_werewolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let captain = named_creature(&mut state, &reg, "Hamlet Captain", P0);
    let villager = named_creature(&mut state, &reg, "Villagers of Estwald", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, villager, &reg);
    assert!(!state.has_subtype(villager, "Human", &reg),
        "test precondition: Howlpack of Estwald is a Werewolf, not a Human");

    let before = state.effective_power(villager, &reg);
    let behavior = reg.get(state.get_object(captain).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, captain, AttackInfo::new(captain, P1), &[], &reg);

    assert_eq!(state.effective_power(villager, &reg), before,
        "a transformed non-Human werewolf must not get Hamlet Captain's +1/+1");
}

/// CR 400.7: a permanent that changes zones becomes a new object with no
/// memory of the old one. Olivia's "Vampire" and Grimoire of the Dead's
/// "Zombie" / black are runtime grants on the object, and were surviving the
/// trip to the graveyard and back. (Tickets olivia_voldaren-01,
/// grimoire_of_the_dead-01.)
#[test]
fn runtime_granted_types_and_colors_are_cleared_on_zone_change() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    {
        let obj = state.get_object_mut(pilgrim).unwrap();
        obj.subtypes.push("Vampire".into());
        obj.subtypes.push("Zombie".into());
        obj.colors.push(Color::Black);
    }
    assert!(state.has_subtype(pilgrim, "Vampire", &reg), "test precondition");

    state.move_object(pilgrim, Zone::Graveyard, &reg);

    let obj = state.get_object(pilgrim).unwrap();
    assert!(obj.subtypes.is_empty(),
        "runtime-granted subtypes must not follow the card out of the battlefield, \
         got {:?}", obj.subtypes);
    assert!(obj.colors.is_empty(),
        "a runtime-granted color must not follow the card out of the battlefield, \
         got {:?}", obj.colors);
    assert!(state.has_subtype(pilgrim, "Human", &reg),
        "the card's printed subtypes come from the registry and are unaffected");
}

/// A token's object-level fields *are* its printed characteristics, so the
/// CR 400.7 cleanup must not strip them.
#[test]
fn tokens_keep_their_own_characteristics_on_zone_change() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let token = *state.create_token_with_subtypes(
        "Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature],
        vec![Keyword::Flying], vec!["Spirit".into()], &reg)
        .first().expect("token should be created");
    assert!(!state.get_object(token).unwrap().subtypes.is_empty(), "test precondition");

    state.move_object(token, Zone::Graveyard, &reg);

    assert_eq!(state.get_object(token).unwrap().subtypes, vec!["Spirit".to_string()],
        "a token's object-level subtypes are its printed ones and must survive");
}

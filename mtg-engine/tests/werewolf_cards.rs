//! Tests for Innistrad werewolf double-faced cards.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::events::GameEvent;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Helper: set up a werewolf on the battlefield and trigger its upkeep transform.
/// Returns the state after triggering the upkeep.
fn trigger_upkeep(state: &mut mtg_engine::state::GameState, registry: &CardRegistry) {
    state.events.push(GameEvent::StepStarted { step: Step::Upkeep });
    triggers::process_triggers(state, registry);
}

// ── Reckless Waif ─────────────────────────────────────────────────

#[test]
fn reckless_waif_transforms_when_no_spells_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    // num_spells_cast_last_turn is empty (no spells cast), is_first_turn is false
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);

    assert_eq!(state.effective_power(waif, &reg).unwrap(), 1);
    assert_eq!(state.effective_toughness(waif, &reg).unwrap(), 1);

    trigger_upkeep(&mut state, &reg);

    // Should be transformed to Merciless Predator 3/2
    let obj = state.get_object(waif).unwrap();
    assert!(obj.is_transformed, "Reckless Waif should transform when no spells cast last turn");
    assert_eq!(obj.name, "Merciless Predator");
    assert_eq!(state.effective_power(waif, &reg).unwrap(), 3);
    assert_eq!(state.effective_toughness(waif, &reg).unwrap(), 2);
}

#[test]
fn reckless_waif_stays_human_on_first_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.is_first_turn = true;
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);

    trigger_upkeep(&mut state, &reg);

    let obj = state.get_object(waif).unwrap();
    assert!(!obj.is_transformed, "Should not transform on first turn");
}

#[test]
fn reckless_waif_stays_human_when_spells_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.num_spells_cast_last_turn.insert(P0, 1);
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);

    trigger_upkeep(&mut state, &reg);

    let obj = state.get_object(waif).unwrap();
    assert!(!obj.is_transformed, "Should not transform when spells were cast");
}

#[test]
fn reckless_waif_transforms_back_when_two_spells_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);

    // Manually transform to werewolf side
    state.get_object_mut(waif).unwrap().is_transformed = true;
    state.get_object_mut(waif).unwrap().name = "Merciless Predator".into();

    // Set up: a player cast 2 spells last turn
    state.num_spells_cast_last_turn.insert(P0, 2);

    trigger_upkeep(&mut state, &reg);

    let obj = state.get_object(waif).unwrap();
    assert!(!obj.is_transformed, "Should transform back when 2+ spells cast");
    assert_eq!(obj.name, "Reckless Waif");
    assert_eq!(state.effective_power(waif, &reg).unwrap(), 1);
}

// ── Gatstaf Shepherd ──────────────────────────────────────────────

#[test]
fn gatstaf_shepherd_transforms_and_gains_intimidate() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let shepherd = named_creature(&mut state, &reg, "Gatstaf Shepherd", P0);

    trigger_upkeep(&mut state, &reg);

    let obj = state.get_object(shepherd).unwrap();
    assert!(obj.is_transformed);
    assert_eq!(obj.name, "Gatstaf Howler");
    assert_eq!(state.effective_power(shepherd, &reg).unwrap(), 3);
    assert_eq!(state.effective_toughness(shepherd, &reg).unwrap(), 3);
    assert!(state.has_keyword(shepherd, Keyword::Intimidate, &reg),
        "Gatstaf Howler should have Intimidate");
}

#[test]
fn gatstaf_shepherd_loses_intimidate_on_transform_back() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let shepherd = named_creature(&mut state, &reg, "Gatstaf Shepherd", P0);

    // Transform to werewolf
    state.get_object_mut(shepherd).unwrap().is_transformed = true;
    assert!(state.has_keyword(shepherd, Keyword::Intimidate, &reg));

    // Set up transform back
    state.num_spells_cast_last_turn.insert(P1, 2);
    trigger_upkeep(&mut state, &reg);

    assert!(!state.get_object(shepherd).unwrap().is_transformed);
    assert!(!state.has_keyword(shepherd, Keyword::Intimidate, &reg),
        "Gatstaf Shepherd should not have Intimidate");
}

// ── Village Ironsmith ─────────────────────────────────────────────

#[test]
fn village_ironsmith_keeps_first_strike_on_both_faces() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let ironsmith = named_creature(&mut state, &reg, "Village Ironsmith", P0);

    // Front face: first strike
    assert!(state.has_keyword(ironsmith, Keyword::FirstStrike, &reg));

    trigger_upkeep(&mut state, &reg);

    // Back face (Ironfang): still first strike
    assert!(state.get_object(ironsmith).unwrap().is_transformed);
    assert_eq!(state.effective_power(ironsmith, &reg).unwrap(), 3);
    assert_eq!(state.effective_toughness(ironsmith, &reg).unwrap(), 1);
    assert!(state.has_keyword(ironsmith, Keyword::FirstStrike, &reg),
        "Ironfang should have First Strike");
}

// ── Villagers of Estwald ──────────────────────────────────────────

#[test]
fn villagers_of_estwald_transforms_to_large_body() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let villagers = named_creature(&mut state, &reg, "Villagers of Estwald", P0);

    assert_eq!(state.effective_power(villagers, &reg).unwrap(), 2);
    assert_eq!(state.effective_toughness(villagers, &reg).unwrap(), 3);

    trigger_upkeep(&mut state, &reg);

    assert_eq!(state.effective_power(villagers, &reg).unwrap(), 4);
    assert_eq!(state.effective_toughness(villagers, &reg).unwrap(), 6);
    assert_eq!(state.get_object(villagers).unwrap().name, "Howlpack of Estwald");
}

// ── Hanweir Watchkeep ─────────────────────────────────────────────

#[test]
fn hanweir_watchkeep_loses_defender_gains_force_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let watchkeep = named_creature(&mut state, &reg, "Hanweir Watchkeep", P0);

    // Front face: Defender
    assert!(state.has_keyword(watchkeep, Keyword::Defender, &reg));
    assert_eq!(state.effective_power(watchkeep, &reg).unwrap(), 1);

    trigger_upkeep(&mut state, &reg);

    // Back face (Bane of Hanweir): 5/5, no Defender, attacks each combat
    assert!(state.get_object(watchkeep).unwrap().is_transformed);
    assert_eq!(state.effective_power(watchkeep, &reg).unwrap(), 5);
    assert!(!state.has_keyword(watchkeep, Keyword::Defender, &reg),
        "Bane of Hanweir should not have Defender");

    // ForceAttack is a continuous effect on the back face
    assert!(state.has_continuous_effect(watchkeep, &|e| {
        match e {
            ContinuousEffect::ForceAttack { scope } => Some(scope),
            _ => None,
        }
    }, &reg), "Bane of Hanweir should have ForceAttack");
}

// ── Tormented Pariah ──────────────────────────────────────────────

#[test]
fn tormented_pariah_transforms_to_large_werewolf() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let pariah = named_creature(&mut state, &reg, "Tormented Pariah", P0);

    assert_eq!(state.effective_power(pariah, &reg).unwrap(), 3);
    assert_eq!(state.effective_toughness(pariah, &reg).unwrap(), 2);

    trigger_upkeep(&mut state, &reg);

    assert_eq!(state.effective_power(pariah, &reg).unwrap(), 6);
    assert_eq!(state.effective_toughness(pariah, &reg).unwrap(), 4);
    assert_eq!(state.get_object(pariah).unwrap().name, "Rampaging Werewolf");
}

// ── Grizzled Outcasts ─────────────────────────────────────────────

#[test]
fn grizzled_outcasts_transforms_to_7_7() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let outcasts = named_creature(&mut state, &reg, "Grizzled Outcasts", P0);

    assert_eq!(state.effective_power(outcasts, &reg).unwrap(), 4);

    trigger_upkeep(&mut state, &reg);

    assert_eq!(state.effective_power(outcasts, &reg).unwrap(), 7);
    assert_eq!(state.effective_toughness(outcasts, &reg).unwrap(), 7);
    assert_eq!(state.get_object(outcasts).unwrap().name, "Krallenhorde Wantons");
}

// ── Mayor of Avabruck ─────────────────────────────────────────────

#[test]
fn mayor_of_avabruck_buffs_humans_on_front_face() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mayor = named_creature(&mut state, &reg, "Mayor of Avabruck", P0);
    let human = named_creature(&mut state, &reg, "Reckless Waif", P0);

    // Mayor gives other Humans +1/+1
    assert_eq!(state.effective_power(human, &reg).unwrap(), 2, "Reckless Waif should get +1/+1 from Mayor");
    assert_eq!(state.effective_toughness(human, &reg).unwrap(), 2);
    // Mayor doesn't buff itself
    assert_eq!(state.effective_power(mayor, &reg).unwrap(), 1);
}

#[test]
fn mayor_of_avabruck_transforms_and_buffs_werewolves() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let mayor = named_creature(&mut state, &reg, "Mayor of Avabruck", P0);
    let other_wolf = named_creature(&mut state, &reg, "Reckless Waif", P0);

    // Transform both
    trigger_upkeep(&mut state, &reg);

    // Mayor is now Howlpack Alpha (3/3), gives other Werewolves/Wolves +1/+1
    assert!(state.get_object(mayor).unwrap().is_transformed);
    assert_eq!(state.effective_power(mayor, &reg).unwrap(), 3);

    // Other werewolf should get the buff: Merciless Predator is 3/2 + 1/1 = 4/3
    assert!(state.get_object(other_wolf).unwrap().is_transformed);
    assert_eq!(state.effective_power(other_wolf, &reg).unwrap(), 4,
        "Merciless Predator should get +1/+1 from Howlpack Alpha");
    assert_eq!(state.effective_toughness(other_wolf, &reg).unwrap(), 3);
}

#[test]
fn howlpack_alpha_creates_wolf_token_on_end_step() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    let mayor = named_creature(&mut state, &reg, "Mayor of Avabruck", P0);
    // Transform to Howlpack Alpha
    state.get_object_mut(mayor).unwrap().is_transformed = true;
    state.get_object_mut(mayor).unwrap().name = "Howlpack Alpha".into();

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    triggers::process_triggers(&mut state, &reg);

    // Should have created a 2/2 Wolf token
    let wolves: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.is_token && o.name == "Wolf")
        .collect();
    assert_eq!(wolves.len(), 1, "Howlpack Alpha should create one Wolf token");
    assert_eq!(wolves[0].power, Some(2));
    assert_eq!(wolves[0].toughness, Some(2));
}

#[test]
fn howlpack_alpha_does_not_create_token_on_front_face() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    let _mayor = named_creature(&mut state, &reg, "Mayor of Avabruck", P0);
    // Front face (not transformed)

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    triggers::process_triggers(&mut state, &reg);

    let wolves: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.is_token && o.name == "Wolf")
        .collect();
    assert_eq!(wolves.len(), 0, "Front face Mayor should not create Wolf tokens");
}

#[test]
fn howlpack_alpha_does_not_create_token_on_opponents_end_step() {
    let reg = registry();
    // Active player is P1 (opponent), but Mayor belongs to P0
    let mut state = game_at_step(Step::EndStep, P1);
    let mayor = named_creature(&mut state, &reg, "Mayor of Avabruck", P0);
    // Transform to Howlpack Alpha
    state.get_object_mut(mayor).unwrap().is_transformed = true;
    state.get_object_mut(mayor).unwrap().name = "Howlpack Alpha".into();

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    triggers::process_triggers(&mut state, &reg);

    // Should NOT create a Wolf token on opponent's end step
    let wolves: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.is_token && o.name == "Wolf")
        .collect();
    assert_eq!(wolves.len(), 0,
        "Howlpack Alpha should not create Wolf tokens on opponent's end step");
}

#[test]
fn mayor_of_avabruck_does_not_transform_on_first_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.is_first_turn = true;
    let mayor = named_creature(&mut state, &reg, "Mayor of Avabruck", P0);

    trigger_upkeep(&mut state, &reg);

    let obj = state.get_object(mayor).unwrap();
    assert!(!obj.is_transformed,
        "Mayor of Avabruck should not transform on first turn");
}

#[test]
fn howlpack_alpha_werewolf_wolf_creature_gets_only_plus_one() {
    // Ruling [2025-01-24]: A creature that is both a Werewolf and a Wolf will
    // only get +1/+1 from Howlpack Alpha's first ability.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mayor = named_creature(&mut state, &reg, "Mayor of Avabruck", P0);
    // Transform to Howlpack Alpha
    state.get_object_mut(mayor).unwrap().is_transformed = true;
    state.get_object_mut(mayor).unwrap().name = "Howlpack Alpha".into();

    // Create a token that is both a Werewolf and a Wolf (2/2 base)
    let dual_id = state.create_token_with_subtypes(
        "Test Werewolf Wolf",
        P0,
        2, 2,
        vec![Color::Green],
        vec![CardType::Creature],
        vec![],
        vec!["Werewolf".into(), "Wolf".into()],
        &reg,
    );

    // The creature should get exactly +1/+1 (not +2/+2) from Howlpack Alpha
    assert_eq!(state.effective_power(dual_id, &reg).unwrap(), 3,
        "Werewolf+Wolf creature should get only +1/+1 from Howlpack Alpha (not +2/+2)");
    assert_eq!(state.effective_toughness(dual_id, &reg).unwrap(), 3,
        "Werewolf+Wolf creature should get only +1/+1 from Howlpack Alpha (not +2/+2)");
}

// ── Daybreak Ranger ───────────────────────────────────────────────

#[test]
fn daybreak_ranger_transforms_to_nightfall_predator() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let ranger = named_creature(&mut state, &reg, "Daybreak Ranger", P0);

    assert_eq!(state.effective_power(ranger, &reg).unwrap(), 2);

    trigger_upkeep(&mut state, &reg);

    assert!(state.get_object(ranger).unwrap().is_transformed);
    assert_eq!(state.effective_power(ranger, &reg).unwrap(), 4);
    assert_eq!(state.effective_toughness(ranger, &reg).unwrap(), 4);
    assert_eq!(state.get_object(ranger).unwrap().name, "Nightfall Predator");
}

#[test]
fn daybreak_ranger_has_activated_ability_on_front_face() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ranger = named_creature(&mut state, &reg, "Daybreak Ranger", P0);

    let abilities = reg.get(state.get_object(ranger).unwrap().card_id).unwrap()
        .activated_abilities(&state, ranger, &reg);
    assert_eq!(abilities.len(), 1);
    assert!(abilities[0].description.contains("flying"));
}

#[test]
fn nightfall_predator_has_fight_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ranger = named_creature(&mut state, &reg, "Daybreak Ranger", P0);
    state.get_object_mut(ranger).unwrap().is_transformed = true;

    let abilities = reg.get(state.get_object(ranger).unwrap().card_id).unwrap()
        .activated_abilities(&state, ranger, &reg);
    assert_eq!(abilities.len(), 1);
    assert!(abilities[0].description.contains("Fight"));
}

#[test]
fn nightfall_predator_can_fight_own_creature() {
    // Per oracle: "{R}, {T}: This creature fights target creature." — no controller restriction.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ranger = named_creature(&mut state, &reg, "Daybreak Ranger", P0);
    state.get_object_mut(ranger).unwrap().is_transformed = true;
    state.get_object_mut(ranger).unwrap().name = "Nightfall Predator".into();

    // Own creature to fight.
    let own_creature = ready_creature(&mut state, P0, 2, 2);

    // Give mana for the {R} cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: ranger,
            ability_index: 0,
            targets: vec![Target::Object(own_creature)],
            tap_plan: vec![],
            sacrifice: None,
        },
        &reg,
    );

    // Both creatures should have dealt damage to each other.
    // Nightfall Predator is 4/4, own creature is 2/2.
    assert_eq!(new_state.get_object(own_creature).unwrap().damage_marked, 4,
        "Own creature should take 4 damage from Nightfall Predator");
    assert_eq!(new_state.get_object(ranger).unwrap().damage_marked, 2,
        "Nightfall Predator should take 2 damage from own creature");
}

// ── Instigator Gang ───────────────────────────────────────────────

#[test]
fn instigator_gang_transforms_and_gains_trample() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);

    assert!(!state.has_keyword(gang, Keyword::Trample, &reg));

    trigger_upkeep(&mut state, &reg);

    assert!(state.get_object(gang).unwrap().is_transformed);
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 5);
    assert!(state.has_keyword(gang, Keyword::Trample, &reg),
        "Wildblood Pack should have Trample");
}

#[test]
fn instigator_gang_buffs_itself_when_attacking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);

    // Base power is 2.
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 2);

    // Declare Instigator Gang as attacker — it should buff itself +1/+0.
    state.events.push(GameEvent::AttackersDeclared {
        attackers: vec![(gang, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // Should be 2 base + 1 from own buff = 3.
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 3,
        "Instigator Gang should buff itself when attacking (+1/+0)");
}

#[test]
fn instigator_gang_buffs_other_attackers_you_control() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);
    let ally = ready_creature(&mut state, P0, 3, 3);

    // Declare both as attackers.
    state.events.push(GameEvent::AttackersDeclared {
        attackers: vec![(gang, P1), (ally, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // Gang: 2 + 1 = 3 (buffs itself too).
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 3,
        "Instigator Gang should be 3 power when attacking");
    // Ally: 3 + 1 = 4 (buffed by Instigator Gang).
    assert_eq!(state.effective_power(ally, &reg).unwrap(), 4,
        "Ally should get +1/+0 from Instigator Gang");
}

#[test]
fn instigator_gang_does_not_buff_opponent_attackers() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P1);
    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);
    let enemy = ready_creature(&mut state, P1, 2, 2);

    // Opponent's creature attacks.
    state.events.push(GameEvent::AttackersDeclared {
        attackers: vec![(enemy, P0)],
    });
    triggers::process_triggers(&mut state, &reg);

    // Enemy should NOT be buffed (different controller).
    assert_eq!(state.effective_power(enemy, &reg).unwrap(), 2,
        "Opponent's creature should not get Instigator Gang's buff");
}

#[test]
fn wildblood_pack_buffs_itself_plus_3() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);

    // Transform to Wildblood Pack.
    state.get_object_mut(gang).unwrap().is_transformed = true;
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 5);

    // Declare Wildblood Pack as attacker — should buff itself +3/+0.
    state.events.push(GameEvent::AttackersDeclared {
        attackers: vec![(gang, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // Should be 5 base + 3 from own buff = 8.
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 8,
        "Wildblood Pack should buff itself +3/+0 when attacking");
}

// ── Ulvenwald Mystics ─────────────────────────────────────────────

#[test]
fn ulvenwald_mystics_transforms_and_gains_regenerate() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let mystics = named_creature(&mut state, &reg, "Ulvenwald Mystics", P0);

    // Front face: no activated abilities
    let front_abilities = reg.get(state.get_object(mystics).unwrap().card_id).unwrap()
        .activated_abilities(&state, mystics, &reg);
    assert_eq!(front_abilities.len(), 0, "Front face should have no activated abilities");

    trigger_upkeep(&mut state, &reg);

    // Back face: {G}: Regenerate
    assert!(state.get_object(mystics).unwrap().is_transformed);
    assert_eq!(state.effective_power(mystics, &reg).unwrap(), 5);
    let back_abilities = reg.get(state.get_object(mystics).unwrap().card_id).unwrap()
        .activated_abilities(&state, mystics, &reg);
    assert_eq!(back_abilities.len(), 1, "Ulvenwald Primordials should have regenerate ability");
    assert!(back_abilities[0].description.contains("Regenerate"));
}

// ── Kruin Outlaw ──────────────────────────────────────────────────

#[test]
fn kruin_outlaw_transforms_gains_double_strike_and_menace() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let outlaw = named_creature(&mut state, &reg, "Kruin Outlaw", P0);

    // Front: first strike, no double strike or menace
    assert!(state.has_keyword(outlaw, Keyword::FirstStrike, &reg));
    assert!(!state.has_keyword(outlaw, Keyword::DoubleStrike, &reg));
    assert!(!state.has_keyword(outlaw, Keyword::Menace, &reg));

    trigger_upkeep(&mut state, &reg);

    // Back: double strike, "can't be blocked except by two or more creatures" for
    // all Werewolves (implemented as MinimumBlockers, not as Keyword::Menace).
    assert!(state.get_object(outlaw).unwrap().is_transformed);
    assert_eq!(state.effective_power(outlaw, &reg).unwrap(), 3);
    assert!(state.has_keyword(outlaw, Keyword::DoubleStrike, &reg),
        "Terror of Kruin Pass should have Double Strike");
    // The blocking restriction is enforced via MinimumBlockers continuous effect,
    // not as a menace keyword. See kruin_outlaw.rs tests for blocking validation.
}

// ── Subtype tests ─────────────────────────────────────────────────

#[test]
fn transformed_werewolf_has_werewolf_subtype_not_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);

    // Front face: Human subtype
    assert!(state.matches_filter(waif,
        &CreatureFilter::HasSubtype("Human".into()), P0, &reg),
        "Front face should have Human subtype");

    // Transform
    state.get_object_mut(waif).unwrap().is_transformed = true;

    // Back face: Werewolf subtype, not Human
    assert!(state.matches_filter(waif,
        &CreatureFilter::HasSubtype("Werewolf".into()), P0, &reg),
        "Back face should have Werewolf subtype");
    assert!(!state.matches_filter(waif,
        &CreatureFilter::HasSubtype("Human".into()), P0, &reg),
        "Back face should not have Human subtype");
}

// ── All werewolves transform together ─────────────────────────────

#[test]
fn multiple_werewolves_transform_on_same_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);
    let shepherd = named_creature(&mut state, &reg, "Gatstaf Shepherd", P0);
    let outcasts = named_creature(&mut state, &reg, "Grizzled Outcasts", P0);

    // No spells cast last turn, all should transform
    trigger_upkeep(&mut state, &reg);

    assert!(state.get_object(waif).unwrap().is_transformed, "Reckless Waif should transform");
    assert!(state.get_object(shepherd).unwrap().is_transformed, "Gatstaf Shepherd should transform");
    assert!(state.get_object(outcasts).unwrap().is_transformed, "Grizzled Outcasts should transform");
}

#[test]
fn multiple_werewolves_transform_back_together() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);
    let shepherd = named_creature(&mut state, &reg, "Gatstaf Shepherd", P0);

    // Manually transform to werewolf side
    state.get_object_mut(waif).unwrap().is_transformed = true;
    state.get_object_mut(shepherd).unwrap().is_transformed = true;

    // A player cast 2 spells last turn
    state.num_spells_cast_last_turn.insert(P1, 2);

    trigger_upkeep(&mut state, &reg);

    assert!(!state.get_object(waif).unwrap().is_transformed, "Should transform back");
    assert!(!state.get_object(shepherd).unwrap().is_transformed, "Should transform back");
}

// ── Werewolf does not transform when shouldn't ────────────────────

#[test]
fn werewolf_side_stays_if_one_spell_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);
    state.get_object_mut(waif).unwrap().is_transformed = true;

    // Only 1 spell cast last turn: not enough to transform back
    state.num_spells_cast_last_turn.insert(P0, 1);

    trigger_upkeep(&mut state, &reg);

    assert!(state.get_object(waif).unwrap().is_transformed,
        "Werewolf should stay transformed with only 1 spell cast");
}

#[test]
fn human_side_stays_if_any_spell_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_creature(&mut state, &reg, "Reckless Waif", P0);

    // Opponent cast 1 spell last turn
    state.num_spells_cast_last_turn.insert(P1, 1);

    trigger_upkeep(&mut state, &reg);

    assert!(!state.get_object(waif).unwrap().is_transformed,
        "Human should stay on front face when any spell was cast last turn");
}

//! Tests for Innistrad Tier 3 cards: token-generating spells, creatures with
//! dies/ETB/death-watch triggers, anthem enchantments, and token mechanics.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ══════════════════════════════════════════════════════════════════
// Token-generating spells
// ══════════════════════════════════════════════════════════════════

/// Midnight Haunting creates two 1/1 Spirit tokens with flying.
#[test]
fn midnight_haunting_creates_two_spirits() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Midnight Haunting").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 3);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Spell goes to graveyard.
    assert_eq!(state.get_object(card).unwrap().zone, Zone::Graveyard);

    // Two Spirit tokens on the battlefield.
    let tokens: Vec<_> = state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.is_token && o.name == "Spirit")
        .collect();
    assert_eq!(tokens.len(), 2, "Should have two Spirit tokens");

    for t in &tokens {
        assert_eq!(t.power, Some(1));
        assert_eq!(t.toughness, Some(1));
        assert!(t.keywords.contains(&Keyword::Flying), "Spirit tokens should have flying");
    }
}

/// Moan of the Unhallowed creates two 2/2 Zombie tokens.
#[test]
fn moan_creates_two_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Moan of the Unhallowed").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 4);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(card).unwrap().zone, Zone::Graveyard);

    let tokens: Vec<_> = state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.is_token && o.name == "Zombie")
        .collect();
    assert_eq!(tokens.len(), 2, "Should have two Zombie tokens");

    for t in &tokens {
        assert_eq!(t.power, Some(2));
        assert_eq!(t.toughness, Some(2));
    }
}

// ══════════════════════════════════════════════════════════════════
// Dies triggers — token creators
// ══════════════════════════════════════════════════════════════════

/// Doomed Traveler creates a 1/1 Spirit token with flying when it dies.
#[test]
fn doomed_traveler_creates_spirit_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let dt_id = reg.get_id_by_name("Doomed Traveler").unwrap();
    let dt = state.create_object(dt_id, P0, Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(dt).unwrap().name = "Doomed Traveler".into();
    state.get_object_mut(dt).unwrap().summoning_sick = false;

    // Kill it with lethal damage.
    state.get_object_mut(dt).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(dt).unwrap().zone, Zone::Graveyard,
        "Doomed Traveler should be dead");

    // Fire death triggers.
    triggers::process_triggers(&mut state, &reg);

    // Should have a Spirit token.
    let spirits: Vec<_> = state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.is_token && o.name == "Spirit")
        .collect();
    assert_eq!(spirits.len(), 1, "Should create one Spirit token");
    assert_eq!(spirits[0].power, Some(1));
    assert_eq!(spirits[0].toughness, Some(1));
    assert!(spirits[0].keywords.contains(&Keyword::Flying));
}

/// Mausoleum Guard creates two 1/1 Spirit tokens with flying when it dies.
#[test]
fn mausoleum_guard_creates_two_spirits_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mg_id = reg.get_id_by_name("Mausoleum Guard").unwrap();
    let mg = state.create_object(mg_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(mg).unwrap().name = "Mausoleum Guard".into();
    state.get_object_mut(mg).unwrap().summoning_sick = false;

    // Kill it with lethal damage.
    state.get_object_mut(mg).unwrap().damage_marked = 2;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(mg).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    let spirits: Vec<_> = state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.is_token && o.name == "Spirit")
        .collect();
    assert_eq!(spirits.len(), 2, "Should create two Spirit tokens");

    for s in &spirits {
        assert_eq!(s.power, Some(1));
        assert_eq!(s.toughness, Some(1));
        assert!(s.keywords.contains(&Keyword::Flying));
    }
}

// ══════════════════════════════════════════════════════════════════
// ETB triggers
// ══════════════════════════════════════════════════════════════════

/// Village Bell-Ringer has flash and untaps all creatures you control on ETB.
#[test]
fn village_bell_ringer_untaps_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two tapped creatures on P0's side.
    let c1 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(c1).unwrap().tapped = true;
    let c2 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(c2).unwrap().tapped = true;

    // An opponent's tapped creature (should NOT be untapped).
    let opp = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(opp).unwrap().tapped = true;

    // Cast Village Bell-Ringer (flash creature, castable anytime).
    let vbr_id = reg.get_id_by_name("Village Bell-Ringer").unwrap();
    let vbr = state.create_object(vbr_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 3);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: vbr, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Process ETB triggers.
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(vbr).unwrap().zone, Zone::Battlefield);
    assert!(!state.get_object(c1).unwrap().tapped,
        "P0's creature should be untapped by Village Bell-Ringer");
    assert!(!state.get_object(c2).unwrap().tapped,
        "P0's second creature should also be untapped");
    assert!(state.get_object(opp).unwrap().tapped,
        "Opponent's creature should NOT be untapped");
}

/// Slayer of the Wicked destroys a Vampire, Werewolf, or Zombie on ETB.
#[test]
fn slayer_of_the_wicked_destroys_zombie() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a Walking Corpse (Zombie) on the battlefield.
    let wc_id = reg.get_id_by_name("Walking Corpse").unwrap();
    let wc = state.create_object(wc_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(wc).unwrap().name = "Walking Corpse".into();
    state.get_object_mut(wc).unwrap().summoning_sick = false;

    // Cast Slayer of the Wicked.
    let slayer_id = reg.get_id_by_name("Slayer of the Wicked").unwrap();
    let slayer = state.create_object(slayer_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 4);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: slayer, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Process ETB triggers.
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(slayer).unwrap().zone, Zone::Battlefield,
        "Slayer should be on the battlefield");
    assert_eq!(state.get_object(wc).unwrap().zone, Zone::Graveyard,
        "Walking Corpse (Zombie) should be destroyed by Slayer of the Wicked");
}

/// Fiend Hunter exiles an opponent's creature on ETB.
#[test]
fn fiend_hunter_exiles_on_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a creature on the battlefield.
    let victim = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(victim).unwrap().name = "Big Creature".into();

    // Cast Fiend Hunter.
    let fh_id = reg.get_id_by_name("Fiend Hunter").unwrap();
    let fh = state.create_object(fh_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 3);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: fh, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Process ETB triggers.
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(fh).unwrap().zone, Zone::Battlefield,
        "Fiend Hunter should be on the battlefield");
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Exile,
        "Opponent's creature should be exiled by Fiend Hunter");
}

// ══════════════════════════════════════════════════════════════════
// Dies triggers — damage and life drain
// ══════════════════════════════════════════════════════════════════

/// Pitchburn Devils deals 3 damage to any target when it dies.
#[test]
fn pitchburn_devils_deals_3_on_death() {
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pd_id = reg.get_id_by_name("Pitchburn Devils").unwrap();
    let pd = state.create_object(pd_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(pd).unwrap().name = "Pitchburn Devils".into();
    state.get_object_mut(pd).unwrap().summoning_sick = false;

    // Kill it with lethal damage.
    state.get_object_mut(pd).unwrap().damage_marked = 3;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(pd).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    // Should be awaiting a target choice (both players are valid targets).
    assert!(state.awaiting_action.is_some(), "Should be awaiting damage target choice");

    // Choose opponent as target.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Player(P1))) },
        &reg,
    );

    assert_eq!(state.get_player(P1).life, 17,
        "Opponent should lose 3 life from Pitchburn Devils dying");
}

/// Falkenrath Noble drains 1 life whenever any creature dies.
#[test]
fn falkenrath_noble_drains_on_any_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Falkenrath Noble on P0's side.
    let fn_id = reg.get_id_by_name("Falkenrath Noble").unwrap();
    let noble = state.create_object(fn_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(noble).unwrap().name = "Falkenrath Noble".into();
    state.get_object_mut(noble).unwrap().summoning_sick = false;

    // A creature on P1's side to kill.
    let victim = ready_creature(&mut state, P1, 1, 1);

    // Kill the victim.
    state.get_object_mut(victim).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 19,
        "P1 should lose 1 life from Falkenrath Noble's trigger");
    assert_eq!(state.get_player(P0).life, 21,
        "P0 should gain 1 life from Falkenrath Noble's trigger");
}

/// Rage Thrower deals 2 damage to the opponent when another creature dies.
#[test]
fn rage_thrower_deals_2_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Rage Thrower on P0's side.
    let rt_id = reg.get_id_by_name("Rage Thrower").unwrap();
    let rt = state.create_object(rt_id, P0, Zone::Battlefield, Some(4), Some(2));
    state.get_object_mut(rt).unwrap().name = "Rage Thrower".into();
    state.get_object_mut(rt).unwrap().summoning_sick = false;

    // A creature on P1's side to kill.
    let victim = ready_creature(&mut state, P1, 1, 1);

    // Kill the victim.
    state.get_object_mut(victim).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 18,
        "P1 should lose 2 life from Rage Thrower's trigger");
}

// ══════════════════════════════════════════════════════════════════
// Dies triggers — +1/+1 counters
// ══════════════════════════════════════════════════════════════════

/// Unruly Mob gains a +1/+1 counter when another creature you control dies.
#[test]
fn unruly_mob_gains_counter_when_ally_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Unruly Mob on P0's side.
    let um_id = reg.get_id_by_name("Unruly Mob").unwrap();
    let mob = state.create_object(um_id, P0, Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(mob).unwrap().name = "Unruly Mob".into();
    state.get_object_mut(mob).unwrap().summoning_sick = false;

    // Another creature on P0's side to sacrifice.
    let ally = ready_creature(&mut state, P0, 1, 1);

    // Kill the ally.
    state.get_object_mut(ally).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(ally).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(mob, CounterType::PlusOnePlusOne), 1,
        "Unruly Mob should have 1 +1/+1 counter");
    assert_eq!(state.effective_power(mob, &reg), Some(2));
    assert_eq!(state.effective_toughness(mob, &reg), Some(2));
}

/// Lumberknot gains a +1/+1 counter when any creature dies.
#[test]
fn lumberknot_gains_counter_on_any_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Lumberknot on P0's side.
    let lk_id = reg.get_id_by_name("Lumberknot").unwrap();
    let knot = state.create_object(lk_id, P0, Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(knot).unwrap().name = "Lumberknot".into();
    state.get_object_mut(knot).unwrap().summoning_sick = false;

    // A creature on P1's side to kill.
    let victim = ready_creature(&mut state, P1, 1, 1);

    // Kill the victim.
    state.get_object_mut(victim).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(knot, CounterType::PlusOnePlusOne), 1,
        "Lumberknot should have 1 +1/+1 counter");
    assert_eq!(state.effective_power(knot, &reg), Some(2));
    assert_eq!(state.effective_toughness(knot, &reg), Some(2));
}

/// Elder Cathar grants a +1/+1 counter to another creature you control when it dies.
#[test]
fn elder_cathar_grants_counter_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Elder Cathar on P0's side.
    let ec_id = reg.get_id_by_name("Elder Cathar").unwrap();
    let cathar = state.create_object(ec_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(cathar).unwrap().name = "Elder Cathar".into();
    state.get_object_mut(cathar).unwrap().summoning_sick = false;

    // Another creature on P0's side to receive the counter.
    let buddy = ready_creature(&mut state, P0, 2, 2);

    // Kill Elder Cathar.
    state.get_object_mut(cathar).unwrap().damage_marked = 2;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(cathar).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(buddy, CounterType::PlusOnePlusOne), 1,
        "Buddy creature should have received a +1/+1 counter from Elder Cathar");
    assert_eq!(state.effective_power(buddy, &reg), Some(3));
    assert_eq!(state.effective_toughness(buddy, &reg), Some(3));
}

/// Village Cannibals gains a +1/+1 counter when a Human dies, but not non-Humans.
#[test]
fn village_cannibals_gains_counter_on_human_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Village Cannibals on P0's side.
    let vc_id = reg.get_id_by_name("Village Cannibals").unwrap();
    let cannibals = state.create_object(vc_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(cannibals).unwrap().name = "Village Cannibals".into();
    state.get_object_mut(cannibals).unwrap().summoning_sick = false;

    // A Human creature on P1's side (Doomed Traveler is Human).
    let human_id = reg.get_id_by_name("Doomed Traveler").unwrap();
    let human = state.create_object(human_id, P1, Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(human).unwrap().name = "Doomed Traveler".into();
    state.get_object_mut(human).unwrap().summoning_sick = false;

    // Kill the Human.
    state.get_object_mut(human).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(human).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(cannibals, CounterType::PlusOnePlusOne), 1,
        "Village Cannibals should gain a counter when a Human dies");
    assert_eq!(state.effective_power(cannibals, &reg), Some(3));
    assert_eq!(state.effective_toughness(cannibals, &reg), Some(3));
}

/// Village Cannibals does NOT gain a counter when a non-Human dies.
#[test]
fn village_cannibals_ignores_non_human_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Village Cannibals on P0's side.
    let vc_id = reg.get_id_by_name("Village Cannibals").unwrap();
    let cannibals = state.create_object(vc_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(cannibals).unwrap().name = "Village Cannibals".into();
    state.get_object_mut(cannibals).unwrap().summoning_sick = false;

    // A non-Human creature (Walking Corpse is a Zombie, not Human).
    let zombie_id = reg.get_id_by_name("Walking Corpse").unwrap();
    let zombie = state.create_object(zombie_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(zombie).unwrap().name = "Walking Corpse".into();
    state.get_object_mut(zombie).unwrap().summoning_sick = false;

    // Kill the Zombie.
    state.get_object_mut(zombie).unwrap().damage_marked = 2;
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(zombie).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(cannibals, CounterType::PlusOnePlusOne), 0,
        "Village Cannibals should NOT gain a counter when a non-Human dies");
    assert_eq!(state.effective_power(cannibals, &reg), Some(2));
}

// ══════════════════════════════════════════════════════════════════
// Anthem enchantment
// ══════════════════════════════════════════════════════════════════

/// Intangible Virtue gives creature tokens you control +1/+1 and vigilance.
#[test]
fn intangible_virtue_buffs_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a token creature (Intangible Virtue only buffs tokens).
    let token = state.create_token(
        "Spirit", P0, 2, 2,
        vec![Color::White],
        vec![CardType::Creature],
        vec![],
    );
    state.get_object_mut(token).unwrap().summoning_sick = false;

    // Also create a non-token creature that should NOT be buffed.
    let non_token = ready_creature(&mut state, P0, 2, 2);

    // Cast Intangible Virtue.
    let iv_id = reg.get_id_by_name("Intangible Virtue").unwrap();
    let iv = state.create_object(iv_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: iv, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(iv).unwrap().zone, Zone::Battlefield);
    // Token should get +1/+1.
    assert_eq!(state.effective_power(token, &reg), Some(3),
        "Token should get +1 power from Intangible Virtue");
    assert_eq!(state.effective_toughness(token, &reg), Some(3),
        "Token should get +1 toughness from Intangible Virtue");
    // Token should have vigilance.
    assert!(state.has_keyword(token, Keyword::Vigilance, &reg),
        "Token should have vigilance from Intangible Virtue");
    // Non-token should NOT be buffed.
    assert_eq!(state.effective_power(non_token, &reg), Some(2),
        "Non-token should NOT get buffed by Intangible Virtue");
    assert_eq!(state.effective_toughness(non_token, &reg), Some(2),
        "Non-token should NOT get buffed by Intangible Virtue");
    // Non-token should NOT have vigilance.
    assert!(!state.has_keyword(non_token, Keyword::Vigilance, &reg),
        "Non-token should NOT have vigilance from Intangible Virtue");
}

// ══════════════════════════════════════════════════════════════════
// Token mechanics
// ══════════════════════════════════════════════════════════════════

/// Tokens have summoning sickness when created.
#[test]
fn token_has_summoning_sickness() {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let token = state.create_token(
        "Spirit", P0, 1, 1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![Keyword::Flying],
    );

    let obj = state.get_object(token).unwrap();
    assert!(obj.summoning_sick,
        "Token should have summoning sickness on the turn it was created");
}

/// Tokens cease to exist (are removed from the game) when killed.
#[test]
fn tokens_cease_to_exist_when_killed() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a Spirit token.
    let token = state.create_token(
        "Spirit", P0, 1, 1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![Keyword::Flying],
    );

    // Kill it by casting Lightning Bolt on it.
    let bolt_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let bolt = state.create_object(bolt_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Object(token)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Bolt deals 3 damage to the 1/1 token. Run SBAs to kill it.
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    // Token should be completely gone (not in graveyard, not anywhere).
    assert!(state.get_object(token).is_none(),
        "Token should cease to exist after dying — removed from the game entirely");
}

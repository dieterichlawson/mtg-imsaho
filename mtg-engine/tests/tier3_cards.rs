//! Tests for Innistrad Tier 3 cards: token-generating spells, creatures with
//! dies/ETB/death-watch triggers, anthem enchantments, and token mechanics.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ══════════════════════════════════════════════════════════════════
// Token-generating spells
// ══════════════════════════════════════════════════════════════════

/// Midnight Haunting creates two 1/1 Spirit tokens with flying.
#[test]
fn midnight_haunting_creates_two_spirits() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = castable_spell(&mut state, &reg, "Midnight Haunting", P0);

    state = cast_and_resolve(&state, &reg, card, vec![]);

    // Spell goes to graveyard.
    assert_eq!(state.get_object(card).unwrap().zone, Zone::Graveyard);

    // Two Spirit tokens on the battlefield.
    assert_eq!(count_tokens_named(&state, "Spirit"), 2, "Should have two Spirit tokens");

    for o in state.objects.values().filter(|o| o.is_token && o.name == "Spirit") {
        assert_eq!(o.power, Some(1));
        assert_eq!(o.toughness, Some(1));
        assert!(o.keywords.contains(&Keyword::Flying), "Spirit tokens should have flying");
    }
}

/// Moan of the Unhallowed creates two 2/2 Zombie tokens.
#[test]
fn moan_creates_two_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = castable_spell(&mut state, &reg, "Moan of the Unhallowed", P0);

    state = cast_and_resolve(&state, &reg, card, vec![]);

    assert_eq!(state.get_object(card).unwrap().zone, Zone::Graveyard);

    assert_eq!(count_tokens_named(&state, "Zombie"), 2, "Should have two Zombie tokens");

    for o in state.objects.values().filter(|o| o.is_token && o.name == "Zombie") {
        assert_eq!(o.power, Some(2));
        assert_eq!(o.toughness, Some(2));
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

    let dt = named_creature(&mut state, &reg, "Doomed Traveler", P0);

    // Kill it with lethal damage.
    state.get_object_mut(dt).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(dt).unwrap().zone, Zone::Graveyard,
        "Doomed Traveler should be dead");

    // Fire death triggers.
    triggers::process_triggers(&mut state, &reg);

    // Should have a Spirit token.
    assert_eq!(count_tokens_named(&state, "Spirit"), 1, "Should create one Spirit token");
    let spirit = find_token_named(&state, "Spirit").unwrap();
    let obj = state.get_object(spirit).unwrap();
    assert_eq!(obj.power, Some(1));
    assert_eq!(obj.toughness, Some(1));
    assert!(obj.keywords.contains(&Keyword::Flying));
}

/// Mausoleum Guard creates two 1/1 Spirit tokens with flying when it dies.
#[test]
fn mausoleum_guard_creates_two_spirits_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mg = named_creature(&mut state, &reg, "Mausoleum Guard", P0);

    // Kill it with lethal damage.
    state.get_object_mut(mg).unwrap().damage_marked = 2;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(mg).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(count_tokens_named(&state, "Spirit"), 2, "Should create two Spirit tokens");

    for s in state.objects.values().filter(|o| o.is_token && o.name == "Spirit") {
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
    let vbr = castable_spell(&mut state, &reg, "Village Bell-Ringer", P0);

    state = cast_and_resolve(&state, &reg, vbr, vec![]);

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
    let wc = named_creature(&mut state, &reg, "Walking Corpse", P1);

    // Cast Slayer of the Wicked.
    let slayer = castable_spell(&mut state, &reg, "Slayer of the Wicked", P0);

    state = cast_and_resolve(&state, &reg, slayer, vec![]);

    // Process ETB triggers — Slayer presents an optional choice.
    triggers::process_triggers(&mut state, &reg);

    // Choose to destroy the Walking Corpse.
    if state.awaiting_action.is_some() {
        state = engine::submit_action(
            &state,
            &Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(wc))),
            },
            &reg,
        );
        check_state_based_actions(&mut state, &reg);
    }

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
    let fh = castable_spell(&mut state, &reg, "Fiend Hunter", P0);

    state = cast_and_resolve(&state, &reg, fh, vec![]);

    // Process ETB triggers — Fiend Hunter presents a choice.
    triggers::process_triggers(&mut state, &reg);

    // Choose to exile the victim.
    if state.awaiting_action.is_some() {
        state = engine::submit_action(
            &state,
            &Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(victim))),
            },
            &reg,
        );
    }

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

    let pd = named_creature(&mut state, &reg, "Pitchburn Devils", P0);

    // Kill it with lethal damage.
    state.get_object_mut(pd).unwrap().damage_marked = 3;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(pd).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    // Should be awaiting a target choice (both players are valid targets).
    assert!(state.awaiting_action.is_some(), "Should be awaiting damage target choice");

    // Choose opponent as target. This attaches the target to the pending
    // trigger and pushes it on the stack.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Player(P1))) },
        &reg,
    );

    // Resolve the trigger on the stack so the damage is actually applied.
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 17,
        "Opponent should lose 3 life from Pitchburn Devils dying");
}

/// Falkenrath Noble drains 1 life whenever any creature dies.
#[test]
fn falkenrath_noble_drains_on_any_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Falkenrath Noble on P0's side.
    let _noble = named_creature(&mut state, &reg, "Falkenrath Noble", P0);

    // A creature on P0's side to kill (Noble triggers on any creature dying).
    let victim = ready_creature(&mut state, P0, 1, 1);

    // Kill the victim.
    state.get_object_mut(victim).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard);

    process_triggers_auto_target_opponent(&mut state, &reg);

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
    let _rt = named_creature(&mut state, &reg, "Rage Thrower", P0);

    // A creature on P1's side to kill.
    let victim = ready_creature(&mut state, P1, 1, 1);

    // Kill the victim.
    state.get_object_mut(victim).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard);

    triggers::process_triggers(&mut state, &reg);

    // Rage Thrower presents a "target player or planeswalker" choice.
    // Choose opponent (P1).
    assert!(state.awaiting_action.is_some(), "Rage Thrower should present a target choice");
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Player(P1))),
        },
        &reg,
    );
    // Resolve the trigger on the stack to actually apply the damage.
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
    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);

    // Another creature on P0's side to sacrifice.
    let ally = ready_creature(&mut state, P0, 1, 1);

    // Kill the ally.
    state.get_object_mut(ally).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

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
    let knot = named_creature(&mut state, &reg, "Lumberknot", P0);

    // A creature on P1's side to kill.
    let victim = ready_creature(&mut state, P1, 1, 1);

    // Kill the victim.
    state.get_object_mut(victim).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

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
    let cathar = named_creature(&mut state, &reg, "Elder Cathar", P0);

    // Another creature on P0's side to receive the counter.
    let buddy = ready_creature(&mut state, P0, 2, 2);

    // Kill Elder Cathar.
    state.get_object_mut(cathar).unwrap().damage_marked = 2;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

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
    let cannibals = named_creature(&mut state, &reg, "Village Cannibals", P0);

    // A Human creature on P1's side (Doomed Traveler is Human).
    let human = named_creature(&mut state, &reg, "Doomed Traveler", P1);

    // Kill the Human.
    state.get_object_mut(human).unwrap().damage_marked = 1;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

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
    let cannibals = named_creature(&mut state, &reg, "Village Cannibals", P0);

    // A non-Human creature (Walking Corpse is a Zombie, not Human).
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P1);

    // Kill the Zombie.
    state.get_object_mut(zombie).unwrap().damage_marked = 2;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

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
        &reg,
    )[0];
    state.get_object_mut(token).unwrap().summoning_sick = false;

    // Also create a non-token creature that should NOT be buffed.
    let non_token = ready_creature(&mut state, P0, 2, 2);

    // Cast Intangible Virtue.
    let iv = castable_spell(&mut state, &reg, "Intangible Virtue", P0);

    state = cast_and_resolve(&state, &reg, iv, vec![]);

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

    let reg = registry();
    let token = state.create_token(
        "Spirit", P0, 1, 1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![Keyword::Flying],
        &reg,
    )[0];

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
        &reg,
    )[0];

    // Kill it by casting Lightning Bolt on it.
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    state = cast_and_resolve(&state, &reg, bolt, vec![Target::Object(token)]);

    // Bolt deals 3 damage to the 1/1 token. Run SBAs to kill it.
    check_state_based_actions(&mut state, &reg);

    // Token should be completely gone (not in graveyard, not anywhere).
    assert!(state.get_object(token).is_none(),
        "Token should cease to exist after dying — removed from the game entirely");
}

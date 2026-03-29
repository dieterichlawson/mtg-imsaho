//! Tests for card mechanics: morbid, leave-battlefield triggers,
//! forced attack, combat damage prevention, protection, token anthems,
//! opponent debuffs, conditional auras, unblockable, can't-block,
//! untap prevention.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

use mtg_engine::ids::CardId;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ══════════════════════════════════════════════════════════════════
// Morbid
// ══════════════════════════════════════════════════════════════════

/// creature_died_this_turn is set when a creature dies via SBA.
#[test]
fn morbid_flag_set_on_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    assert!(!state.creature_died_this_turn);

    let creature = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(creature).unwrap().damage_marked = 1;

    check_state_based_actions_with_registry(&mut state, Some(&reg));
    assert!(state.creature_died_this_turn,
        "Morbid flag should be set after creature dies");
}

/// creature_died_this_turn resets at start of new turn.
#[test]
fn morbid_flag_resets_on_new_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    // Advance to next turn.
    loop {
        engine::advance_step(&mut state, &reg);
        if state.turn_number > 1 { break; }
    }

    assert!(!state.creature_died_this_turn,
        "Morbid flag should reset at start of new turn");
}

/// Brimstone Volley deals 5 damage when morbid.
#[test]
fn brimstone_volley_morbid_deals_5() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let bv_id = reg.get_id_by_name("Brimstone Volley").unwrap();
    let bv = state.create_object(bv_id, P0, Zone::Hand, None, None);
    state.get_object_mut(bv).unwrap().name = "Brimstone Volley".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 3);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bv, targets: vec![Target::Player(P1)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 15,
        "Brimstone Volley with morbid should deal 5 damage (20 - 5 = 15)");
}

/// Brimstone Volley deals 3 damage without morbid.
#[test]
fn brimstone_volley_no_morbid_deals_3() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    assert!(!state.creature_died_this_turn);

    let bv_id = reg.get_id_by_name("Brimstone Volley").unwrap();
    let bv = state.create_object(bv_id, P0, Zone::Hand, None, None);
    state.get_object_mut(bv).unwrap().name = "Brimstone Volley".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 3);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bv, targets: vec![Target::Player(P1)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 17,
        "Brimstone Volley without morbid should deal 3 damage (20 - 3 = 17)");
}

/// Somberwald Spider enters with +1/+1 counters when morbid.
#[test]
fn somberwald_spider_morbid_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let spider_id = reg.get_id_by_name("Somberwald Spider").unwrap();
    let spider = state.create_object(spider_id, P0, Zone::Hand, Some(2), Some(4));
    state.get_object_mut(spider).unwrap().name = "Somberwald Spider".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 5);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spider, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(spider, CounterType::PlusOnePlusOne), 2,
        "Somberwald Spider should enter with 2 +1/+1 counters when morbid");
    assert_eq!(state.effective_power(spider, &reg), Some(4)); // 2 base + 2 counters
    assert_eq!(state.effective_toughness(spider, &reg), Some(6)); // 4 base + 2 counters
}

/// Somberwald Spider has no counters without morbid.
#[test]
fn somberwald_spider_no_morbid_no_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    assert!(!state.creature_died_this_turn);

    let spider_id = reg.get_id_by_name("Somberwald Spider").unwrap();
    let spider = state.create_object(spider_id, P0, Zone::Hand, Some(2), Some(4));
    state.get_object_mut(spider).unwrap().name = "Somberwald Spider".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 5);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spider, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(spider, CounterType::PlusOnePlusOne), 0);
}

// ══════════════════════════════════════════════════════════════════
// Fiend Hunter leave-battlefield
// ══════════════════════════════════════════════════════════════════

/// Fiend Hunter returns the exiled creature when it leaves the battlefield.
#[test]
fn fiend_hunter_returns_exiled_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a creature.
    let target = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(target).unwrap().name = "Target Creature".into();

    // Cast Fiend Hunter (ETB exiles the target).
    let fh_id = reg.get_id_by_name("Fiend Hunter").unwrap();
    let fh = state.create_object(fh_id, P0, Zone::Hand, Some(1), Some(3));
    state.get_object_mut(fh).unwrap().name = "Fiend Hunter".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 3);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: fh, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(target).unwrap().zone, Zone::Exile,
        "Target should be exiled by Fiend Hunter ETB");
    assert_eq!(state.get_object(fh).unwrap().zone, Zone::Battlefield);

    // Now kill the Fiend Hunter.
    state.get_object_mut(fh).unwrap().damage_marked = 3;
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(fh).unwrap().zone, Zone::Graveyard,
        "Fiend Hunter should be dead");
    assert_eq!(state.get_object(target).unwrap().zone, Zone::Battlefield,
        "Exiled creature should return to the battlefield when Fiend Hunter dies");
}

// ══════════════════════════════════════════════════════════════════
// Nightbird's Clutches — can't block this turn
// ══════════════════════════════════════════════════════════════════

/// Nightbird's Clutches prevents blocking, not just taps.
#[test]
fn nightbirds_clutches_prevents_blocking() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let blocker = ready_creature(&mut state, P1, 3, 3);
    let nc_id = reg.get_id_by_name("Nightbird's Clutches").unwrap();
    let nc = state.create_object(nc_id, P0, Zone::Hand, None, None);
    state.get_object_mut(nc).unwrap().name = "Nightbird's Clutches".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: nc, targets: vec![Target::Object(blocker)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.until_end_of_turn_cant_block.contains(&blocker));

    let eligible = combat::eligible_blockers_with_registry(&state, P1, &reg);
    assert!(!eligible.contains(&blocker),
        "Creature targeted by Nightbird's Clutches should not be eligible to block");
}

// ══════════════════════════════════════════════════════════════════
// Bonds of Faith — conditional aura
// ══════════════════════════════════════════════════════════════════

/// Bonds of Faith gives +2/+2 to a Human creature.
#[test]
fn bonds_of_faith_buffs_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Savannah Lions is a Human (Cat Warrior — actually, let's use Elder Cathar which is Human Soldier).
    let human_id = reg.get_id_by_name("Elder Cathar").unwrap();
    let creature = state.create_object(human_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().name = "Elder Cathar".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;

    let bof_id = reg.get_id_by_name("Bonds of Faith").unwrap();
    let bof = state.create_object(bof_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bof, targets: vec![Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    // Human should get +2/+2.
    assert_eq!(state.effective_power(creature, &reg), Some(4));
    assert_eq!(state.effective_toughness(creature, &reg), Some(4));
    assert!(state.can_attack(creature, &reg), "Human with Bonds should be able to attack");
}

/// Bonds of Faith locks down a non-Human creature.
#[test]
fn bonds_of_faith_locks_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Grizzly Bears is a Bear, not a Human.
    let bears = ready_creature(&mut state, P1, 2, 2);

    let bof_id = reg.get_id_by_name("Bonds of Faith").unwrap();
    let bof = state.create_object(bof_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bof, targets: vec![Target::Object(bears)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert!(!state.can_attack(bears, &reg), "Non-Human with Bonds should not attack");
    assert!(!state.can_block(bears, &reg), "Non-Human with Bonds should not block");
    // No P/T bonus.
    assert_eq!(state.effective_power(bears, &reg), Some(2));
}

// ══════════════════════════════════════════════════════════════════
// Furor of the Bitten — forced attack
// ══════════════════════════════════════════════════════════════════

/// Creature enchanted with Furor of the Bitten is forced to attack.
#[test]
fn furor_forces_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().controller = P0;

    // Attach Furor of the Bitten.
    let furor_id = reg.get_id_by_name("Furor of the Bitten").unwrap();
    let furor = state.create_object(furor_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(furor).unwrap().name = "Furor of the Bitten".into();
    state.get_object_mut(furor).unwrap().attached_to = Some(creature);
    state.get_object_mut(furor).unwrap().summoning_sick = false;

    // Player declares zero attackers.
    state = engine::submit_action(
        &state,
        &Action::DeclareAttackers { attackers: vec![] },
        &reg,
    );

    // The forced attacker should have been auto-added.
    let is_attacking = state.combat.as_ref()
        .map(|c| c.attackers.contains_key(&creature))
        .unwrap_or(false);
    assert!(is_attacking,
        "Creature with Furor of the Bitten should be forced to attack even if not declared");
}

// ══════════════════════════════════════════════════════════════════
// Ghostly Possession — damage prevention
// ══════════════════════════════════════════════════════════════════

/// Creature with Ghostly Possession takes no combat damage.
#[test]
fn ghostly_possession_prevents_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);

    // Attach Ghostly Possession to the blocker.
    let gp_id = reg.get_id_by_name("Ghostly Possession").unwrap();
    let gp = state.create_object(gp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(gp).unwrap().name = "Ghostly Possession".into();
    state.get_object_mut(gp).unwrap().attached_to = Some(blocker);
    state.get_object_mut(gp).unwrap().summoning_sick = false;

    combat::declare_attackers(&mut state, &[(attacker, P1)]);
    combat::declare_blockers(&mut state, &[(blocker, attacker)]);
    combat::deal_combat_damage(&mut state, &reg);

    // Blocker with Ghostly Possession should take no damage.
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "Ghostly Possession should prevent combat damage TO the creature");
    // Attacker should also take no damage (prevented FROM the creature).
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "Ghostly Possession should prevent combat damage FROM the creature");
}

// ══════════════════════════════════════════════════════════════════
// Grave Bramble — protection from Zombies
// ══════════════════════════════════════════════════════════════════

/// Zombie can't deal damage to Grave Bramble (protection from Zombies).
#[test]
fn grave_bramble_protection_prevents_zombie_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Attacker is a Zombie (Walking Corpse).
    let wc_id = reg.get_id_by_name("Walking Corpse").unwrap();
    let zombie = state.create_object(wc_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(zombie).unwrap().name = "Walking Corpse".into();
    state.get_object_mut(zombie).unwrap().summoning_sick = false;

    // Blocker is Grave Bramble (protection from Zombies).
    let gb_id = reg.get_id_by_name("Grave Bramble").unwrap();
    let bramble = state.create_object(gb_id, P1, Zone::Battlefield, Some(3), Some(4));
    state.get_object_mut(bramble).unwrap().name = "Grave Bramble".into();
    state.get_object_mut(bramble).unwrap().summoning_sick = false;

    combat::declare_attackers(&mut state, &[(zombie, P1)]);
    combat::declare_blockers(&mut state, &[(bramble, zombie)]);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(bramble).unwrap().damage_marked, 0,
        "Grave Bramble should take no damage from Zombie (protection)");
    // Grave Bramble still deals damage to the Zombie.
    assert_eq!(state.get_object(zombie).unwrap().damage_marked, 3,
        "Grave Bramble should still deal damage to the Zombie");
}

// ══════════════════════════════════════════════════════════════════
// Intangible Virtue — token anthem + vigilance
// ══════════════════════════════════════════════════════════════════

/// Intangible Virtue buffs tokens but not non-tokens.
#[test]
fn intangible_virtue_token_only() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a token and a non-token creature.
    let token = state.create_token("Spirit", P0, 1, 1, vec![Color::White],
        vec![CardType::Creature], vec![Keyword::Flying]);
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

    // Token should get +1/+1 and vigilance.
    assert_eq!(state.effective_power(token, &reg), Some(2),
        "Token should get +1 power from Intangible Virtue");
    assert_eq!(state.effective_toughness(token, &reg), Some(2),
        "Token should get +1 toughness from Intangible Virtue");
    assert!(state.has_keyword(token, Keyword::Vigilance, &reg),
        "Token should have vigilance from Intangible Virtue");

    // Non-token should NOT be affected.
    assert_eq!(state.effective_power(non_token, &reg), Some(2),
        "Non-token should not get bonus from Intangible Virtue");
    assert_eq!(state.effective_toughness(non_token, &reg), Some(2));
    assert!(!state.has_keyword(non_token, Keyword::Vigilance, &reg),
        "Non-token should not get vigilance from Intangible Virtue");
}

// ══════════════════════════════════════════════════════════════════
// One-Eyed Scarecrow — opponent flyer debuff
// ══════════════════════════════════════════════════════════════════

/// One-Eyed Scarecrow gives -1/-0 to opponent's flyers.
#[test]
fn one_eyed_scarecrow_debuffs_opponent_flyers() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has One-Eyed Scarecrow on battlefield.
    let scarecrow_id = reg.get_id_by_name("One-Eyed Scarecrow").unwrap();
    let scarecrow = state.create_object(scarecrow_id, P0, Zone::Battlefield, Some(2), Some(3));
    state.get_object_mut(scarecrow).unwrap().name = "One-Eyed Scarecrow".into();
    state.get_object_mut(scarecrow).unwrap().summoning_sick = false;

    // P1 has a flyer (Moon Heron 3/2 flying).
    let heron_id = reg.get_id_by_name("Moon Heron").unwrap();
    let flyer = state.create_object(heron_id, P1, Zone::Battlefield, Some(3), Some(2));
    state.get_object_mut(flyer).unwrap().name = "Moon Heron".into();
    state.get_object_mut(flyer).unwrap().summoning_sick = false;
    state.get_object_mut(flyer).unwrap().keywords = vec![Keyword::Flying];

    // P1 has a ground creature.
    let ground = ready_creature(&mut state, P1, 2, 2);

    // Flyer should be debuffed to 2/2 (3-1=2 power).
    assert_eq!(state.effective_power(flyer, &reg), Some(2),
        "Opponent's flyer should get -1 power from One-Eyed Scarecrow");
    assert_eq!(state.effective_toughness(flyer, &reg), Some(2),
        "Toughness should be unchanged");

    // Ground creature should NOT be debuffed.
    assert_eq!(state.effective_power(ground, &reg), Some(2),
        "Opponent's ground creature should not be affected");

    // P0's own flyers should NOT be debuffed.
    let own_flyer = state.create_token("Spirit", P0, 1, 1, vec![Color::White],
        vec![CardType::Creature], vec![Keyword::Flying]);
    assert_eq!(state.effective_power(own_flyer, &reg), Some(1),
        "Own flyers should not be debuffed by own Scarecrow");
}

// ══════════════════════════════════════════════════════════════════
// Elder Cathar — Human bonus
// ══════════════════════════════════════════════════════════════════

/// Elder Cathar gives 2 counters to a Human, 1 to non-Human.
#[test]
fn elder_cathar_gives_two_counters_to_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Elder Cathar on battlefield.
    let ec_id = reg.get_id_by_name("Elder Cathar").unwrap();
    let ec = state.create_object(ec_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(ec).unwrap().name = "Elder Cathar".into();
    state.get_object_mut(ec).unwrap().summoning_sick = false;

    // A Human ally (Doomed Traveler).
    let dt_id = reg.get_id_by_name("Doomed Traveler").unwrap();
    let human = state.create_object(dt_id, P0, Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(human).unwrap().name = "Doomed Traveler".into();
    state.get_object_mut(human).unwrap().summoning_sick = false;

    // Kill Elder Cathar.
    state.get_object_mut(ec).unwrap().damage_marked = 2;
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(human, CounterType::PlusOnePlusOne), 2,
        "Human should get 2 +1/+1 counters from Elder Cathar");
}

#[test]
fn elder_cathar_gives_one_counter_to_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ec_id = reg.get_id_by_name("Elder Cathar").unwrap();
    let ec = state.create_object(ec_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(ec).unwrap().name = "Elder Cathar".into();
    state.get_object_mut(ec).unwrap().summoning_sick = false;

    // A non-Human ally (Grizzly Bears = Bear).
    let bears = ready_creature(&mut state, P0, 2, 2);

    state.get_object_mut(ec).unwrap().damage_marked = 2;
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(bears, CounterType::PlusOnePlusOne), 1,
        "Non-Human should get only 1 +1/+1 counter from Elder Cathar");
}

// ══════════════════════════════════════════════════════════════════
// Invisible Stalker — can't be blocked
// ══════════════════════════════════════════════════════════════════

/// Invisible Stalker can't be blocked by any creature.
#[test]
fn invisible_stalker_unblockable() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let stalker_id = reg.get_id_by_name("Invisible Stalker").unwrap();
    let stalker = state.create_object(stalker_id, P0, Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(stalker).unwrap().name = "Invisible Stalker".into();
    state.get_object_mut(stalker).unwrap().summoning_sick = false;

    let blocker = ready_creature(&mut state, P1, 5, 5);

    assert!(!combat::can_block_attacker(&state, blocker, stalker, &reg),
        "No creature should be able to block Invisible Stalker");
}

// ══════════════════════════════════════════════════════════════════
// Vampire Interloper — can't block
// ══════════════════════════════════════════════════════════════════

/// Vampire Interloper can't be used as a blocker.
#[test]
fn vampire_interloper_cant_block() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let vi_id = reg.get_id_by_name("Vampire Interloper").unwrap();
    let vi = state.create_object(vi_id, P1, Zone::Battlefield, Some(2), Some(1));
    state.get_object_mut(vi).unwrap().name = "Vampire Interloper".into();
    state.get_object_mut(vi).unwrap().summoning_sick = false;

    let eligible = combat::eligible_blockers_with_registry(&state, P1, &reg);
    assert!(!eligible.contains(&vi),
        "Vampire Interloper should not be eligible to block");
}

// ══════════════════════════════════════════════════════════════════
// Claustrophobia — prevents untapping
// ══════════════════════════════════════════════════════════════════

/// Creature enchanted by Claustrophobia doesn't untap during untap step.
#[test]
fn claustrophobia_prevents_untap() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a creature with Claustrophobia attached.
    let creature = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(creature).unwrap().tapped = true;

    let cl_id = reg.get_id_by_name("Claustrophobia").unwrap();
    let cl = state.create_object(cl_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(cl).unwrap().name = "Claustrophobia".into();
    state.get_object_mut(cl).unwrap().attached_to = Some(creature);
    state.get_object_mut(cl).unwrap().summoning_sick = false;

    // Advance to P1's untap step.
    state.active_player = PlayerId(1);
    state.step = Step::Cleanup; // will advance to P0's untap, then eventually P1's
    // Directly simulate P1's untap step.
    state.step = Step::Untap;
    state.active_player = PlayerId(1);
    engine::advance_step(&mut state, &reg); // This performs untap actions for P1's untap
    // Actually advance_step moves to the NEXT step. Let me call perform_turn_based_actions
    // by setting up the untap step properly.

    // Simpler: just run the untap logic directly. Set up as P1's turn at untap.
    let mut state2 = game_at_step(Step::PrecombatMain, PlayerId(1));
    let creature2 = state2.create_object(CardId(99), PlayerId(1), Zone::Battlefield, Some(3), Some(3));
    state2.get_object_mut(creature2).unwrap().summoning_sick = false;
    state2.get_object_mut(creature2).unwrap().tapped = true;

    let cl2 = state2.create_object(cl_id, P0, Zone::Battlefield, None, None);
    state2.get_object_mut(cl2).unwrap().name = "Claustrophobia".into();
    state2.get_object_mut(cl2).unwrap().attached_to = Some(creature2);
    state2.get_object_mut(cl2).unwrap().summoning_sick = false;

    // Also add a normal tapped creature (should untap normally).
    let normal = state2.create_object(CardId(98), PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    state2.get_object_mut(normal).unwrap().summoning_sick = false;
    state2.get_object_mut(normal).unwrap().tapped = true;

    // Simulate the untap step by going through a full turn cycle.
    state2.step = Step::Cleanup;
    state2.active_player = PlayerId(0);
    // Advance until we hit P1's Draw step (which means untap just happened).
    loop {
        engine::advance_step(&mut state2, &reg);
        if state2.active_player == PlayerId(1) && state2.step == Step::Draw {
            break;
        }
    }

    assert!(state2.get_object(creature2).unwrap().tapped,
        "Creature with Claustrophobia should remain tapped after untap step");
    assert!(!state2.get_object(normal).unwrap().tapped,
        "Normal creature should untap normally");
}

// ══════════════════════════════════════════════════════════════════
// Feeling of Dread — targets two creatures
// ══════════════════════════════════════════════════════════════════

/// Feeling of Dread can tap two creatures.
#[test]
fn feeling_of_dread_taps_two() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature1 = ready_creature(&mut state, P1, 3, 3);
    let creature2 = ready_creature(&mut state, P1, 2, 2);

    let fod_id = reg.get_id_by_name("Feeling of Dread").unwrap();
    let fod = state.create_object(fod_id, P0, Zone::Hand, None, None);
    state.get_object_mut(fod).unwrap().name = "Feeling of Dread".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: fod,
            targets: vec![Target::Object(creature1), Target::Object(creature2)],
        },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.get_object(creature1).unwrap().tapped,
        "First target should be tapped");
    assert!(state.get_object(creature2).unwrap().tapped,
        "Second target should be tapped");
}

// ══════════════════════════════════════════════════════════════════
// Mid-resolution choices
// ══════════════════════════════════════════════════════════════════

/// Frightful Delusion presents a choice when opponent has mana.
#[test]
fn frightful_delusion_choice_when_opponent_has_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts a creature.
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, P0, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bears, targets: vec![] },
        &reg,
    );

    // P1 casts Frightful Delusion. Give P0 mana so they CAN pay.
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1); // P0 has {1} to pay
    let fd_id = reg.get_id_by_name("Frightful Delusion").unwrap();
    let fd = state.create_object(fd_id, P1, Zone::Hand, None, None);
    state.get_object_mut(fd).unwrap().name = "Frightful Delusion".into();
    state.get_player_mut(P1).mana_pool.add(ManaType::Blue, 3);
    state.priority_player = Some(P1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: fd, targets: vec![Target::Object(bears)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should have set an awaiting_action for P0 to choose.
    assert!(state.awaiting_action.is_some(),
        "Frightful Delusion should present a choice when opponent has mana");

    // P0 chooses not to pay — spell gets countered.
    let legal = engine::legal_actions(&state, &reg);
    assert!(legal.actions.len() >= 2, "Should have pay/don't pay options");
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: mtg_engine::actions::ResolvedChoice::PayDecision(false) },
        &reg,
    );

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Bears should be countered when opponent doesn't pay");
}

/// Frightful Delusion auto-counters when opponent has no mana.
#[test]
fn frightful_delusion_auto_counters_without_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, P0, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bears, targets: vec![] },
        &reg,
    );

    // P0 has NO mana (pool empty after casting).
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);

    let fd_id = reg.get_id_by_name("Frightful Delusion").unwrap();
    let fd = state.create_object(fd_id, P1, Zone::Hand, None, None);
    state.get_object_mut(fd).unwrap().name = "Frightful Delusion".into();
    state.get_player_mut(P1).mana_pool.add(ManaType::Blue, 3);
    state.priority_player = Some(P1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: fd, targets: vec![Target::Object(bears)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should auto-counter (no choice needed).
    assert!(state.awaiting_action.is_none(),
        "Should auto-counter when opponent has no mana");
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Bears should be auto-countered");
}

/// Unburial Rites presents a choice when multiple creatures in graveyard.
#[test]
fn unburial_rites_choice_with_multiple_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put two creatures in P0's graveyard.
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();

    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let tusker = state.create_object(tusker_id, P0, Zone::Graveyard, Some(3), Some(3));
    state.get_object_mut(tusker).unwrap().name = "Kalonian Tusker".into();

    // Cast Unburial Rites.
    let ur_id = reg.get_id_by_name("Unburial Rites").unwrap();
    let ur = state.create_object(ur_id, P0, Zone::Hand, None, None);
    state.get_object_mut(ur).unwrap().name = "Unburial Rites".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 5);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: ur, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should have a choice between the two creatures.
    assert!(state.awaiting_action.is_some(),
        "Unburial Rites should present a choice with 2 creatures");

    // Choose to return the Tusker.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(
                Some(Target::Object(tusker))
            ),
        },
        &reg,
    );

    assert_eq!(state.get_object(tusker).unwrap().zone, Zone::Battlefield,
        "Chosen creature should return to battlefield");
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Unchosen creature should stay in graveyard");
}

/// Pitchburn Devils choice with multiple targets.
#[test]
fn pitchburn_devils_choice_with_targets() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Pitchburn Devils on P0's battlefield.
    let pd_id = reg.get_id_by_name("Pitchburn Devils").unwrap();
    let pd = state.create_object(pd_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(pd).unwrap().name = "Pitchburn Devils".into();
    state.get_object_mut(pd).unwrap().summoning_sick = false;

    // P1 has a creature (so there are multiple damage targets).
    let blocker = ready_creature(&mut state, P1, 4, 4);

    // Kill Pitchburn Devils.
    state.events.clear();
    state.get_object_mut(pd).unwrap().damage_marked = 3;
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    // Should have a choice (creature + both players = 3+ targets).
    assert!(state.awaiting_action.is_some(),
        "Pitchburn Devils should present a choice with multiple targets");

    // Choose to damage the opponent's creature.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(
                Some(Target::Object(blocker))
            ),
        },
        &reg,
    );

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 3,
        "Chosen creature should take 3 damage");
}

/// Forbidden Alchemy choice from top 4 cards.
#[test]
fn forbidden_alchemy_choice_from_top_4() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a library with 4+ known cards.
    let bolt_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let growth_id = reg.get_id_by_name("Giant Growth").unwrap();

    let c1 = state.create_object(bolt_id, P0, Zone::Library, None, None);
    state.get_object_mut(c1).unwrap().name = "Lightning Bolt".into();
    let c2 = state.create_object(bears_id, P0, Zone::Library, Some(2), Some(2));
    state.get_object_mut(c2).unwrap().name = "Grizzly Bears".into();
    let c3 = state.create_object(forest_id, P0, Zone::Library, None, None);
    state.get_object_mut(c3).unwrap().name = "Forest".into();
    let c4 = state.create_object(growth_id, P0, Zone::Library, None, None);
    state.get_object_mut(c4).unwrap().name = "Giant Growth".into();
    state.players[0].library_order = vec![c1, c2, c3, c4];

    // Cast Forbidden Alchemy.
    let fa_id = reg.get_id_by_name("Forbidden Alchemy").unwrap();
    let fa = state.create_object(fa_id, P0, Zone::Hand, None, None);
    state.get_object_mut(fa).unwrap().name = "Forbidden Alchemy".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 3);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: fa, targets: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should present a choice of 4 cards.
    assert!(state.awaiting_action.is_some(),
        "Forbidden Alchemy should present a choice from top 4");

    // Choose Lightning Bolt.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenCard(c1),
        },
        &reg,
    );

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Hand,
        "Chosen card should be in hand");
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Graveyard,
        "Unchosen card should be in graveyard");
    assert_eq!(state.get_object(c3).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(c4).unwrap().zone, Zone::Graveyard);
}

// ══════════════════════════════════════════════════════════════════
// Regeneration
// ══════════════════════════════════════════════════════════════════

/// A regeneration shield prevents death from lethal damage.
#[test]
fn regeneration_shield_prevents_lethal_damage_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;
    state.get_object_mut(creature).unwrap().damage_marked = 2; // lethal

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    // Should have regenerated, not died.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature with regeneration shield should not die from lethal damage");
    assert!(state.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
        "Regenerated creature should have damage removed");
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "One shield should be consumed");
}

/// Regeneration does NOT prevent death from 0 toughness.
#[test]
fn regeneration_does_not_prevent_zero_toughness_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Creature with 0 effective toughness (e.g., from -2/-2 aura on a 2/2).
    let creature = ready_creature(&mut state, P0, 2, 0);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Regeneration should not save from 0 toughness");
}

/// Multiple regeneration shields stack — second lethal damage also regenerates.
#[test]
fn multiple_regeneration_shields() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().regeneration_shields = 2;
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1,
        "One shield consumed, one remaining");

    // Deal lethal damage again.
    state.get_object_mut(creature).unwrap().damage_marked = 2;
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Second shield should save from second lethal");
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0);
}

/// Regeneration shields expire at end of turn (cleanup step).
#[test]
fn regeneration_shields_expire_at_cleanup() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;

    // Advance to cleanup.
    loop {
        engine::advance_step(&mut state, &reg);
        if state.step == Step::Cleanup { break; }
    }

    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "Unused regeneration shields should expire at cleanup");
}

/// destroy_creature helper respects regeneration.
#[test]
fn destroy_creature_respects_regeneration() {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;

    let destroyed = state.destroy_creature(creature);

    assert!(!destroyed, "destroy_creature should return false when regeneration saves");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);
    assert!(state.get_object(creature).unwrap().tapped);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0);
}

/// destroy_creature without shields actually destroys.
#[test]
fn destroy_creature_without_shield_kills() {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0);

    let destroyed = state.destroy_creature(creature);

    assert!(destroyed, "destroy_creature should return true when no shields");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Regeneration saves from deathtouch damage.
#[test]
fn regeneration_saves_from_deathtouch() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 5, 5);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;
    state.get_object_mut(creature).unwrap().damage_marked = 1;
    state.get_object_mut(creature).unwrap().dealt_deathtouch_damage = true;

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Regeneration should save from deathtouch damage");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0);
}

// ══════════════════════════════════════════════════════════════════
// Skeletal Grimace — activated ability + regeneration
// ══════════════════════════════════════════════════════════════════

/// Skeletal Grimace grants {B}: Regenerate as an activated ability.
#[test]
fn skeletal_grimace_grants_regenerate_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a creature and attach Skeletal Grimace.
    let creature = ready_creature(&mut state, P0, 2, 2);
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, P0, Zone::Hand, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: sg, targets: vec![Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Creature should have +1/+1 from the aura.
    assert_eq!(state.effective_power(creature, &reg), Some(3));
    assert_eq!(state.effective_toughness(creature, &reg), Some(3));

    // Activate the regenerate ability: pay {B}, get a shield.
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(activate.is_some(), "Should be able to activate {{B}}: Regenerate");

    state = engine::submit_action(&state, activate.unwrap(), &reg);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1,
        "Activating should add a regeneration shield");
}

/// Skeletal Grimace regeneration saves creature from lethal damage.
#[test]
fn skeletal_grimace_regeneration_saves_from_lethal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, P0, Zone::Hand, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: sg, targets: vec![Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Activate regenerate.
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);

    // Verify shield is active before dealing damage.
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1,
        "Shield should be active before damage");
    assert_eq!(state.effective_toughness(creature, &reg), Some(3),
        "Effective toughness should be 3 with aura");

    // Now deal lethal damage (effective toughness is 3 with the aura).
    state.get_object_mut(creature).unwrap().damage_marked = 3;
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    // Should have regenerated, not died.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature with Skeletal Grimace regeneration should survive lethal");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
        "Damage should be removed after regeneration");
    assert!(state.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "Shield should be consumed");
}

/// Skeletal Grimace regeneration saves creature from Doom Blade.
#[test]
fn skeletal_grimace_regeneration_vs_doom_blade() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0's creature with Skeletal Grimace attached.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().name = "Runeclaw Bear".into();
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, P0, Zone::Hand, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    // Cast and resolve Skeletal Grimace.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: sg, targets: vec![Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Activate regenerate.
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1);

    // P1 casts Doom Blade targeting the creature.
    state.priority_player = Some(P1);
    let db_id = reg.get_id_by_name("Doom Blade").unwrap();
    let db = state.create_object(db_id, P1, Zone::Hand, None, None);
    state.get_object_mut(db).unwrap().name = "Doom Blade".into();
    state.get_player_mut(P1).mana_pool.add(ManaType::Black, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: db, targets: vec![Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Creature should survive via regeneration.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Regeneration should save from Doom Blade");
    assert!(state.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "Shield should be consumed");
}

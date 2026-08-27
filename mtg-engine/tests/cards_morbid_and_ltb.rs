//! Tests for card mechanics: morbid, leave-battlefield triggers,
//! forced attack, combat damage prevention, protection, token anthems,
//! opponent debuffs, conditional auras, unblockable, can't-block,
//! untap prevention.
//!
//! Cards covered (20), so this is greppable by name as well as by rule:
//!
//! - Bonds of Faith
//! - Brimstone Volley
//! - Claustrophobia
//! - Elder Cathar
//! - Feeling of Dread
//! - Fiend Hunter
//! - Forbidden Alchemy
//! - Frightful Delusion
//! - Ghostly Possession
//! - Grave Bramble
//! - Intangible Virtue
//! - Invisible Stalker
//! - Morkrut Banshee
//! - Nightbird's Clutches
//! - One-Eyed Scarecrow
//! - Pitchburn Devils
//! - Skeletal Grimace
//! - Somberwald Spider
//! - Unburial Rites
//! - Vampire Interloper

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::{CardId, PlayerId};
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ══════════════════════════════════════════════════════════════════
// Morbid
// ══════════════════════════════════════════════════════════════════

/// `creature_died_this_turn` is set when a creature dies via SBA.
#[test]
fn morbid_flag_set_on_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    assert!(!state.creature_died_this_turn);

    let creature = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(creature).unwrap().damage_marked = 1;

    check_state_based_actions(&mut state, &reg);
    assert!(state.creature_died_this_turn,
        "Morbid flag should be set after creature dies");
}

/// `creature_died_this_turn` resets at start of new turn.
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

    let bv = castable_spell(&mut state, &reg, "Brimstone Volley", P0);

    state = cast_and_resolve(&state, &reg, bv, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P1).life, 15,
        "Brimstone Volley with morbid should deal 5 damage (20 - 5 = 15)");
}

/// Brimstone Volley deals 3 damage without morbid.
#[test]
fn brimstone_volley_no_morbid_deals_3() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    assert!(!state.creature_died_this_turn);

    let bv = castable_spell(&mut state, &reg, "Brimstone Volley", P0);

    state = cast_and_resolve(&state, &reg, bv, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P1).life, 17,
        "Brimstone Volley without morbid should deal 3 damage (20 - 3 = 17)");
}

/// Somberwald Spider enters with +1/+1 counters when morbid.
#[test]
fn somberwald_spider_morbid_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let spider = castable_spell(&mut state, &reg, "Somberwald Spider", P0);

    state = cast_and_resolve(&state, &reg, spider, vec![]);
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

    let spider = castable_spell(&mut state, &reg, "Somberwald Spider", P0);

    state = cast_and_resolve(&state, &reg, spider, vec![]);
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
    let fh = castable_spell(&mut state, &reg, "Fiend Hunter", P0);

    state = cast_and_resolve(&state, &reg, fh, vec![]);
    triggers::process_triggers(&mut state, &reg);

    // Fiend Hunter now presents a choice — choose to exile the target.
    if state.awaiting_action.is_some() {
        state = engine::submit_action(
            &state,
            &Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(target))),
            },
            &reg,
        );
    }

    assert_eq!(state.get_object(target).unwrap().zone, Zone::Exile,
        "Target should be exiled by Fiend Hunter ETB");
    assert_eq!(state.get_object(fh).unwrap().zone, Zone::Battlefield);

    // Now kill the Fiend Hunter.
    state.get_object_mut(fh).unwrap().damage_marked = 3;
    state.events.clear();
    state.trigger_event_index = 0;
    check_state_based_actions(&mut state, &reg);
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
    let nc = castable_spell(&mut state, &reg, "Nightbird's Clutches", P0);

    state = cast_and_resolve(&state, &reg, nc, vec![Target::Object(blocker)]);

    assert!(state.until_end_of_turn.iter().any(|e| matches!(e,
        mtg_engine::state::TemporaryEffect::CantBlock { target } if *target == blocker)));

    let eligible = combat::eligible_blockers(&state, P1, &reg);
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

    // Elder Cathar is a Human Soldier.
    let creature = named_permanent(&mut state, &reg, "Elder Cathar", P0);

    let bof = castable_spell(&mut state, &reg, "Bonds of Faith", P0);

    state = cast_and_resolve(&state, &reg, bof, vec![Target::Object(creature)]);
    triggers::process_triggers(&mut state, &reg);

    // Human should get +2/+2.
    assert_eq!(state.effective_power(creature, &reg), Some(4));
    assert_eq!(state.effective_toughness(creature, &reg), Some(4));
    assert!(state.can_attack(creature, &reg), "Human with Bonds should be able to attack");
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
        .is_some_and(|c| c.attackers.contains_key(&creature));
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

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
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
    let zombie = named_permanent(&mut state, &reg, "Walking Corpse", P0);

    // Blocker is Grave Bramble (protection from Zombies).
    let bramble = named_permanent(&mut state, &reg, "Grave Bramble", P1);

    submit_declare_attackers(&mut state, &[(zombie, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(bramble, zombie)], &reg);
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
        vec![CardType::Creature], vec![Keyword::Flying], &reg)[0];
    let non_token = ready_creature(&mut state, P0, 2, 2);

    // Cast Intangible Virtue.
    let iv = castable_spell(&mut state, &reg, "Intangible Virtue", P0);

    state = cast_and_resolve(&state, &reg, iv, vec![]);

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
    let _scarecrow = named_permanent(&mut state, &reg, "One-Eyed Scarecrow", P0);

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
        vec![CardType::Creature], vec![Keyword::Flying], &reg)[0];
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
    let ec = named_permanent(&mut state, &reg, "Elder Cathar", P0);

    // A Human ally (Doomed Traveler).
    let human = named_permanent(&mut state, &reg, "Doomed Traveler", P0);

    // Kill Elder Cathar.
    state.get_object_mut(ec).unwrap().damage_marked = 2;
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(human, CounterType::PlusOnePlusOne), 2,
        "Human should get 2 +1/+1 counters from Elder Cathar");
}

#[test]
fn elder_cathar_gives_one_counter_to_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ec = named_permanent(&mut state, &reg, "Elder Cathar", P0);

    // A non-Human ally (Grizzly Bears = Bear).
    let bears = ready_creature(&mut state, P0, 2, 2);

    state.get_object_mut(ec).unwrap().damage_marked = 2;
    check_state_based_actions(&mut state, &reg);
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

    let stalker = named_permanent(&mut state, &reg, "Invisible Stalker", P0);

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

    let vi = named_permanent(&mut state, &reg, "Vampire Interloper", P1);

    let eligible = combat::eligible_blockers(&state, P1, &reg);
    assert!(!eligible.contains(&vi),
        "Vampire Interloper should not be eligible to block");
}

// ══════════════════════════════════════════════════════════════════
// Claustrophobia — prevents untapping
// ══════════════════════════════════════════════════════════════════

/// Creature enchanted by Claustrophobia doesn't untap during untap step.
/// Claustrophobia: "enchanted creature doesn't untap during its controller's
/// untap step." Run a real untap step and check the enchanted creature against a
/// plain one on the same battlefield.
#[test]
fn claustrophobia_prevents_untap() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, PlayerId(1));

    let enchanted = state.create_object(CardId(99), PlayerId(1), Zone::Battlefield, Some(3), Some(3));
    let normal = state.create_object(CardId(98), PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    for id in [enchanted, normal] {
        let obj = state.get_object_mut(id).unwrap();
        obj.summoning_sick = false;
        obj.tapped = true;
    }

    let cl_id = reg.get_id_by_name("Claustrophobia").unwrap();
    let cl = state.create_object(cl_id, P0, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(cl).unwrap();
        obj.name = "Claustrophobia".into();
        obj.attached_to = Some(enchanted);
        obj.summoning_sick = false;
    }

    // Walk a full turn cycle round to P1's draw step, so P1's untap step has run.
    state.step = Step::Cleanup;
    state.active_player = P0;
    loop {
        engine::advance_step(&mut state, &reg);
        if state.active_player == PlayerId(1) && state.step == Step::Draw {
            break;
        }
    }

    assert!(state.get_object(enchanted).unwrap().tapped,
        "the enchanted creature does not untap");
    assert!(!state.get_object(normal).unwrap().tapped,
        "test precondition: the untap step did run — an unenchanted creature untapped");
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

    let fod = castable_spell(&mut state, &reg, "Feeling of Dread", P0);

    state = cast_and_resolve(&state, &reg, fod, vec![Target::Object(creature1), Target::Object(creature2)]);

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
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    state = cast_onto_stack(&state, &reg, bears, vec![]);

    // P1 casts Frightful Delusion. Give P0 mana so they CAN pay.
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1); // P0 has {1} to pay
    let fd = spell_in_hand(&mut state, &reg, "Frightful Delusion", P1);
    add_mana_for(&mut state, &reg, "Frightful Delusion", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, fd, vec![Target::Object(bears)]);

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
fn frightful_delusion_offers_the_choice_even_with_an_empty_pool() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    state = cast_onto_stack(&state, &reg, bears, vec![]);

    // P0 has NO mana (pool empty after casting).
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);

    let fd = spell_in_hand(&mut state, &reg, "Frightful Delusion", P1);
    add_mana_for(&mut state, &reg, "Frightful Delusion", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, fd, vec![Target::Object(bears)]);

    // CR 608.2g: the choice belongs to the spell's controller even with an
    // empty pool — they may tap for the mana. With no mana sources at all the
    // only answer available is "don't pay".
    let actions = engine::legal_actions(&state, &reg).actions;
    assert!(matches!(&state.awaiting_action,
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            player, choice: mtg_engine::state::ResolutionChoiceKind::PayOrNot { .. }, .. }) if *player == P0),
        "the spell's controller must be asked; got {:?}", state.awaiting_action);
    assert!(actions.len() == 1 && matches!(&actions[0], Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::PayDecision(false) }),
        "with no mana and nothing to tap, declining is the only legal answer; got {actions:?}");

    state = engine::submit_action(&state, &actions[0], &reg);
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Bears is countered once the payment is declined");
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

    // Cast Unburial Rites targeting the Tusker (target chosen at cast time).
    let ur = castable_spell(&mut state, &reg, "Unburial Rites", P0);

    state = cast_and_resolve(&state, &reg, ur, vec![Target::Object(tusker)]);

    // Tusker should be on the battlefield, Bears should stay in graveyard.
    assert_eq!(state.get_object(tusker).unwrap().zone, Zone::Battlefield,
        "Targeted creature should return to battlefield");
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Non-targeted creature should stay in graveyard");
}

/// Pitchburn Devils choice with multiple targets.
#[test]
fn pitchburn_devils_choice_with_targets() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Pitchburn Devils on P0's battlefield.
    let pd = named_permanent(&mut state, &reg, "Pitchburn Devils", P0);

    // P1 has a creature (so there are multiple damage targets).
    let blocker = ready_creature(&mut state, P1, 4, 4);

    // Kill Pitchburn Devils.
    state.events.clear();
    state.get_object_mut(pd).unwrap().damage_marked = 3;
    check_state_based_actions(&mut state, &reg);
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
    // Resolve the pending trigger on the stack to actually apply damage.
    triggers::process_triggers(&mut state, &reg);

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
    let fa = castable_spell(&mut state, &reg, "Forbidden Alchemy", P0);

    state = cast_and_resolve(&state, &reg, fa, vec![]);

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
/// Regeneration does NOT prevent death from 0 toughness.
/// Multiple regeneration shields stack — second lethal damage also regenerates.
/// Regeneration shields expire at end of turn (cleanup step).
/// `try_destroy` respects regeneration.
/// `try_destroy` without shields actually destroys.
/// Sacrifice bypasses regeneration shields.
/// Sacrifice sets `creature_died_this_turn` for morbid.
#[test]
fn sacrifice_triggers_morbid() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    assert!(!state.creature_died_this_turn);

    mtg_engine::destruction::sacrifice(&mut state, creature, &reg);

    assert!(state.creature_died_this_turn,
        "Sacrifice should set creature_died_this_turn for morbid");
}

/// Regeneration saves from deathtouch damage.
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
    let sg = castable_spell(&mut state, &reg, "Skeletal Grimace", P0);

    state = cast_and_resolve(&state, &reg, sg, vec![Target::Object(creature)]);

    // Creature should have +1/+1 from the aura.
    assert_eq!(state.effective_power(creature, &reg), Some(3));
    assert_eq!(state.effective_toughness(creature, &reg), Some(3));

    // Activate the regenerate ability: pay {B}, get a shield.
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(activate.is_some(), "Should be able to activate {{B}}: Regenerate");

    state = resolve_activated(engine::submit_action(&state, activate.unwrap(), &reg), &reg);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1,
        "Activating should add a regeneration shield");
}

/// Skeletal Grimace regeneration saves creature from lethal damage.
#[test]
fn skeletal_grimace_regeneration_saves_from_lethal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let sg = castable_spell(&mut state, &reg, "Skeletal Grimace", P0);

    state = cast_and_resolve(&state, &reg, sg, vec![Target::Object(creature)]);

    // Activate regenerate.
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = resolve_activated(engine::submit_action(&state, &activate, &reg), &reg);

    // Verify shield is active before dealing damage.
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1,
        "Shield should be active before damage");
    assert_eq!(state.effective_toughness(creature, &reg), Some(3),
        "Effective toughness should be 3 with aura");

    // Now deal lethal damage (effective toughness is 3 with the aura).
    state.get_object_mut(creature).unwrap().damage_marked = 3;
    check_state_based_actions(&mut state, &reg);

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
    let sg = castable_spell(&mut state, &reg, "Skeletal Grimace", P0);

    // Cast and resolve Skeletal Grimace.
    state = cast_and_resolve(&state, &reg, sg, vec![Target::Object(creature)]);

    // Activate regenerate.
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = resolve_activated(engine::submit_action(&state, &activate, &reg), &reg);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1);

    // P1 casts Doom Blade targeting the creature.
    state.priority_player = Some(P1);
    let db = castable_spell(&mut state, &reg, "Doom Blade", P1);

    state = cast_and_resolve(&state, &reg, db, vec![Target::Object(creature)]);

    // Creature should survive via regeneration.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Regeneration should save from Doom Blade");
    assert!(state.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "Shield should be consumed");
}

/// Skeletal Grimace regeneration saves creature from deathtouch damage.
#[test]
fn skeletal_grimace_regeneration_vs_deathtouch() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0's creature with Skeletal Grimace attached.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().name = "Runeclaw Bear".into();
    let sg = castable_spell(&mut state, &reg, "Skeletal Grimace", P0);

    // Cast and resolve Skeletal Grimace.
    state = cast_and_resolve(&state, &reg, sg, vec![Target::Object(creature)]);

    // Activate regenerate.
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = resolve_activated(engine::submit_action(&state, &activate, &reg), &reg);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1);

    // Simulate deathtouch damage (even 1 damage from deathtouch is lethal).
    state.get_object_mut(creature).unwrap().damage_marked = 1;
    state.get_object_mut(creature).unwrap().dealt_deathtouch_damage = true;
    check_state_based_actions(&mut state, &reg);

    // Creature should survive via regeneration.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Regeneration should save from deathtouch damage");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
        "Damage should be removed after regeneration");
    assert!(!state.get_object(creature).unwrap().dealt_deathtouch_damage,
        "Deathtouch flag should be cleared after regeneration");
    assert!(state.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "Shield should be consumed");
}

// -------------------------------------------------------------------------
// What each card offers to point at
// -------------------------------------------------------------------------

/// "When Fiend Hunter enters the battlefield, you may exile another target
/// creature." Another — not another *opponent's*: your own creature is a legal
/// choice, and a real one (exiling it dodges a sweeper, and it comes back when
/// the Hunter leaves).
///
/// Both of the tests this replaces asserted only that *a* choice was presented,
/// which says nothing about who is in it — an engine auto-exiling the
/// opponent's biggest creature and then asking an unrelated question passes
/// that.
#[test]
fn fiend_hunter_offers_every_creature_but_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let own = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = ready_creature(&mut state, P1, 3, 3);
    let theirs_too = ready_creature(&mut state, P1, 2, 2);

    let hunter = castable_spell(&mut state, &reg, "Fiend Hunter", P0);
    let mut state = cast_and_resolve(&state, &reg, hunter, vec![]);
    triggers::process_triggers(&mut state, &reg);

    let options = pending_choice_options(&state);
    for (id, who) in [(own, "your own creature"), (theirs, "an opponent's"), (theirs_too, "and the other")] {
        assert!(options.contains(&Target::Object(id)),
            "{who} is a legal choice for 'another target creature'; offered {options:?}");
    }
    assert!(!options.contains(&Target::Object(hunter)),
        "'another' excludes the Fiend Hunter itself");

    // Nothing has been exiled while the choice is still pending.
    for id in [own, theirs, theirs_too] {
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
    }
}

/// Morkrut Banshee should be able to target itself with -4/-4.
#[test]
fn morkrut_banshee_can_target_self() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true; // enable morbid

    let banshee = castable_spell(&mut state, &reg, "Morkrut Banshee", P0);
    let mut state = cast_and_resolve(&state, &reg, banshee, vec![]);
    triggers::process_triggers(&mut state, &reg);

    // With only Morkrut Banshee on the battlefield, it is the only legal
    // target for its own morbid ETB, so it must target itself. Either it is
    // still there at 4-4=0 toughness on its way to dying, or SBA has already
    // taken it — both mean the -4/-4 landed; nothing on the battlefield at
    // full toughness would.
    let obj = state.objects.values()
        .find(|o| o.name == "Morkrut Banshee")
        .expect("the Banshee is somewhere");
    match obj.zone {
        // Still on the battlefield, on its way out: 4 base toughness less 4.
        Zone::Battlefield => assert_eq!(state.effective_toughness(obj.id, &reg), Some(0),
            "the -4/-4 landed on the only legal target, which is itself"),
        // Or state-based actions already took it, which means the same thing.
        Zone::Graveyard => {}
        other => panic!("the Banshee should be on the battlefield at 0 toughness or \
                         already in the graveyard, not in {other:?}"),
    }
}

/// Frightful Delusion: opponent should discard even when they pay {1}.
#[test]
fn frightful_delusion_discard_on_pay() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 casts a creature.
    let creature = castable_spell(&mut state, &reg, "Grizzly Bears", P1);
    state.priority_player = Some(P1);
    state = cast_onto_stack(&state, &reg, creature, vec![]);

    // P0 casts Frightful Delusion targeting the creature spell.
    state.priority_player = Some(P0);
    let fd = castable_spell(&mut state, &reg, "Frightful Delusion", P0);
    state = cast_onto_stack(&state, &reg, fd, vec![Target::Object(creature)]);

    // Give P1 mana to pay {1} and a card in hand to discard.
    state.get_player_mut(P1).mana_pool.add(ManaType::Colorless, 1);
    let _discard_card = state.create_object(CardId(9999), P1, Zone::Hand, None, None);

    // Resolve Frightful Delusion. P1 should get a pay-or-not choice.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // P1 pays {1} to keep their spell. Asserted rather than tested for: with the
    // payment inside an `if`, a Frightful Delusion that stopped asking would
    // never pay and the discard below would be measuring the wrong scenario.
    assert!(state.awaiting_action.is_some(),
        "CR 608.2g: the spell's controller is asked whether to pay {{1}}");
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::PayDecision(true),
        },
        &reg,
    );

    // After paying, P1 should STILL have to discard a card.
    // Oracle: "Counter target spell unless its controller pays {1}. That player discards a card."
    // The discard is a separate effect that always happens.
    let hand_count = state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_count, 0,
        "Frightful Delusion: opponent should discard even after paying mana. \
         Hand has {hand_count} cards (should be 0)");
}

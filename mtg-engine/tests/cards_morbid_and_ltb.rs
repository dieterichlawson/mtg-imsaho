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
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
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

/// "Morbid — When this creature enters, if a creature died this turn, you gain
/// 5 life."
///
/// The card's whole effect, which nothing tested: its only coverage was the
/// registry-wide intervening-if sweep, and that asserts the trigger goes on
/// the stack, not that anything happens when it resolves. Both arms, and the
/// opponent's life as a control.
#[test]
fn hollowhenge_scavenger_gains_five_life_only_if_a_creature_died() {
    let reg = registry();
    for died in [true, false] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.creature_died_this_turn = died;
        let before = state.get_player(P0).life;
        let their_life = state.get_player(P1).life;

        let scavenger = castable_spell(&mut state, &reg, "Hollowhenge Scavenger", P0);
        let mut state = cast_and_resolve(&state, &reg, scavenger, vec![]);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(state.get_object(scavenger).unwrap().zone, Zone::Battlefield,
            "the Scavenger arrives whether or not morbid is on");
        assert_eq!(state.get_player(P0).life, if died { before + 5 } else { before },
            "creature_died_this_turn = {died}");
        assert_eq!(state.get_player(P1).life, their_life,
            "'you gain 5 life' — the opponent gains nothing either way");
    }
}

/// CR 608.2g: "you" is the Scavenger's last known controller. A permanent that
/// has left the battlefield has its `controller` reset to its owner (CR 400.7),
/// so a Scavenger killed in response to its own morbid trigger is the case
/// where reading the field and reading last-known information differ.
///
/// CR 113.7a: killing it does not counter the trigger — the life is still
/// gained.
#[test]
fn hollowhenge_scavengers_life_goes_to_its_last_controller() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let scavenger = named_permanent(&mut state, &reg, "Hollowhenge Scavenger", P0);
    // Owned by the opponent, controlled by P0.
    state.get_object_mut(scavenger).unwrap().owner = P1;
    let card_id = state.get_object(scavenger).unwrap().card_id;

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(scavenger, card_id, P0, "Hollowhenge Scavenger"),
        event: TriggerEvent::SelfEntered,
    }));
    // Killed with its own trigger already on the stack.
    state.move_object(scavenger, Zone::Graveyard, &reg);

    let mine = state.get_player(P0).life;
    let theirs = state.get_player(P1).life;
    triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, mine + 5,
        "the life goes to the player who controlled it, not the one who owns it");
    assert_eq!(state.get_player(P1).life, theirs,
        "and the owner gains nothing");
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

/// Morbid is a condition in the effect, so it is read when the spell
/// **resolves** (CR 608.2) — not when it is cast. Brimstone Volley is an
/// instant, so there is a window: cast it with nothing dead, kill something in
/// response, and it deals 5.
///
/// This also runs the flag through a real death rather than setting the bool,
/// which the two tests above do.
#[test]
fn brimstone_volley_reads_morbid_when_it_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    assert!(!state.creature_died_this_turn, "test setup: nothing has died yet");

    let victim = ready_creature(&mut state, P1, 1, 1);
    let bv = castable_spell(&mut state, &reg, "Brimstone Volley", P0);
    let mut state = cast_onto_stack(&state, &reg, bv, vec![Target::Player(P1)]);

    // In response, a creature dies.
    kill_by_damage(&mut state, &reg, victim);
    assert!(state.creature_died_this_turn, "a real death sets the flag");

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 15,
        "morbid was false when the spell was cast and true when it resolved, \
         so it deals 5");
}

/// "a creature died **this turn**" — a death on the previous turn does not
/// carry over.
#[test]
fn brimstone_volley_forgets_a_death_from_the_previous_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = ready_creature(&mut state, P1, 1, 1);
    kill_by_damage(&mut state, &reg, victim);
    assert!(state.creature_died_this_turn, "test setup");

    stock_library(&mut state, &reg, P0, 5);
    stock_library(&mut state, &reg, P1, 5);
    advance_to_next_turn(&mut state, &reg);
    assert!(!state.creature_died_this_turn, "the turn ended and took the flag with it");

    // It is the opponent's turn now; an instant is castable with priority.
    state.priority_player = Some(P0);
    let bv = castable_spell(&mut state, &reg, "Brimstone Volley", P0);
    let life_before = state.get_player(P1).life;
    let state = cast_and_resolve(&state, &reg, bv, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P1).life, life_before - 3,
        "3 damage, not 5");
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

/// Answer every pending resolution choice by picking `pick`, running triggers
/// between each. Fiend Hunter can raise two in a row — the trigger's target
/// (CR 603.3d) and then its "you may".
fn answer_choices_with(
    mut state: mtg_engine::state::GameState,
    reg: &CardRegistry,
    pick: mtg_engine::ids::ObjectId,
) -> mtg_engine::state::GameState {
    for _ in 0..4 {
        if state.awaiting_action.is_none() { break; }
        state = engine::submit_action(&state, &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(pick))),
        }, reg);
        triggers::process_triggers(&mut state, reg);
    }
    state
}

/// Ruling: "If a token is exiled this way, it won't return to the
/// battlefield."
///
/// The Hunter says "return the exiled **card**", and a token is not a card
/// (CR 111.1). It never gets that far here — CR 704.5d makes a token that is
/// not on the battlefield cease to exist, so by the time the Hunter leaves
/// there is nothing left to look for.
#[test]
fn fiend_hunter_does_not_return_an_exiled_token() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let token = state.create_token_with_subtypes("", P1, 2, 2, vec![Color::Black],
        vec![CardType::Creature], vec![], vec!["Zombie".into()], &reg)[0];
    let hunter = named_permanent(&mut state, &reg, "Fiend Hunter", P0);

    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: hunter, controller: P0 });
    triggers::process_triggers(&mut state, &reg);
    let mut state = answer_choices_with(state, &reg, token);
    assert_eq!(state.get_object(token).map(|o| o.zone), Some(Zone::Exile),
        "test precondition: the token was exiled");

    check_state_based_actions(&mut state, &reg);
    assert!(state.get_object(token).is_none(),
        "CR 704.5d: a token that is not on the battlefield ceases to exist");

    // Now the Hunter leaves.
    state.get_object_mut(hunter).unwrap().damage_marked = 99;
    state.events.clear();
    state.trigger_event_index = 0;
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(hunter).unwrap().zone, Zone::Graveyard,
        "test precondition: the Hunter left the battlefield");
    assert!(state.get_object(token).is_none(),
        "the token does not come back — the Hunter returns a card, and there \
         is no longer anything there at all");
}

/// Ruling: "If Fiend Hunter leaves the battlefield before its first ability
/// has resolved, its second ability will trigger and do nothing. Then its
/// first ability will resolve and exile the target creature indefinitely."
///
/// The order is the whole point: the leave trigger goes on the stack above the
/// enters trigger, so it resolves first, when nothing has been exiled yet.
#[test]
fn fiend_hunter_killed_in_response_exiles_the_creature_for_good() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = ready_creature(&mut state, P1, 3, 3);
    let hunter = named_permanent(&mut state, &reg, "Fiend Hunter", P0);

    // The enters trigger goes on the stack, target locked (CR 603.3d).
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: hunter, controller: P0 });
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(state.stack.len(), 1, "test precondition: the enters trigger is on the stack");

    // In response, the Hunter is destroyed. Its leave trigger goes on top.
    state.get_object_mut(hunter).unwrap().damage_marked = 99;
    check_state_based_actions(&mut state, &reg);
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(state.stack.len(), 2,
        "the leave trigger fires and goes above the enters trigger");

    triggers::process_triggers(&mut state, &reg);
    let mut state = answer_choices_with(state, &reg, victim);
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Exile,
        "the leave trigger resolved first and found nothing exiled; the enters \
         trigger then exiled the creature with nothing left to return it");
}

/// Ruling: "Once the exiled creature returns, it's considered a new object
/// with no relation to the object that it was. Auras attached to the exiled
/// creature will be put into their owners' graveyards. Equipment attached to
/// the exiled creature will become unattached and remain on the battlefield.
/// Any counters on the exiled creature will cease to exist."
#[test]
fn fiend_hunter_returns_a_new_object() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    state.add_counters(victim, CounterType::PlusOnePlusOne, 2);
    let aura = named_permanent(&mut state, &reg, "Dead Weight", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(victim);
    let gear = named_permanent(&mut state, &reg, "Butcher's Cleaver", P1);
    state.get_object_mut(gear).unwrap().attached_to = Some(victim);

    let hunter = named_permanent(&mut state, &reg, "Fiend Hunter", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: hunter, controller: P0 });
    triggers::process_triggers(&mut state, &reg);
    let mut state = answer_choices_with(state, &reg, victim);
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(aura).unwrap().zone, Zone::Graveyard,
        "CR 704.5m: the Aura has nothing to enchant and goes to its owner's graveyard");
    assert_eq!(state.get_object(gear).unwrap().zone, Zone::Battlefield,
        "CR 704.5n: the Equipment stays on the battlefield");
    assert_eq!(state.get_object(gear).unwrap().attached_to, None,
        "and becomes unattached");

    // The Hunter leaves; the creature comes back as a new object.
    state.get_object_mut(hunter).unwrap().damage_marked = 99;
    state.events.clear();
    state.trigger_event_index = 0;
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
        "test precondition: it returned");
    assert_eq!(counters_of(&state, victim, CounterType::PlusOnePlusOne), 0,
        "its counters ceased to exist — what returned is a new object");
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

/// Attach the Aura and declare no attackers; say whether the creature was
/// dragged into combat anyway.
fn forced_into_combat_by_furor(
    state: &mut mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    creature: ObjectId,
) -> bool {
    let furor_id = reg.get_id_by_name("Furor of the Bitten").unwrap();
    let furor = state.create_object(furor_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(furor).unwrap().name = "Furor of the Bitten".into();
    state.get_object_mut(furor).unwrap().attached_to = Some(creature);

    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);
    *state = engine::submit_action(state, &Action::DeclareAttackers { attackers: vec![] }, reg);
    state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&creature))
}

/// "attacks each combat **if able**", and haste is what makes a creature that
/// arrived this turn able (CR 302.6). The Aura route to the same rule the
/// Curses reach through a global effect.
#[test]
fn furor_forces_a_hasty_creature_the_turn_it_arrives() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let skeleton = named_permanent(&mut state, &reg, "Manor Skeleton", P0);
    state.get_object_mut(skeleton).unwrap().summoning_sick = true;

    assert!(forced_into_combat_by_furor(&mut state, &reg, skeleton),
        "haste makes it able, so the Aura makes it attack");
}

/// Ruling: "If the enchanted creature can't attack for any reason (such as
/// being tapped or **having come under that player's control that turn**),
/// then it doesn't attack."
///
/// `change_control` sets summoning sickness for the new controller, which is
/// what that clause is; a stolen creature with no haste stays home however
/// furious it is.
#[test]
fn furor_does_not_force_a_creature_that_just_changed_hands() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let stolen = ready_creature(&mut state, P1, 2, 2);
    state.change_control(stolen, P0);

    assert!(!forced_into_combat_by_furor(&mut state, &reg, stolen),
        "it came under its new controller's control this turn and has no haste");
}

/// Both halves are the Aura's, so both end when it does.
#[test]
fn furors_buff_and_compulsion_end_with_the_aura() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let furor = castable_spell(&mut state, &reg, "Furor of the Bitten", P0);
    let mut state = cast_and_resolve(&state, &reg, furor, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(4), "test setup: +2/+2");
    assert!(state.must_attack(creature, &reg), "test setup: and compelled");

    state.move_object(furor, Zone::Graveyard, &reg);

    assert_eq!(state.effective_power(creature, &reg), Some(2), "the buff goes");
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
    assert!(!state.must_attack(creature, &reg), "and so does the compulsion");
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

/// "dealt **by** enchanted creature" includes damage to a player. The test
/// above is creature-against-creature; the damage a creature deals to a player
/// is a different path in the damage pipeline, and this is the one that
/// matters most for the card — an enchanted attacker gets through and does
/// nothing.
#[test]
fn ghostly_possession_prevents_the_creatures_damage_to_a_player() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let gp_id = reg.get_id_by_name("Ghostly Possession").unwrap();
    let gp = state.create_object(gp_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(gp).unwrap().name = "Ghostly Possession".into();
    state.get_object_mut(gp).unwrap().attached_to = Some(attacker);

    let before = state.get_player(P1).life;
    attacks_unblocked(&mut state, attacker, P1);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, before,
        "an unblocked enchanted attacker deals no combat damage to the player");
}

/// "Prevent all **combat** damage". Noncombat damage is not combat damage and
/// is not prevented — the word is doing work, and nothing tested it.
#[test]
fn ghostly_possession_does_not_prevent_noncombat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    let source = ready_creature(&mut state, P0, 1, 1);
    let gp_id = reg.get_id_by_name("Ghostly Possession").unwrap();
    let gp = state.create_object(gp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(gp).unwrap().name = "Ghostly Possession".into();
    state.get_object_mut(gp).unwrap().attached_to = Some(creature);

    mtg_engine::damage::deal_damage(&mut state, source,
        mtg_engine::events::DamageTarget::Object(creature), 2,
        mtg_engine::damage::DamageKind::NonCombat, &reg);

    assert_eq!(state.get_object(creature).unwrap().damage_marked, 2,
        "a Geistflame or a fight still marks its damage");
}

/// The prevention is the Aura's, so it goes when the Aura does — a continuous
/// effect lasts only while its source is on the battlefield.
#[test]
fn ghostly_possessions_prevention_ends_with_the_aura() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 5);
    let gp_id = reg.get_id_by_name("Ghostly Possession").unwrap();
    let gp = state.create_object(gp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(gp).unwrap().name = "Ghostly Possession".into();
    state.get_object_mut(gp).unwrap().attached_to = Some(blocker);

    state.move_object(gp, Zone::Graveyard, &reg);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 3,
        "with the Aura gone, the damage lands");
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

/// "put a +1/+1 counter on target **creature you control**."
///
/// The Cathar is in the graveyard by the time its own death trigger goes on
/// the stack, and leaving the battlefield resets `controller` to `owner`
/// (CR 400.7). So a Cathar whose owner and controller agree cannot tell a
/// correct read of the *last known* controller (CR 608.2g) from a read of the
/// reset field — both give the same answer. Here they disagree.
#[test]
fn elder_cathar_counters_a_creature_its_last_controller_controlled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ec = named_permanent(&mut state, &reg, "Elder Cathar", P0);
    state.get_object_mut(ec).unwrap().owner = P1;

    let mine = ready_creature(&mut state, P0, 2, 2);
    let theirs = ready_creature(&mut state, P1, 2, 2);

    state.get_object_mut(ec).unwrap().damage_marked = 2;
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(mine, CounterType::PlusOnePlusOne), 1,
        "'you' is the Cathar's last known controller, not its owner");
    assert_eq!(state.get_counter_count(theirs, CounterType::PlusOnePlusOne), 0,
        "'target creature you control' never reaches an opponent's creature");
}

/// "If that creature **is** a Human, put two +1/+1 counters on it instead."
///
/// That is read as the ability resolves, not when its target was chosen — CR
/// 603.3d locks the target in, not the target's characteristics. The existing
/// werewolf test transforms before the trigger exists, which a check made at
/// either moment would pass; this one transforms in between, where only a
/// resolution-time check gives the right answer.
#[test]
fn elder_cathars_human_check_is_made_on_resolution() {
    for (transformed_in_response, expected) in [(false, 2), (true, 1)] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let ec = named_permanent(&mut state, &reg, "Elder Cathar", P0);
        let card_id = state.get_object(ec).unwrap().card_id;
        // Tormented Pariah's front face is a Human Werewolf; its back face,
        // Rampaging Werewolf, is not a Human.
        let pariah = named_permanent(&mut state, &reg, "Tormented Pariah", P0);
        assert!(state.has_subtype(pariah, "Human", &reg),
            "test precondition: the front face is a Human");

        // The target is locked in while the Pariah is still a Human.
        state.stack.push(StackEntry::Trigger(PendingTrigger {
            source: TriggerSource {
                chosen_targets: vec![Target::Object(pariah)],
                ..TriggerSource::new(ec, card_id, P0, "Elder Cathar")
            },
            event: TriggerEvent::SelfDies,
        }));
        state.move_object(ec, Zone::Graveyard, &reg);

        if transformed_in_response {
            mtg_engine::cards::helpers::apply_transform(&mut state, pariah, &reg);
        }
        triggers::resolve_next_trigger(&mut state, &reg);

        assert_eq!(state.get_counter_count(pariah, CounterType::PlusOnePlusOne), expected,
            "transformed in response = {transformed_in_response}: the Human \
             check reads the face that is live when the ability resolves");
    }
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

    // Its other line — flying — asked of the game, not the card data.
    assert!(state.has_keyword(vi, Keyword::Flying, &reg),
        "Vampire Interloper flies");
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

/// Scryfall ruling (2011-09-22): "You must target a spell in order to cast
/// Frightful Delusion. You can't cast it without a legal target just to make a
/// player discard a card."
///
/// The discard reads like a second, independent sentence — and it is, once the
/// spell resolves — but it is not a reason to cast the card with nothing on
/// the stack.
#[test]
fn frightful_delusion_cannot_be_cast_just_for_the_discard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 holds a card, so there would be something to discard.
    state.create_object(CardId(9999), P1, Zone::Hand, None, None);
    let fd = castable_spell(&mut state, &reg, "Frightful Delusion", P0);

    let can_cast = |state: &mtg_engine::state::GameState| {
        engine::legal_actions(state, &reg).actions.iter()
            .any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == fd))
    };
    assert!(!can_cast(&state),
        "nothing is on the stack, so there is no legal target and the card \
         cannot be cast at all");

    // The control: a spell on the stack makes it castable, so the assertion
    // above is about the missing target and not about mana or timing.
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P1);
    state.priority_player = Some(P1);
    let mut state = cast_onto_stack(&state, &reg, bears, vec![]);
    state.priority_player = Some(P0);
    add_mana_for(&mut state, &reg, "Frightful Delusion", P0);
    assert!(can_cast(&state), "with a spell on the stack it can be cast");
}

/// "That player discards a card." With more than one card in hand that is the
/// player's choice, not the engine's pick.
///
/// Both existing discard tests hand the player exactly one card, which takes
/// the branch that discards without asking. The branch that asks was
/// uncovered, and it is the one that moved out of the engine and into the card
/// in this audit.
#[test]
fn frightful_delusion_lets_the_player_choose_which_card_to_discard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);
    let mut state = cast_onto_stack(&state, &reg, bears, vec![]);
    let held: Vec<ObjectId> = (0..2).map(|i| {
        let id = state.create_object(CardId(9999), P0, Zone::Hand, None, None);
        state.get_object_mut(id).unwrap().name = format!("Held {i}");
        id
    }).collect();

    let fd = spell_in_hand(&mut state, &reg, "Frightful Delusion", P1);
    add_mana_for(&mut state, &reg, "Frightful Delusion", P1);
    state.priority_player = Some(P1);
    let state = cast_and_resolve(&state, &reg, fd, vec![Target::Object(bears)]);
    let state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::PayDecision(false) }, &reg);

    // The counter has happened; the discard is a pending choice for P0.
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "the payment was declined, so the spell is countered");
    assert!(matches!(&state.awaiting_action,
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            player,
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseCardFromHand { .. }, .. })
        if *player == P0),
        "with two cards in hand the discard is P0's choice; got {:?}", state.awaiting_action);
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2,
        "and nothing has been discarded while it is pending");

    let state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::ChosenCard(held[1]) }, &reg);
    assert_eq!(state.get_object(held[1]).unwrap().zone, Zone::Graveyard,
        "the card the player picked is the one discarded");
    assert_eq!(state.get_object(held[0]).unwrap().zone, Zone::Hand,
        "and the other is not");
}

/// An empty hand discards nothing and asks nothing. "That player discards a
/// card" does as much as it can (CR 608.2) — with no cards, that is nothing,
/// and it must not leave a prompt hanging.
#[test]
fn frightful_delusion_asks_nothing_of_a_player_with_an_empty_hand() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);
    let mut state = cast_onto_stack(&state, &reg, bears, vec![]);
    assert!(state.objects_in_zone(Zone::Hand, P0).is_empty(),
        "test precondition: P0 is holding nothing");

    let fd = spell_in_hand(&mut state, &reg, "Frightful Delusion", P1);
    add_mana_for(&mut state, &reg, "Frightful Delusion", P1);
    state.priority_player = Some(P1);
    let state = cast_and_resolve(&state, &reg, fd, vec![Target::Object(bears)]);
    let state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::PayDecision(false) }, &reg);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "the counter still happens");
    assert!(state.awaiting_action.is_none(),
        "and nothing is left pending; got {:?}", state.awaiting_action);
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

/// "{B}: Regenerate this creature" resolving after the creature has already
/// been destroyed does nothing — and, in particular, leaves nothing behind for
/// a later reanimation to pick up. CR 113.7a keeps the ability on the stack;
/// CR 400.7 makes what comes back a different object.
#[test]
fn skeletal_grimaces_regeneration_leaves_nothing_on_a_dead_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let sg = castable_spell(&mut state, &reg, "Skeletal Grimace", P0);
    let mut state = cast_and_resolve(&state, &reg, sg, vec![Target::Object(creature)]);
    assert_eq!(state.get_object(sg).unwrap().attached_to, Some(creature), "test setup");

    // Destroyed with its own regenerate ability still to resolve.
    mtg_engine::destruction::try_destroy(&mut state, creature, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);

    let card_id = state.get_object(sg).unwrap().card_id;
    reg.get(card_id).unwrap()
        .resolve_activated_ability(&mut state, creature, 0, &[], &reg);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "the shield has nothing to attach to");

    // A turn later, something brings it back.
    advance_to_next_turn(&mut state, &reg);
    state.move_object(creature, Zone::Battlefield, &reg);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "and no free regeneration comes back with it");
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

/// Ruling 2020-08-07: "Morkrut Banshee's morbid ability triggers only once,
/// not once for each creature that has died this turn."
///
/// Morbid is an intervening-if on an enters trigger, so the number of deaths
/// is a yes/no question and never a count.
#[test]
fn morkrut_banshees_morbid_triggers_once_however_many_creatures_died() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Three creatures die this turn.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        mtg_engine::destruction::try_destroy(&mut state, c, &reg);
    }
    assert!(state.creature_died_this_turn, "test setup");

    // The Banshee is the only creature left on the battlefield, so its trigger
    // has exactly one legal target and dispatch pushes it instead of stopping
    // to prompt — a prompt would leave the stack empty for a reason that has
    // nothing to do with how many times it triggered.
    let banshee = named_permanent(&mut state, &reg, "Morkrut Banshee", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: banshee, controller: P0,
    });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);
    mtg_engine::triggers::process_pending_trigger_pushes(&mut state, &reg);

    let banshee_triggers = state.stack.iter().filter(|e| matches!(e,
        mtg_engine::state::StackEntry::Trigger(t) if t.source_object() == banshee)).count();
    assert_eq!(banshee_triggers, 1,
        "one enters trigger, whatever the body count");
}

/// "...gets -4/-4 **until end of turn**." A creature that survives it is back
/// to its printed size next turn (CR 514.2).
#[test]
fn morkrut_banshees_minus_four_wears_off() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let victim = ready_creature(&mut state, P1, 6, 6);
    let banshee = named_permanent(&mut state, &reg, "Morkrut Banshee", P0);
    let card_id = state.get_object(banshee).unwrap().card_id;
    reg.get(card_id).unwrap()
        .on_enter_battlefield(&mut state, banshee, &[Target::Object(victim)], &reg);

    assert_eq!(
        (state.effective_power(victim, &reg), state.effective_toughness(victim, &reg)),
        (Some(2), Some(2)),
        "6/6 less 4/4");

    advance_to_next_turn(&mut state, &reg);
    assert_eq!(
        (state.effective_power(victim, &reg), state.effective_toughness(victim, &reg)),
        (Some(6), Some(6)),
        "the debuff ended at the cleanup step");
}

/// CR 113.7a: the enters trigger is on the stack independently of the Banshee,
/// so removal in response does not save the target. The handler already ignores
/// its own source — nothing in "target creature gets -4/-4 until end of turn"
/// is about the Banshee — and this is what says so.
#[test]
fn morkrut_banshees_debuff_lands_after_the_banshee_is_killed_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let victim = ready_creature(&mut state, P1, 6, 6);
    let banshee = named_permanent(&mut state, &reg, "Morkrut Banshee", P0);
    let card_id = state.get_object(banshee).unwrap().card_id;

    state.move_object(banshee, Zone::Graveyard, &reg);
    reg.get(card_id).unwrap()
        .on_enter_battlefield(&mut state, banshee, &[Target::Object(victim)], &reg);

    assert_eq!(state.effective_toughness(victim, &reg), Some(2),
        "the trigger resolves without its source");
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

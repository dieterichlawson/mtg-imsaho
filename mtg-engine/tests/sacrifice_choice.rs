//! Regression tests for the `SacrificeCost` auto-pick bug (audit finding).
//!
//! Background: in earlier versions, when an activated ability with
//! `SacrificeCost::SacrificeCreature` resolved, the engine auto-picked the
//! first creature in the zone as the sacrifice. For Demonmail Hauberk's
//! "Equip — Sacrifice a creature" ability that meant the engine could (and
//! did, in the audit log) sacrifice the very creature being targeted by the
//! equip — fizzling the ability. The player lost a creature for nothing.
//!
//! Same bug applied to Disciple of Griselbrand and Skirsdag Cultist (both use
//! `SacrificeCreature`). The auto-pick also gave Disciple a non-optimal sacrifice
//! since the player couldn't pick the highest-toughness creature.
//!
//! The fix: `legal_actions` enumerates one `Action::ActivateAbility` per
//! (target, sacrifice) combo, mirroring how `CastSpell` handles spell-side
//! sacrifice costs. The apply path uses the explicit sacrifice rather than
//! auto-picking.
//!
//! A second filter was layered on top of that fix: pairs where sacrifice ==
//! target were hidden, so the player "could never accidentally pick a fizzling
//! combo". That went too far. CR 601.2b chooses targets before CR 601.2h pays
//! costs, so sacrificing the creature you targeted is a legal activation — the
//! sacrifice happens and only the ability is countered on resolution (CR
//! 608.2b). Sometimes the sacrifice is the whole point: Demonmail Hauberk's
//! "Equip—Sacrifice a creature" is a free sorcery-speed sac outlet, and with a
//! single creature on the battlefield the hidden pair was the only way to use
//! it. Hiding a legal play is not the engine's call.
//!
//! These tests pin the real property — the player picks the sacrifice, and
//! every legal pair is offered.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::ids::ObjectId;
use mtg_engine::types::*;

// ════════════════════════════════════════════════════════════════════
// Demonmail Hauberk — Equip — Sacrifice a creature
// ════════════════════════════════════════════════════════════════════

#[test]
fn hauberk_legal_actions_enumerate_target_sacrifice_combos() {
    // 3 creatures + Hauberk on board. One ActivateAbility per (target,
    // sacrifice) pair — 3 targets × 3 sacrifices = 9 combos, including the
    // three where the sacrifice is the target.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = named_permanent(&mut state, &reg, "Demonmail Hauberk", P0);
    let a = ready_creature(&mut state, P0, 1, 1);
    let b = ready_creature(&mut state, P0, 2, 2);
    let c = ready_creature(&mut state, P0, 3, 3);

    let legal = engine::legal_actions(&state, &reg);
    let combos: Vec<(ObjectId, ObjectId)> = legal.actions.iter().filter_map(|act| {
        if let Action::ActivateAbility { object_id, targets, sacrifice, .. } = act {
            if *object_id != hauberk { return None; }
            let target_id = match targets.first()? {
                Target::Object(id) => *id,
                Target::Player(_) => return None,
                Target::Illegal => unreachable!("Target::Illegal is substituted at resolution and never offered to a player"),
            };
            let sac_id = (*sacrifice)?;
            Some((target_id, sac_id))
        } else { None }
    }).collect();

    assert_eq!(combos.len(), 9, "should enumerate 3 targets × 3 sacrifices = 9 combos, got {combos:?}");

    // Every pair, the self-sacrificing ones included.
    for &t in &[a, b, c] {
        for &sac in &[a, b, c] {
            assert!(combos.contains(&(t, sac)), "missing combo (target={}, sac={})", t.0, sac.0);
        }
    }
}

#[test]
fn hauberk_explicit_sacrifice_attaches_correctly() {
    // The audit log scenario: hauberk + 3 creatures, model wants to equip
    // creature_b and sacrifice creature_a. With the fix, the engine respects
    // the player's explicit choice and the equip succeeds.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = named_permanent(&mut state, &reg, "Demonmail Hauberk", P0);
    let creature_a = ready_creature(&mut state, P0, 1, 1);
    let creature_b = ready_creature(&mut state, P0, 2, 2);

    let action = Action::ActivateAbility {
        object_id: hauberk,
        ability_index: 0,
        targets: vec![Target::Object(creature_b)],
        tap_plan: vec![],
        sacrifice: Some(creature_a),
        x_value: None,
        source_card_id: None,
    };
    let new_state = resolve_activated(engine::submit_action(&state, &action, &reg), &reg);

    // creature_a sacrificed, creature_b equipped, hauberk attached.
    assert_eq!(new_state.get_object(creature_a).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_object(creature_b).unwrap().zone, Zone::Battlefield);
    assert_eq!(new_state.get_object(hauberk).unwrap().attached_to, Some(creature_b));
    // Hauberk gives +4/+2 → creature_b should be 2+4 = 6 power, 2+2 = 4 toughness.
    assert_eq!(new_state.effective_power(creature_b, &reg), Some(6));
    assert_eq!(new_state.effective_toughness(creature_b, &reg), Some(4));
}

#[test]
fn hauberk_with_one_creature_is_a_free_sacrifice_outlet() {
    // Hauberk + exactly one creature. The only target is also the only
    // creature that can pay, so the activation must sacrifice the creature it
    // targeted. That is legal (CR 601.2b before 601.2h) and it is the reason
    // to play this card next to a Doomed Traveler: the equip fizzles, the
    // sacrifice is what you wanted.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = named_permanent(&mut state, &reg, "Demonmail Hauberk", P0);
    let solo = ready_creature(&mut state, P0, 2, 2);

    let legal = engine::legal_actions(&state, &reg);
    let action = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == hauberk))
        .expect("equip is activatable: a creature is on the battlefield to pay the cost")
        .clone();
    assert!(matches!(&action,
        Action::ActivateAbility { targets, sacrifice: Some(sac), .. }
            if targets == &[Target::Object(solo)] && *sac == solo),
        "the only pair available targets and sacrifices the one creature: {action:?}");

    let after = resolve_activated(engine::submit_action(&state, &action, &reg), &reg);

    assert_eq!(after.get_object(solo).unwrap().zone, Zone::Graveyard,
        "the cost was paid — that is the point of the activation");
    assert_eq!(after.get_object(hauberk).unwrap().attached_to, None,
        "and the equip found no legal target on resolution, so it attached to \
         nothing (CR 608.2b)");
}

#[test]
fn hauberk_offers_the_pair_that_sacrifices_its_own_target() {
    // With several creatures the self-sacrificing pair is not the useful one,
    // but it is legal, so it is offered. The player decides whether a
    // fizzling equip is worth a sacrifice; the engine does not decide for them.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = named_permanent(&mut state, &reg, "Demonmail Hauberk", P0);
    let a = ready_creature(&mut state, P0, 1, 1);
    let b = ready_creature(&mut state, P0, 2, 2);
    let _c = ready_creature(&mut state, P0, 3, 3);

    let legal = engine::legal_actions(&state, &reg);
    let offers = |t: ObjectId, sac: ObjectId| legal.actions.iter().any(|act| matches!(act,
        Action::ActivateAbility { object_id, targets, sacrifice: Some(s), .. }
            if *object_id == hauberk && targets == &[Target::Object(t)] && *s == sac));

    assert!(offers(a, a), "target and sacrifice the same creature is a legal activation");
    assert!(offers(a, b), "and so is the ordinary pair");
}

// ════════════════════════════════════════════════════════════════════
// Disciple of Griselbrand — {1}, Sacrifice a creature: gain life equal to toughness
// ════════════════════════════════════════════════════════════════════

#[test]
fn disciple_of_griselbrand_player_picks_highest_toughness_sacrifice() {
    // Disciple has an untargeted SacrificeCreature ability — the player
    // should be able to pick which creature to sacrifice (i.e. the one with
    // the highest toughness for max life gain).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    let small = ready_creature(&mut state, P0, 1, 1);
    let big = ready_creature(&mut state, P0, 2, 6);

    // Mana for the {1} cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    // Should enumerate one ActivateAbility per eligible sacrifice (3 creatures total).
    let activates: Vec<&Action> = legal.actions.iter().filter(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == disciple)).collect();
    assert_eq!(activates.len(), 3,
        "should offer one option per sacrificable creature (disciple itself + small + big), got {}",
        activates.len());

    // Now pick the option that sacrifices the big creature.
    let big_action = activates.iter().find(|a| matches!(a,
        Action::ActivateAbility { sacrifice: Some(s), .. } if *s == big)).unwrap();
    let life_before = state.get_player(P0).life;
    let new_state = resolve_activated(engine::submit_action(&state, big_action, &reg), &reg);
    let gained = new_state.get_player(P0).life - life_before;
    assert_eq!(gained, 6, "should gain 6 life (big creature's toughness)");
    assert_eq!(new_state.get_object(big).unwrap().zone, Zone::Graveyard);
    // Disciple and small both still on board.
    assert_eq!(new_state.get_object(disciple).unwrap().zone, Zone::Battlefield);
    assert_eq!(new_state.get_object(small).unwrap().zone, Zone::Battlefield);
}

/// Ruling (2011-09-22): "The amount of life you gain is equal to the toughness
/// of the creature as it last existed on the battlefield, not its toughness in
/// the graveyard."
///
/// Which creature is "the sacrificed creature" is settled when the cost is paid
/// (CR 601.2h); the ability resolves later, and players get priority in
/// between. The card used to read the *most recent* `CreatureDied` event, so
/// anything that died in that window answered for the creature that actually
/// paid: sacrificing a 1/1 while the opponent killed a 5/9 in response gained
/// nine life.
#[test]
fn disciple_of_griselbrand_reads_the_creature_that_paid_not_the_last_one_to_die() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    let paid_with = ready_creature(&mut state, P0, 1, 1);
    let bystander = ready_creature(&mut state, P0, 5, 9);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let act = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, sacrifice: Some(s), .. }
            if *object_id == disciple && *s == paid_with))
        .expect("sacrificing the 1/1 is on offer")
        .clone();
    let mut state = engine::submit_action(&state, &act, &reg);
    let life_before = state.get_player(P0).life;

    // In the window between paying the cost and the ability resolving,
    // something else dies — and it is bigger.
    mtg_engine::destruction::try_destroy(&mut state, bystander, &reg);
    assert_eq!(state.get_object(bystander).unwrap().zone, Zone::Graveyard,
        "test setup: the bystander died after the cost was paid");

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P0).life - life_before, 1,
        "one life, for the 1/1 that paid the cost — not nine for the 5/9 that \
         happened to die most recently");
}

/// The other half of the same ruling: "the toughness of the creature **as it
/// last existed on the battlefield**, not its toughness in the graveyard."
///
/// A creature with a +1/+1 counter is bigger on the battlefield than the card
/// that lands in the graveyard, which CR 400.7 makes a new object printed as
/// itself. Reading the graveyard object's toughness instead of the
/// `CreatureDied` event's `last_known_toughness` passed the whole workspace.
#[test]
fn disciple_of_griselbrand_gains_the_toughness_it_had_on_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    let fodder = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    state.add_counters(fodder, CounterType::PlusOnePlusOne, 3);
    assert_eq!(state.effective_toughness(fodder, &reg), Some(5),
        "test setup: a 2/2 with three +1/+1 counters is a 5/5 on the battlefield");

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    let legal = engine::legal_actions(&state, &reg);
    let act = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, sacrifice: Some(s), .. }
            if *object_id == disciple && *s == fodder))
        .expect("sacrificing the Walking Corpse is on offer")
        .clone();
    let life_before = state.get_player(P0).life;
    let state = resolve_activated(engine::submit_action(&state, &act, &reg), &reg);

    assert_eq!(state.get_player(P0).life - life_before, 5,
        "five life — what it was on the battlefield, not the 2 the card in the \
         graveyard is printed with");
}

#[test]
fn disciple_of_griselbrand_can_sacrifice_itself() {
    // Disciple is its own valid sacrifice — gain 1 life from its 1 toughness
    // (and lose the disciple). The legal_actions list should include this combo.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let self_sac_action = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, sacrifice: Some(s), .. }
            if *object_id == disciple && *s == disciple)).cloned();
    assert!(self_sac_action.is_some(),
        "disciple should be allowed to sacrifice itself for the cost");

    let new_state = resolve_activated(engine::submit_action(&state, &self_sac_action.unwrap(), &reg), &reg);
    // Disciple in graveyard, gained 1 life.
    assert_eq!(new_state.get_object(disciple).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_player(P0).life, 21);
}

// ════════════════════════════════════════════════════════════════════
// Skirsdag Cultist — {R}, {T}, Sacrifice a creature: 2 damage to any target
// ════════════════════════════════════════════════════════════════════

#[test]
fn skirsdag_cultist_explicit_sacrifice() {
    // Cultist + fodder + opponent creature. Player picks fodder as the
    // sacrifice and the opponent creature as the damage target.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    let fodder = ready_creature(&mut state, P0, 1, 1);
    let target = ready_creature(&mut state, P1, 3, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let action = Action::ActivateAbility {
        object_id: cultist,
        ability_index: 0,
        targets: vec![Target::Object(target)],
        tap_plan: vec![],
        sacrifice: Some(fodder),
        x_value: None,
        source_card_id: None,
    };
    let new_state = resolve_activated(engine::submit_action(&state, &action, &reg), &reg);
    assert_eq!(new_state.get_object(target).unwrap().damage_marked, 2);
    assert_eq!(new_state.get_object(fodder).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_object(cultist).unwrap().zone, Zone::Battlefield,
        "cultist should still be alive (we sacrificed the fodder, not itself)");
}

#[test]
fn skirsdag_cultist_enumerates_every_target_sacrifice_pair() {
    // Setup: cultist + fodder + opp creature + 1 untapped Red mana floating.
    // "Any target": cultist, fodder, opp creature, P0 and P1 — 5 targets.
    // Sacrifices: cultist or fodder — 2 options. Every pair is legal, so
    // 5 × 2 = 10. The two pairs that sacrifice the creature they target used
    // to be hidden; they fizzle the damage, which is the player's business.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    let fodder = ready_creature(&mut state, P0, 1, 1);
    let _target = ready_creature(&mut state, P1, 3, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &reg);
    let count = legal.actions.iter().filter(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == cultist)).count();
    assert_eq!(count, 10, "expected 5 targets × 2 sacrifices = 10 combos");

    // Including the self-sacrificing one.
    assert!(legal.actions.iter().any(|act| matches!(act,
        Action::ActivateAbility { object_id, targets, sacrifice: Some(s), .. }
            if *object_id == cultist && targets == &[Target::Object(fodder)] && *s == fodder)),
        "targeting the fodder and sacrificing it is legal, so it is offered");
}

#[test]
fn skirsdag_cultist_may_sacrifice_the_creature_it_targeted() {
    // The activation is legal even though the damage will fizzle: the cost is
    // paid on activation (CR 601.2h), so the creature dies either way, and
    // whether that trade is worth making is the player's judgement.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    let own_creature = ready_creature(&mut state, P0, 2, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &reg);
    let action = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, sacrifice: Some(s), .. }
            if *object_id == cultist
            && targets == &[Target::Object(own_creature)]
            && *s == own_creature))
        .expect("targeting and sacrificing the same creature is a legal activation")
        .clone();

    let after = resolve_activated(engine::submit_action(&state, &action, &reg), &reg);
    assert_eq!(after.get_object(own_creature).unwrap().zone, Zone::Graveyard,
        "the sacrifice is a cost, so it happens whatever becomes of the damage");
}

// ════════════════════════════════════════════════════════════════════
// No-mana-autotap-with-sacrifice rule
// ════════════════════════════════════════════════════════════════════

#[test]
fn disciple_does_not_appear_with_only_untapped_lands_and_no_floating_mana() {
    // Disciple's {1} cost: with 1 untapped land and no floating mana, the
    // ability should NOT auto-tap (sacrifice abilities require manual mana).
    // This protects against the engine tapping a creature mana source for the
    // {1} and then sacrificing that same creature, or other autotap weirdness.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    let _fodder = ready_creature(&mut state, P0, 1, 1);
    // 1 untapped Forest, but no mana floating in the pool.
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    assert!(state.get_player(P0).mana_pool.is_empty());

    let legal = engine::legal_actions(&state, &reg);
    let any_disciple = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == disciple));
    assert!(!any_disciple,
        "disciple's ability should NOT appear when only untapped lands are available — \
         the player must manually tap their lands first to float the mana");
}

#[test]
fn disciple_appears_when_mana_is_already_in_the_pool() {
    // Same setup as above but with the {1} already floating in the mana pool —
    // now the ability should appear.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    let _fodder = ready_creature(&mut state, P0, 1, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let any_disciple = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == disciple));
    assert!(any_disciple,
        "disciple's ability should appear once the mana is in the pool");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Demonmail Hauberk's equip cost is "Sacrifice a creature."
/// The engine only checks that ANY creature exists (including the
/// creature being equipped), not that a DIFFERENT creature can be sacrificed.
#[test]
fn hauberk_can_sacrifice_the_creature_it_is_equipping_to_move_itself() {
    // The card's one ruling, in as many words: "You can sacrifice the creature
    // Demonmail Hauberk is equipping in order to equip it to another
    // creature."
    //
    // This test used to assert the opposite of what it is now: that equip is
    // unavailable with a single creature on the battlefield, reasoning from
    // this same ruling. The ruling grants a permission — you *may* sacrifice
    // the equipped creature — and says nothing about a minimum board. Reading
    // a restriction out of it cost the card its use as a sacrifice outlet.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = named_permanent(&mut state, &reg, "Demonmail Hauberk", P0);
    let equipped = ready_creature(&mut state, P0, 2, 2);
    let other = ready_creature(&mut state, P0, 3, 3);

    // Start with the Hauberk already on `equipped`.
    state.get_object_mut(hauberk).unwrap().attached_to = Some(equipped);

    let legal = engine::legal_actions(&state, &reg);
    let action = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, sacrifice: Some(s), .. }
            if *object_id == hauberk && targets == &[Target::Object(other)] && *s == equipped))
        .expect("equip `other`, paying by sacrificing the creature currently equipped")
        .clone();

    let after = resolve_activated(engine::submit_action(&state, &action, &reg), &reg);

    assert_eq!(after.get_object(equipped).unwrap().zone, Zone::Graveyard,
        "the equipped creature paid the cost");
    assert_eq!(after.get_object(hauberk).unwrap().attached_to, Some(other),
        "and the Hauberk moved to the creature it targeted");
    assert_eq!(after.effective_power(other, &reg), Some(7), "3 + 4");
    assert_eq!(after.effective_toughness(other, &reg), Some(5), "3 + 2");
}



/// Issue #141: in a REAL game the ability resolves through later
/// `submit_action` passes, and submit_action clears `state.events` at the
/// top of every action — the old event-scan found nothing there, so the
/// Disciple gained 0 life in every real game while the direct-resolution
/// tests above kept passing. The toughness snapshot rides the stack entry
/// now; this test drives the whole flow through submit_action.
#[test]
fn disciple_of_griselbrand_gains_life_through_real_priority_passes() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    let fodder = ready_creature(&mut state, P0, 2, 5);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let act = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, sacrifice: Some(s), .. }
            if *object_id == disciple && *s == fodder))
        .expect("sacrificing the 2/5 is on offer")
        .clone();
    let life_before = state.get_player(P0).life;

    let mut state = engine::submit_action(&state, &act, &reg);
    // Both players pass — each pass is its own submit_action, and every
    // submit_action clears state.events. Then the game loop resolves the
    // top of the stack, exactly as run_game_loop does after the passes.
    state = engine::submit_action(&state, &Action::PassPriority, &reg);
    state = engine::submit_action(&state, &Action::PassPriority, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.stack.is_empty(), "the ability resolved through the passes");
    assert_eq!(state.get_player(P0).life - life_before, 5,
        "five life for the 2/5 that paid the cost, in a real-game flow");
}

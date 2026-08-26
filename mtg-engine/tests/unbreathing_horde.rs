//! Tests for Unbreathing Horde.
//!
//! Oracle: {2}{B} 0/0 Zombie
//! Unbreathing Horde enters the battlefield with a +1/+1 counter on it for each
//! other Zombie you control and each Zombie card in your graveyard.
//! If Unbreathing Horde would be dealt damage, prevent that damage and remove a
//! +1/+1 counter from it.

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Combat damage is prevented and a counter is removed.
#[test]
fn prevents_combat_damage_removes_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    // Give it 3 +1/+1 counters.
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    // Attacker attacks, Horde blocks.
    let attacker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P0, &[horde]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // The Horde should have taken no damage but lost a counter.
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "Damage should be prevented");
    let counters = state.get_object(horde).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 2, "Should have lost one +1/+1 counter");
}

/// When Unbreathing Horde deals damage as attacker, the other creature still takes damage.
#[test]
fn still_deals_damage_to_others() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    let blocker = ready_creature(&mut state, P1, 2, 5);
    // Horde attacks, blocker blocks.
    attacks_blocked_by(&mut state, horde, P1, &[blocker]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // The blocker should have taken damage from Horde (0 base + 3 counters = 3 power).
    assert!(state.get_object(blocker).unwrap().damage_marked > 0,
        "Blocker should take damage from Unbreathing Horde");
    // The Horde should have taken no damage (prevented).
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "Horde damage should be prevented");
}

/// ETB counter count is correct with zombies on battlefield and graveyard.
#[test]
fn enters_with_correct_counter_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 2 zombies on battlefield.
    let _z1 = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let _z2 = named_permanent(&mut state, &reg, "Diregraf Ghoul", P0);

    // Put 1 zombie in graveyard.
    let _z3 = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    // Cast Unbreathing Horde — on_resolve counts graveyard before moving to battlefield.
    let horde = castable_spell(&mut state, &reg, "Unbreathing Horde", P0);
    state = cast_and_resolve(&state, &reg, horde, vec![]);

    let counters = state.get_object(horde).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    // 2 battlefield zombies + 1 graveyard zombie = 3 counters.
    assert_eq!(counters, 3, "Should have 3 +1/+1 counters (2 bf + 1 gy zombies)");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug AC (`audits/AUDIT_BUGS.md)`: Unbreathing Horde under-counts when
/// reanimated from a graveyard. Per Scryfall ruling: "If Unbreathing
/// Horde enters from a graveyard, it counts itself for its enter-with-
/// counters ability."
///
/// Oracle (Unbreathing Horde): "This creature enters with a +1/+1
/// counter on it for each other Zombie you control and each Zombie
/// card in your graveyard."
///
/// "Enters with X counters" is a CR 614.1c replacement effect, so the
/// count is computed at entry timing — at which point the Horde is
/// still in the graveyard zone (it hasn't fully entered yet) and the
/// "Zombie cards in your graveyard" count includes the Horde itself.
///
/// Failure mode: `unbreathing_horde.rs` runs the
/// `add_zombie_counters` helper from the `on_enter_battlefield`
/// handler — i.e. AFTER the move to battlefield. By that point,
/// `count_zombies_in_graveyard` no longer sees the Horde (it's on
/// the battlefield), so the reanimated Horde misses one counter
/// compared to the cast path.
///
/// We put two other Zombies in P0's graveyard alongside the Horde,
/// then move the Horde to the battlefield (mirroring Unburial Rites
/// reanimation), then fire the ETB handler. The fix should give the
/// Horde three +1/+1 counters (2 other Zombies + the Horde itself);
/// the bug gives it only two.
#[test]
fn bug_ac_unbreathing_horde_counts_itself_when_reanimated() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two other Zombie creature cards in P0's graveyard.
    let walking_corpse_id = registry.get_id_by_name("Walking Corpse").unwrap();
    let z1 = state.create_object(walking_corpse_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(z1).unwrap().name = "Walking Corpse (a)".into();
    let z2 = state.create_object(walking_corpse_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(z2).unwrap().name = "Walking Corpse (b)".into();

    // Unbreathing Horde sitting in P0's graveyard, ready to be reanimated.
    let horde_card_id = registry.get_id_by_name("Unbreathing Horde").unwrap();
    let horde = state.create_object(horde_card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(horde).unwrap().name = "Unbreathing Horde".into();

    // Reanimate: move the Horde to the battlefield and fire its ETB
    // handler (this mirrors what Unburial Rites does).
    state.move_object(horde, Zone::Battlefield, &registry);
    let behavior = registry.get(horde_card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, horde, &[], &registry);

    let counters = state
        .get_object(horde)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);

    assert!(
        counters >= 3,
        "Reanimated Unbreathing Horde should enter with at least 3 \
         +1/+1 counters (2 other Zombies in graveyard + the Horde \
         counts itself per the Scryfall ruling). Bug AC: \
         on_enter_battlefield runs after the move, so the helper sees \
         only the 2 other Zombies in the graveyard and adds 2 counters. \
         Got: {counters}",
    );
}

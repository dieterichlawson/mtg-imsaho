//! A creature removed from combat (`GameState::remove_from_combat`: by
//! regenerating, CR 701.15c; by changing control, 506.4d; or by
//! anything else that removes it, 506.4), which the card proofs' recorders
//! and the damage proofs take for granted.

use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{card_proof, new_game};
use crate::state::CombatState;

/// Removing a creature from combat (CR 506.4): it is attacking,
/// blocked, blocking and among the creatures that dealt first-strike
/// damage no longer, nor among any attacker's blockers, and attacks no
/// planeswalker; every other creature keeps its place. So an attacker it
/// blocked stays blocked (509.2: blocked once, blocked for the combat),
/// and a creature blocking it, if it attacked, stays a blocking creature,
/// blocking nothing (509.1g, 510.1d, #983), until it is removed itself.
/// On a combat of attackers 1 and 2, attacking as `attacking` says, the
/// first at a planeswalker; blockers 3 and 4, each blocking the attacker
/// `blocks` names, if any; every attacker with a blocker blocked, and
/// one whose blockers are all gone blocked or not, any answer; and the
/// creatures `struck` names having dealt first-strike damage; and the
/// creature `removed`: constants of each harness, for the combat's sets
/// built every way, or a creature the solver chose taken out of them,
/// ran the solver out of memory.
fn leaving(attacking: [bool; 2], blocks: [Option<u64>; 2], struck: [bool; 4], removed: u64) {
    let mut state = new_game();
    let defending = PlayerId(u8::from(kani::any::<bool>()));
    let mut combat = CombatState::default();
    combat.blocking_creatures.reserve(2);
    combat.blocked_attackers.reserve(2);
    combat.dealt_first_strike.reserve(4);
    for (i, &on) in attacking.iter().enumerate() {
        if on {
            let a = ObjectId(i as u64 + 1);
            combat.attackers.insert(a, defending);
            combat.blocker_assignments.insert(a, Vec::with_capacity(2));
            if i == 0 {
                combat.planeswalker_defenders.insert(a, ObjectId(5));
            }
        }
    }
    for (j, on) in blocks.iter().enumerate() {
        if let Some(a) = on {
            let b = ObjectId(j as u64 + 3);
            combat.blocker_assignments.get_mut(&ObjectId(*a)).expect("an attacker").push(b);
            combat.blocking_creatures.insert(b);
        }
    }
    let mut blocked = [false; 2];
    for (i, slot) in blocked.iter_mut().enumerate() {
        let a = i as u64 + 1;
        *slot = attacking[i] && (blocks.contains(&Some(a)) || kani::any());
        if *slot {
            combat.blocked_attackers.insert(ObjectId(a));
        }
    }
    for (i, &dealt) in struck.iter().enumerate() {
        if dealt {
            combat.dealt_first_strike.insert(ObjectId(i as u64 + 1));
        }
    }
    state.combat = Some(combat);
    let removed = ObjectId(removed);

    state.remove_from_combat(removed);

    let combat = state.combat.as_ref().expect("still a combat");
    for i in 0..2 {
        let a = ObjectId(i as u64 + 1);
        let stays = attacking[i] && a != removed;
        assert!(combat.attackers.contains_key(&a) == stays);
        assert!(combat.blocked_attackers.contains(&a) == (blocked[i] && a != removed));
        assert!(combat.planeswalker_defenders.get(&a).is_some() == (stays && i == 0));
        match combat.blocker_assignments.get(&a) {
            None => {
                assert!(!stays);
            }
            Some(list) => {
                assert!(stays);
                // Its blockers, in order, but the one removed.
                let mut k = 0;
                for (j, on) in blocks.iter().enumerate() {
                    let b = ObjectId(j as u64 + 3);
                    if *on == Some(a.0) && b != removed {
                        assert!(k < list.len() && list[k] == b);
                        k += 1;
                    }
                }
                assert!(list.len() == k);
            }
        }
    }
    for (j, on) in blocks.iter().enumerate() {
        let b = ObjectId(j as u64 + 3);
        assert!(combat.blocking_creatures.contains(&b) == (on.is_some() && b != removed));
    }
    for (i, &dealt) in struck.iter().enumerate() {
        let c = ObjectId(i as u64 + 1);
        assert!(combat.dealt_first_strike.contains(&c) == (dealt && c != removed));
    }
    std::mem::forget(state);
}

/// The proofs of a creature removed from combat, a combat each.
macro_rules! each_combat_left {
    ($($name:ident: $attacking:expr, $blocks:expr, $struck:expr, $removed:expr;)*) => {$(
        card_proof! {
            /// [`leaving`] for the combat and the creature of its name.
            fn $name() {
                leaving($attacking, $blocks, $struck, $removed);
            }
        }
    )*};
}

each_combat_left! {
    an_unblocked_attacker_removed_leaves_the_other_attacking: [true, true], [None, None], [true, false, false, false], 1;
    a_creature_in_no_combat_removed_changes_nothing: [true, false], [Some(1), None], [false, false, true, false], 4;
    a_blocked_attacker_removed_leaves_its_blocker_blocking: [true, false], [Some(1), None], [false, false, true, false], 1;
    a_blocker_removed_leaves_its_attacker_blocked: [true, false], [Some(1), None], [true, false, true, false], 3;
    a_double_blocked_attacker_removed_leaves_both_blocking: [true, false], [Some(1), Some(1)], [true, false, false, true], 1;
    one_of_two_blockers_removed_leaves_the_other_in_order: [true, false], [Some(1), Some(1)], [true, false, false, true], 3;
    the_second_attacker_removed_leaves_the_first_as_it_was: [true, true], [Some(1), Some(2)], [false, true, true, false], 2;
    the_second_blocker_removed_leaves_its_attacker_blocked: [true, true], [Some(1), Some(2)], [false, true, true, false], 4;
}

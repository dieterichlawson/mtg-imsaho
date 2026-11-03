//! Who has priority once an action has been played
//! (`engine::hand_on_priority`, CR 117.3-117.4): the game loop's last step
//! after each action it plays. Which step comes after one ends is
//! `turn_based`'s, and what resolving the top of the stack does is
//! `resolving`'s; each stands in here as recorded.

use crate::actions::{Action, ResolvedChoice};
use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, StackEntry};

/// What the priority step did besides handing priority: 1 ended the step,
/// 2 resolved the top of the stack.
static mut DID: [u8; 2] = [0; 2];
static mut DID_N: usize = 0;

fn did(n: u8) {
    // SAFETY: a proof runs on one thread, and only this function writes DID and DID_N.
    let (d, k) = unsafe { (&mut *std::ptr::addr_of_mut!(DID), &mut *std::ptr::addr_of_mut!(DID_N)) };
    d[*k] = n;
    *k += 1;
}

#[allow(dead_code)] // each named only by `kani::stub`
fn step_ended(_: &mut GameState, _: &CardRegistry) {
    did(1);
}
#[allow(dead_code)]
fn top_resolved(_: &mut GameState, _: &CardRegistry) {
    did(2);
}

crate::proofs::cards::card_proof! {
    #[kani::stub(crate::engine::advance_step, step_ended)]
    #[kani::stub(crate::stack::resolve_top_of_stack, top_resolved)]
    /// A pass of priority (CR 117.3d, 117.4): with fewer than both players
    /// having passed in succession, priority goes to the other player (and
    /// stays with nobody if nobody held it); with both having passed and
    /// the stack empty, the step ends, nobody holding priority as it does;
    /// with both having passed and something on the stack, the top of it
    /// resolves, the passes in a row start again, and the active player
    /// receives priority (117.3b). Which, of these, is the turn kernel's
    /// (`mtg_kernel::turn::after_pass`, proved for every number of
    /// players). For either player active, priority with either or with
    /// nobody, none to three passes in a row, and the stack empty or not.
    fn a_pass_hands_priority_on_or_resolves_the_top_of_the_stack_or_ends_the_step() {
        let mut registry = CardRegistry::new();
        let _ = registry.register(Box::new(crate::proofs::cards::Blank));
        let mut state = crate::proofs::cards::new_game();
        let active = PlayerId(u8::from(kani::any::<bool>()));
        state.active_player = active;
        let holder: Option<PlayerId> = if kani::any() { Some(PlayerId(u8::from(kani::any::<bool>()))) } else { None };
        state.priority_player = holder;
        let passes = u32::from(kani::any::<u8>() % 4);
        state.consecutive_passes = passes;
        let stacked: bool = kani::any();
        if stacked {
            state.stack.push(StackEntry::Spell(ObjectId(1)));
        }

        crate::engine::hand_on_priority(&mut state, &Action::PassPriority, None, 2, &registry);

        // SAFETY: as in `did`.
        let (d, n) = unsafe { (*std::ptr::addr_of!(DID), *std::ptr::addr_of!(DID_N)) };
        if passes < 2 {
            let current = holder.unwrap_or(active);
            assert!(n == 0 && state.consecutive_passes == passes);
            assert!(state.priority_player == holder.map(|_| PlayerId(1 - current.0)));
        } else if !stacked {
            assert!(n == 1 && d[0] == 1 && state.priority_player.is_none());
        } else {
            assert!(n == 1 && d[0] == 2 && state.consecutive_passes == 0 && state.priority_player == Some(active));
        }
        std::mem::forget((registry, state));
    }
}

crate::proofs::cards::card_proof! {
    #[kani::stub(crate::engine::advance_step, step_ended)]
    #[kani::stub(crate::stack::resolve_top_of_stack, top_resolved)]
    /// Priority after an action other than a pass (CR 117.3b-c, 508.1,
    /// 509.1): a declaration of attackers or of blockers gives it to the
    /// active player; an answer to a question gives it to the player
    /// casting or activating, if the question was part of the cast or the
    /// activation, and to the active player otherwise; a land played, a
    /// spell cast, an ability activated, a mulligan or a discard leaves it
    /// where it is; and none of them ends the step or resolves anything.
    /// For either player active, priority with either or with nobody, and
    /// the question part of a cast by either player or not.
    fn an_action_other_than_a_pass_leaves_priority_where_the_rules_put_it() {
        let mut registry = CardRegistry::new();
        let _ = registry.register(Box::new(crate::proofs::cards::Blank));
        let mut state = crate::proofs::cards::new_game();
        let active = PlayerId(u8::from(kani::any::<bool>()));
        state.active_player = active;
        let holder: Option<PlayerId> = if kani::any() { Some(PlayerId(u8::from(kani::any::<bool>()))) } else { None };
        state.priority_player = holder;
        let caster: Option<PlayerId> = if kani::any() { Some(PlayerId(u8::from(kani::any::<bool>()))) } else { None };
        let kind = kani::any::<u8>() % 7;
        let action = match kind {
            0 => Action::DeclareAttackers { attackers: Vec::new(), planeswalker_attacks: Vec::new() },
            1 => Action::DeclareBlockers { assignments: Vec::new() },
            2 => Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(kani::any()) },
            3 => Action::PlayLand { object_id: ObjectId(1) },
            4 => Action::ActivateManaAbility { object_id: ObjectId(1), ability_index: 0 },
            5 => Action::MulliganKeep,
            _ => Action::DiscardCards { cards: Vec::new() },
        };

        crate::engine::hand_on_priority(&mut state, &action, caster, 2, &registry);

        // SAFETY: as in `did`.
        let n = unsafe { *std::ptr::addr_of!(DID_N) };
        let expected = match kind {
            0 | 1 => Some(active),
            2 => Some(caster.unwrap_or(active)),
            _ => holder,
        };
        assert!(n == 0 && state.priority_player == expected);
        std::mem::forget((registry, state, action));
    }
}

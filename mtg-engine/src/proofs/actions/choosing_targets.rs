//! Asking a player to choose what an effect applies to as it resolves,
//! which a card proof's recorders stand in for: `helpers::present_target_choice`
//! and `present_optional_target_choice`, and the tokens that enter the
//! combat attacking, `helpers::tokens_enter_combat_attacking` (CR 506.3,
//! 508.4). What the effect does to what is chosen is
//! `apply_pending_effect`'s, recorded here.

use crate::actions::Target;
use crate::cards::helpers;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_for, blank_board, Act, Snapshot};
use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
use crate::types::{CardType, Zone};

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::engine::apply_pending_effect, crate::proofs::cards::record_effect)]
/// "Choose one of them" as an effect resolves: with nothing to choose
/// among, nothing happens; with one, and the choice not one the player
/// may decline, the effect applies to it at once; otherwise the player is
/// asked, once, to choose among exactly those, in their order, declining
/// allowed exactly when the choice is optional, and nothing has happened
/// yet. For either player, up to three options of any kind, through
/// `present_target_choice` and `present_optional_target_choice`. Nothing
/// else changes.
fn a_choice_of_target_applies_at_once_or_asks_among_exactly_those() {
    let (registry, mut state, blank) = blank_board();
    let source = state.create_object(blank, PlayerId(0), Zone::Stack, None, None);
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let any_target = || match kani::any::<u8>() % 3 {
        0 => Target::Object(ObjectId(7)),
        1 => Target::Player(PlayerId(u8::from(kani::any::<bool>()))),
        _ => Target::Object(ObjectId(8)),
    };
    let all = [any_target(), any_target(), any_target()];
    let n: usize = kani::any();
    kani::assume(n <= 3);
    let mut options = Vec::with_capacity(3);
    for t in &all[..n] {
        options.push(t.clone());
    }
    let effect = PendingEffect::DealDamage { amount: 2, source_id: source };
    let optional: bool = kani::any();
    let before = Snapshot::of(&state);

    if optional && kani::any() {
        helpers::present_optional_target_choice(&mut state, source, who, options, effect.clone(), "a choice", &registry);
    } else {
        helpers::present_target_choice(&mut state, source, who, options, effect.clone(), "a choice", optional, &registry);
    }

    if n == 0 {
        assert!(acted() == 0 && Snapshot::of(&state) == before);
    } else if n == 1 && !optional {
        assert!(asked_for(&[Act::Effect(all[0].clone(), effect.clone())]) && Snapshot::of(&state) == before);
    } else {
        assert!(acted() == 0 && Snapshot::of(&state) == before.with_asking(true));
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source: s, choice: ResolutionChoiceKind::ChooseTarget { options: o, optional: opt, effect: e, .. },
        }) if *player == who && *s == source && o.as_slice() == &all[..n] && *opt == optional && *e == effect));
    }
    kani::cover!(n == 1 && optional, "one option, which may be declined");
    std::mem::forget((registry, state, all, effect));
}

/// Tokens put onto the battlefield attacking (CR 506.3): each arrives
/// tapped, which taps nothing and so is not announced, and was never
/// declared an attacker, so summoning sickness has nothing to stop
/// (302.6); and the player who controls them chooses what the first of
/// them attacks among the opponents still in the game and the
/// planeswalkers they control (508.4), with the rest to follow, as
/// `present_target_choice` asks (above): the opponent at once when it is
/// the only option. For `n` tokens, a planeswalker of either player's if
/// `with_walker`, and the opponent out of the game if `gone`. Nothing
/// else changes.
fn entering_attacking(you: PlayerId, with_walker: bool, n: usize, gone: bool) {
    let (registry, mut state, blank) = blank_board();
    let them = PlayerId(1 - you.0);
    let source = state.create_object(blank, you, Zone::Stack, None, None);
    let walker = with_walker.then(|| {
        let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, None, None);
        state.get_object_mut(id).expect("just created").card_types = vec![CardType::Planeswalker];
        id
    });
    let theirs = walker.filter(|w| state.get_object(*w).expect("on the board").controller == them);
    if gone {
        // A branch on the player, not an index by it: `any_object` says why.
        if them == PlayerId(0) { state.players[0].lost = true } else { state.players[1].lost = true }
    }
    let tokens = [0, 1].map(|_| state.create_object(blank, you, Zone::Battlefield, Some(1), Some(1)));
    let before = Snapshot::of(&state);

    helpers::tokens_enter_combat_attacking(&mut state, source, you, &tokens[..n], &registry);

    let mut entered = before.clone();
    for (i, t) in tokens.iter().enumerate() {
        let o = state.get_object(*t).expect("on the board");
        if i < n {
            entered = entered.with_tapped(*t, true);
            assert!(!o.summoning_sick);
        } else {
            assert!(o.summoning_sick && !o.tapped);
        }
    }
    let effect = PendingEffect::TokenAttacks { token_id: tokens[0], remaining: tokens[1..n].to_vec(), source_id: source };
    let options = usize::from(theirs.is_some()) + usize::from(!gone);
    match (theirs, gone) {
        (None, true) => {
            assert!(acted() == 0 && Snapshot::of(&state) == entered);
        }
        (Some(w), true) => {
            assert!(asked_for(&[Act::Effect(Target::Object(w), effect.clone())]) && Snapshot::of(&state) == entered);
        }
        (None, false) => {
            assert!(asked_for(&[Act::Effect(Target::Player(them), effect.clone())]) && Snapshot::of(&state) == entered);
        }
        (Some(w), false) => {
            assert!(acted() == 0 && Snapshot::of(&state) == entered.with_asking(true));
            assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source: s, choice: ResolutionChoiceKind::ChooseTarget { options: o, optional: false, effect: e, .. },
            }) if *player == you && *s == source && o.len() == 2 && o[0] == Target::Object(w) && o[1] == Target::Player(them) && *e == effect));
        }
    }
    kani::cover!(n == 2 && options == 2, "two tokens, two things to attack");
    std::mem::forget((registry, state, effect));
}

/// `entering_attacking` for tokens of either player's and the opponent in
/// the game or out, each a constant in each call. Whether there is a
/// planeswalker, and how many tokens enter, are constants in each harness:
/// a planeswalker made or not moved the tokens' ids, a count left open was
/// a list of every length, as was the list of what to attack with the
/// opponent in the game or not, and the player an index into the players
/// the solver left open; together they ran out of memory.
fn for_either_player(with_walker: bool, n: usize) {
    let (you, gone): (bool, bool) = (kani::any(), kani::any());
    for p in [false, true] {
        for g in [false, true] {
            if you == p && gone == g {
                entering_attacking(PlayerId(u8::from(p)), with_walker, n, g);
            }
        }
    }
}

/// A proof above for each board.
macro_rules! each_board {
    ($($name:ident: $walker:expr, $n:expr;)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::engine::apply_pending_effect, crate::proofs::cards::record_effect)]
        /// The proof above it, for one board.
        fn $name() {
            for_either_player($walker, $n);
        }
    )*};
}

each_board! {
    a_token_entering_attacking_on_a_board_without_a_planeswalker: false, 1;
    two_tokens_entering_attacking_on_a_board_without_a_planeswalker: false, 2;
    a_token_entering_attacking_on_a_board_with_a_planeswalker: true, 1;
    two_tokens_entering_attacking_on_a_board_with_a_planeswalker: true, 2;
}

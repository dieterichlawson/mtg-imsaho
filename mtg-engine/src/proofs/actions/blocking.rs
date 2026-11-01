//! Which creature may block which (`combat::can_block_attacker`, CR
//! 509.1a-b): the matrix of blocks the rules allow, pair by pair, which
//! the blocks kernel (`blocks::judge_blocks`) is handed and judges a
//! declaration against.
//!
//! Every question the rule asks of a creature is answered openly here,
//! the same each time it is asked of the same creature: whether it has
//! flying, reach or intimidate, can't be blocked, can't block, is a
//! creature or an artifact, its colors, and whether the attacker has
//! protection from the blocker. Each of those is its own function's
//! proof (`predicates`, `effects`, `scopes`, `until_end_of_turn`); this
//! one is of how the rule puts the answers together.

use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::blank_board;
use crate::state::{GameState, Protections};
use crate::types::{CardType, Color, Keyword, Zone};

/// What is asked of a creature.
#[derive(Clone, Copy)]
enum Asked {
    Flying,
    Reach,
    Intimidate,
    Unblockable,
    CanBlock,
    Creature,
    Artifact,
    White,
    Red,
    /// That the attacker has protection from it.
    Protects,
}

/// The answers, by creature (the board's first two objects, ids 1 and 2)
/// and question: chosen the first time each is asked, the same after.
static mut ANSWERS: [[Option<bool>; Asked::Protects as usize + 1]; 2] = [[None; Asked::Protects as usize + 1]; 2];

fn answer(id: ObjectId, asked: Asked) -> bool {
    let i = match id.0 {
        1 => 0,
        2 => 1,
        _ => panic!("a question about an object this proof does not make"),
    };
    // SAFETY: a proof runs on one thread, and only this function names ANSWERS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ANSWERS) };
    *answers[i][asked as usize].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `GameState::has_keyword`: flying, reach and intimidate
/// any answer; the rule asks nothing else.
#[allow(dead_code)] // named only by `kani::stub`
fn has_keyword(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    match keyword {
        Keyword::Flying => answer(id, Asked::Flying),
        Keyword::Reach => answer(id, Asked::Reach),
        Keyword::Intimidate => answer(id, Asked::Intimidate),
        _ => panic!("blocking asks only about flying, reach and intimidate"),
    }
}

/// Stands in for `GameState::cant_be_blocked`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn cant_be_blocked(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    answer(id, Asked::Unblockable)
}

/// Stands in for `GameState::can_block`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn can_block(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    answer(id, Asked::CanBlock)
}

/// Stands in for `GameState::is_creature`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn is_creature(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    answer(id, Asked::Creature)
}

/// Stands in for `GameState::has_card_type`: whether it is an artifact,
/// any answer; the rule asks nothing else.
#[allow(dead_code)] // named only by `kani::stub`
fn has_card_type(_: &GameState, id: ObjectId, card_type: CardType, _: &CardRegistry) -> bool {
    assert!(card_type == CardType::Artifact, "blocking asks only whether a blocker is an artifact");
    answer(id, Asked::Artifact)
}

/// Stands in for `GameState::colors_of`: white, red, both or neither.
#[allow(dead_code)] // named only by `kani::stub`
fn colors_of(_: &GameState, id: ObjectId, _: &CardRegistry) -> Vec<Color> {
    let mut colors = Vec::with_capacity(2);
    if answer(id, Asked::White) {
        colors.push(Color::White);
    }
    if answer(id, Asked::Red) {
        colors.push(Color::Red);
    }
    colors
}

/// Stands in for `Protections::covers`: whether the attacker has
/// protection from the blocker, any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn covers(_: &Protections, _: &GameState, source: ObjectId, _: &CardRegistry) -> bool {
    answer(source, Asked::Protects)
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
#[kani::stub(crate::state::GameState::cant_be_blocked, cant_be_blocked)]
#[kani::stub(crate::state::GameState::can_block, can_block)]
#[kani::stub(crate::state::GameState::is_creature, is_creature)]
#[kani::stub(crate::state::GameState::has_card_type, has_card_type)]
#[kani::stub(crate::state::GameState::colors_of, colors_of)]
#[kani::stub(crate::state::Protections::covers, covers)]
/// A creature may block an attacker exactly when it is an untapped
/// creature on the battlefield that can block (CR 509.1a), and the
/// attacker can be blocked (509.1b), and, if the attacker has flying, the
/// blocker has flying or reach (702.9b, 702.17b), and, if it has
/// intimidate, the blocker is an artifact or shares a color with it
/// (702.13b), and it has no protection from the blocker (702.16b). Over
/// two objects, the blocker either player's, tapped or not, on the
/// battlefield or in a graveyard, and every answer to every question.
fn a_creature_may_block_an_attacker_exactly_as_the_rules_allow() {
    let (registry, mut state, blank) = blank_board();
    let attacker = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
    let on: bool = kani::any();
    let blocker = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())),
        if on { Zone::Battlefield } else { Zone::Graveyard }, Some(2), Some(2));
    assert!(attacker == ObjectId(1) && blocker == ObjectId(2));
    let tapped: bool = kani::any();
    state.get_object_mut(blocker).expect("just created").tapped = tapped;

    let can = crate::combat::can_block_attacker(&state, blocker, attacker, &registry);

    let able = on && !tapped && answer(blocker, Asked::Creature) && answer(blocker, Asked::CanBlock);
    let up = answer(blocker, Asked::Flying) || answer(blocker, Asked::Reach);
    let shares = (answer(attacker, Asked::White) && answer(blocker, Asked::White))
        || (answer(attacker, Asked::Red) && answer(blocker, Asked::Red));
    let rules = able
        && !answer(attacker, Asked::Unblockable)
        && (!answer(attacker, Asked::Flying) || up)
        && (!answer(attacker, Asked::Intimidate) || answer(blocker, Asked::Artifact) || shares)
        && !answer(blocker, Asked::Protects);
    assert!(can == rules);
    kani::cover!(can && answer(attacker, Asked::Intimidate) && !answer(blocker, Asked::Artifact), "a shared color lets it block");
    std::mem::forget((registry, state));
}

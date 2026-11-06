//! What `GameState::matches_filter` makes of each creature filter a card
//! in the pool prints (CR 205.3, 205.4, 702): Stromkirk Noble's "can't be
//! blocked by Humans", Blazing Torch's "by Vampires or Zombies", Orchard
//! Spirit's "except by creatures with flying or reach", Spare from Evil's
//! "non-Human creatures" and Moonmist's "Werewolves and Wolves". Each
//! filter is its own Boolean combination of the questions it asks of the
//! object, and these show that `matches_filter` answers exactly that
//! combination.
//!
//! The questions themselves are any answer here, the same each time they
//! are asked of the object: whether it has a subtype, a card type, a
//! keyword. Each is its own function's proof (`predicates`, `scopes`);
//! this is of how a filter puts the answers together, so that the proofs
//! of what an effect printing one does (`effects`, `until_end_of_turn`)
//! can stand in for the filter with any answer and check only that it is
//! the effect's own filter that is asked. Each filter is built here, where
//! it is matched: read back from an effect printed on the heap, the solver
//! cannot tell it from any other filter, and matched every one.

use crate::proofs::held::Held;
use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::blank_board;
use crate::state::GameState;
use crate::types::{CardType, CreatureFilter as F, Keyword, Zone};

/// What is asked of the object.
#[derive(Clone, Copy)]
enum Asked {
    Human,
    Vampire,
    Zombie,
    Wolf,
    Werewolf,
    Creature,
    Flying,
    Reach,
}

/// The answers, by question: chosen the first time each is asked, the same
/// after.
static mut ANSWERS: Held<[Option<bool>; Asked::Reach as usize + 1]> = Held::new([None; Asked::Reach as usize + 1], (&raw const ANSWERS).cast());

fn answer(asked: Asked) -> bool {
    // SAFETY: a proof runs on one thread, and only this function names ANSWERS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ANSWERS.value) };
    *answers[asked as usize].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `GameState::has_subtype`: any answer for each subtype the
/// filters here name.
#[allow(dead_code)] // named only by `kani::stub`
fn has_subtype(_: &GameState, _: ObjectId, subtype: &str, _: &CardRegistry) -> bool {
    match subtype {
        "Human" => answer(Asked::Human),
        "Vampire" => answer(Asked::Vampire),
        "Zombie" => answer(Asked::Zombie),
        "Wolf" => answer(Asked::Wolf),
        "Werewolf" => answer(Asked::Werewolf),
        _ => panic!("a subtype no filter here names"),
    }
}

/// Stands in for `GameState::has_card_type`: any answer for creature.
#[allow(dead_code)] // named only by `kani::stub`
fn has_card_type(_: &GameState, _: ObjectId, card_type: CardType, _: &CardRegistry) -> bool {
    assert!(card_type == CardType::Creature, "a card type no filter here names");
    answer(Asked::Creature)
}

/// Stands in for `GameState::has_keyword`: any answer for flying and reach.
#[allow(dead_code)] // named only by `kani::stub`
fn has_keyword(_: &GameState, _: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    match keyword {
        Keyword::Flying => answer(Asked::Flying),
        Keyword::Reach => answer(Asked::Reach),
        _ => panic!("a keyword no filter here names"),
    }
}

/// Whether `filter` matches a creature of either player's, asked by a
/// source of either player's.
fn matched(filter: &F) -> bool {
    let (registry, mut state, blank) = blank_board();
    let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    let source_controller = PlayerId(u8::from(kani::any::<bool>()));
    let matched = state.matches_filter(id, filter, id, source_controller, &registry);
    std::mem::forget((registry, state));
    matched
}

/// The proofs of the filters, each question any answer as above.
macro_rules! each_filter {
    ($($name:ident: $body:block)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::has_subtype, has_subtype)]
        #[kani::stub(crate::state::GameState::has_card_type, has_card_type)]
        #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
        /// The filter of its name matches exactly what it says.
        fn $name() $body
    )*};
}

each_filter! {
    // Stromkirk Noble's: a creature may block it unless it is a Human.
    not_human_matches_exactly_what_is_not_a_human: {
        let f = F::Not(Box::new(F::HasSubtype("Human".into())));
        assert!(matched(&f) == !answer(Asked::Human));
        std::mem::forget(f);
    }
    // Blazing Torch's: unless it is a Vampire or a Zombie.
    not_vampire_or_zombie_matches_exactly_what_is_neither: {
        let f = F::Not(Box::new(F::Or(vec![F::HasSubtype("Vampire".into()), F::HasSubtype("Zombie".into())])));
        assert!(matched(&f) == !(answer(Asked::Vampire) || answer(Asked::Zombie)));
        std::mem::forget(f);
    }
    // Orchard Spirit's: only a creature with flying or reach.
    flying_or_reach_matches_exactly_what_has_either: {
        let f = F::Or(vec![F::HasKeyword(Keyword::Flying), F::HasKeyword(Keyword::Reach)]);
        assert!(matched(&f) == (answer(Asked::Flying) || answer(Asked::Reach)));
        std::mem::forget(f);
    }
    // Spare from Evil's: a creature that is not a Human.
    a_non_human_creature_matches_exactly_a_creature_not_a_human: {
        let f = F::And(vec![F::HasCardType(CardType::Creature), F::Not(Box::new(F::HasSubtype("Human".into())))]);
        assert!(matched(&f) == (answer(Asked::Creature) && !answer(Asked::Human)));
        std::mem::forget(f);
    }
    // Moonmist's: a Wolf or a Werewolf.
    wolf_or_werewolf_matches_exactly_what_is_either: {
        let f = F::Or(vec![F::HasSubtype("Wolf".into()), F::HasSubtype("Werewolf".into())]);
        assert!(matched(&f) == (answer(Asked::Wolf) || answer(Asked::Werewolf)));
        std::mem::forget(f);
    }
}

/// The filter [`stand_in`] checks it is handed.
static mut ASKED_OF: Held<*const F> = Held::new(std::ptr::null(), (&raw const ASKED_OF).cast());

/// Whether each of the board's first four objects matches that filter, as
/// [`stand_in`] answers: chosen once, the same after.
static mut MATCHED: Held<[Option<bool>; 4]> = Held::new([None; 4], (&raw const MATCHED).cast());

/// Points [`stand_in`] at the one filter it may be asked of: the filter an
/// effect on a proof's board prints.
pub(super) fn asked_of(filter: &F) {
    // SAFETY: a proof runs on one thread, and only this function writes ASKED_OF.
    unsafe { *std::ptr::addr_of_mut!(ASKED_OF.value) = filter };
}

/// Stands in for `GameState::matches_filter` in the proofs of what an
/// effect that prints a filter does (`effects`, `until_end_of_turn`):
/// asked only of the effect's own filter (the one [`asked_of`] names, the
/// same object, not one like it), any answer for each object, the same
/// each time. What the answer is for each filter a card prints is the
/// proofs above; read back from an effect on the heap, the filter was
/// matched as every filter there is, and those proofs ran out of memory at
/// 12 GB.
#[allow(dead_code)] // named only by `kani::stub`
pub(super) fn stand_in(_: &GameState, id: ObjectId, filter: &F, _: ObjectId, _: PlayerId, _: &CardRegistry) -> bool {
    // SAFETY: as in `asked_of`.
    assert!(std::ptr::eq(filter, unsafe { *std::ptr::addr_of!(ASKED_OF.value) }), "the effect's own filter");
    matches(id)
}

/// Whether object `id` matches the filter, as [`stand_in`] answers.
pub(super) fn matches(id: ObjectId) -> bool {
    let i = usize::try_from(id.0).expect("an id of the board");
    assert!((1..=4).contains(&i), "an object this proof does not make");
    // SAFETY: a proof runs on one thread, and only this function names MATCHED.
    let matched = unsafe { &mut *std::ptr::addr_of_mut!(MATCHED.value) };
    *matched[i - 1].get_or_insert_with(kani::any::<bool>)
}

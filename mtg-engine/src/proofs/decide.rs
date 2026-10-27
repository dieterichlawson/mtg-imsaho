//! Open answers to the engine's questions, for proving the code that asks
//! them on its own.
//!
//! A proof of a function that asks, say, "is this object a creature?"
//! replaces the function that answers with [`decided`]: Kani chooses the
//! answer, any answer, and gives the same one every time the same question
//! is asked about the same thing, as the real function would on a board
//! that does not change in between. What the proof then shows holds
//! whatever the answers are, so it holds for the real answers; that the
//! answering function gives the right ones is that function's own proof.

use crate::ids::ObjectId;

/// A question about an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    IsCreature,
    IsPlaneswalker,
    CanTargetPlayer,
    MatchesFilter,
    CanBeTargeted,
    IsValidTarget,
    HasHexproof,
    HasProtection,
}

/// The answers given so far: a fixed array and a count rather than a
/// `Vec`, since a push that may or may not reallocate, depending on how
/// many questions came before, is a reallocation CBMC follows down every
/// path.
const MAX_ANSWERS: usize = 16;
static mut ANSWERS: [Option<(Question, ObjectId, bool)>; MAX_ANSWERS] = [None; MAX_ANSWERS];

/// Kani's answer to `question` about `id`: chosen the first time it is
/// asked, the same after.
pub fn decided(question: Question, id: ObjectId) -> bool {
    // SAFETY: a proof runs on one thread, and only this module names ANSWERS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ANSWERS) };
    for slot in answers.iter_mut() {
        match slot {
            Some((q, i, a)) if *q == question && *i == id => return *a,
            Some(_) => {}
            None => {
                let a: bool = kani::any();
                *slot = Some((question, id, a));
                return a;
            }
        }
    }
    panic!("more questions than a proof here asks");
}

/// The object id a question about player `p` is filed under: one no game
/// here creates.
pub fn about_player(p: crate::ids::PlayerId) -> ObjectId {
    ObjectId(u64::MAX - u64::from(p.0))
}

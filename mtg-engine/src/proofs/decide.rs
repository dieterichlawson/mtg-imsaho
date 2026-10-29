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
    IsIndestructible,
    HasDeathtouch,
    HasLifelink,
}

/// The objects a question can be about: those a proof here makes, which a
/// game numbers from 1 up, and the two players (`about_player`).
const OBJECTS: usize = 32;
const IDS: usize = OBJECTS + 2;
const QUESTIONS: usize = Question::HasLifelink as usize + 1;

/// The answers given so far, one place for each question about each
/// object, found by the question and the object rather than looked for
/// among those asked before: which questions were asked before depends on
/// the answers, so a list of them is a different list down each path, and
/// where the paths meet every question after was compared with every entry
/// of every list (the proofs of targeting ran out of memory doing it).
static mut ANSWERS: [[Option<bool>; IDS]; QUESTIONS] = [[None; IDS]; QUESTIONS];

/// Where the answers about `id` are kept.
fn place(id: ObjectId) -> usize {
    match id.0 {
        n if n < OBJECTS as u64 => n as usize,
        n if n >= u64::MAX - 1 => OBJECTS + (u64::MAX - n) as usize,
        _ => panic!("a question about an object no proof here makes"),
    }
}

/// Kani's answer to `question` about `id`: chosen the first time it is
/// asked, the same after.
pub fn decided(question: Question, id: ObjectId) -> bool {
    // SAFETY: a proof runs on one thread, and only this module names ANSWERS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ANSWERS) };
    let answer = &mut answers[question as usize][place(id)];
    match *answer {
        Some(a) => a,
        None => {
            let a: bool = kani::any();
            *answer = Some(a);
            a
        }
    }
}

/// The object id a question about player `p` is filed under: one no game
/// here creates.
pub fn about_player(p: crate::ids::PlayerId) -> ObjectId {
    ObjectId(u64::MAX - u64::from(p.0))
}

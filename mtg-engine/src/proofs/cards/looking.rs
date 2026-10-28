//! Forbidden Alchemy, {2}{U} Instant: "Look at the top four cards of your
//! library. Put one of them into your hand and the rest into your
//! graveyard. Flashback {6}{B}"
//!
//! The card looks, and asks its controller which card to keep when there
//! is a choice to make; the answer is taken by the engine's handler for
//! that question (`engine::actions::choices`), proved here with it, since
//! nothing else asks it. Over a library of each size up to five cards (so
//! that one is past the fourth), a harness each, and every answer: each
//! card of the library, those not looked at among them. A library whose
//! length the solver left open was a list of every length, and every read
//! of the question and the answer a question of which.

use super::{acted, asked_for, board, card_proof, kept, were, Act, Snapshot};
use crate::actions::ResolvedChoice;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, ResolutionChoiceKind};
use crate::types::Zone;

use crate::cards::isd;

/// Forbidden Alchemy: with no card in its controller's library,
/// nothing; with one, that card into their hand; with more, they are
/// asked to choose one of the top four (all of them, if fewer), and on
/// an answer that is one of those, it goes to their hand and the rest
/// of those are milled into their graveyard, in order; an answer that
/// is not one of them is refused and the question stands. Over a
/// library of `n` cards.
fn forbidden_alchemy(n: u8) {
    let (registry, mut state, blank, card) = board(Box::new(isd::forbidden_alchemy::ForbiddenAlchemy));
    let mut library = Vec::with_capacity(5);
    for _ in 0..n {
        library.push(state.create_object(blank, PlayerId(0), Zone::Library, None, None));
    }
    state.players[0].library_order = library.clone();
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);
    let looked: Vec<ObjectId> = library.iter().take(4).copied().collect();
    match looked.len() {
        0 => {
            assert!(acted() == 0 && Snapshot::of(&state) == before);
        }
        1 => {
            assert!(asked_for(&[Act::Move(looked[0], Zone::Hand)]) && Snapshot::of(&state) == before);
        }
        _ => {
            let asks = |a: &Option<AwaitingAction>| matches!(a, Some(AwaitingAction::ResolutionChoice {
                player: PlayerId(0), source, choice: ResolutionChoiceKind::ChooseFromLookedAt { looked_at, .. },
            }) if *source == spell && *looked_at == looked);
            assert!(asks(&state.awaiting_action) && acted() == 0);
            let answer = library[usize::from(kani::any::<u8>() % n)];
            let _ = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenCard(answer), &registry);
            if looked.contains(&answer) {
                let rest = kept(&looked, |id| id != answer);
                assert!(acted() == 1 + rest.len() && were(0, 1, |_| Act::Move(answer, Zone::Hand))
                    && were(1, rest.len(), |i| Act::MillCard(rest[i])));
                assert!(state.awaiting_action.is_none());
            } else {
                assert!(acted() == 0 && asks(&state.awaiting_action));
            }
        }
    }
    std::mem::forget((registry, state, library, looked));
}

/// The proof above, a harness for each size of library.
macro_rules! for_each_size {
    ($($name:ident: $n:literal;)*) => {$(
        card_proof! {
            /// The proof above it, for one size of library.
            fn $name() {
                forbidden_alchemy($n);
            }
        }
    )*};
}

for_each_size! {
    forbidden_alchemy_over_an_empty_library: 0;
    forbidden_alchemy_over_a_library_of_one: 1;
    forbidden_alchemy_over_a_library_of_two: 2;
    forbidden_alchemy_over_a_library_of_three: 3;
    forbidden_alchemy_over_a_library_of_four: 4;
    forbidden_alchemy_over_a_library_of_five: 5;
}

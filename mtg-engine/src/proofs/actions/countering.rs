//! Countering a spell, which a card proof's recorder (`Act::Counter`)
//! stands in for: `helpers::counter_spell` and `counter_spell_exiling`
//! (CR 701.6a). The move off the stack is `move_object`'s, recorded here
//! (its own proof says what it does).

use crate::cards::helpers;
use crate::ids::PlayerId;
use crate::proofs::cards::{acted, asked_for, blank_board, Act, Snapshot};
use crate::state::StackEntry;
use crate::types::Zone;

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
/// "Counter target spell" (CR 701.6a): a spell on the stack is taken off
/// it, the rest of the stack left in its order, and put into its owner's
/// graveyard, or into exile if it was cast with flashback (702.34a) or the
/// counter says to exile it; the answer is that it was countered. A spell
/// no longer on the stack (it resolved, or was countered already) is left
/// as it was, and the answer is that nothing was. For either player's
/// spell, under another spell or not, through `counter_spell` and
/// `counter_spell_exiling`. Nothing else changes.
fn countering_takes_a_spell_off_the_stack_to_its_graveyard_or_exile() {
    let (registry, mut state, blank) = blank_board();
    let below = state.create_object(blank, PlayerId(0), Zone::Stack, None, None);
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let on: bool = kani::any();
    let spell = state.create_object(blank, owner, if on { Zone::Stack } else { Zone::Graveyard }, None, None);
    let flashback: bool = kani::any();
    state.get_object_mut(spell).expect("just created").cast_with_flashback = flashback;
    let under: bool = kani::any();
    if under {
        state.stack.push(StackEntry::Spell(below));
    }
    if on {
        state.stack.push(StackEntry::Spell(spell));
    }
    let exiling: bool = kani::any();
    let before = Snapshot::of(&state);

    let countered = if exiling {
        helpers::counter_spell_exiling(&mut state, spell, &registry)
    } else {
        helpers::counter_spell(&mut state, spell, &registry)
    };

    assert!(countered == on);
    if on {
        let to = if exiling || flashback { Zone::Exile } else { Zone::Graveyard };
        assert!(asked_for(&[Act::Move(spell, to)]));
    } else {
        assert!(acted() == 0);
    }
    assert!(state.stack.len() == usize::from(under) && state.stack.iter().all(|e| e.as_spell() == Some(below)));
    let expected = if on { before.with_stack(usize::from(under)) } else { before };
    assert!(Snapshot::of(&state) == expected);
    kani::cover!(on && flashback && !exiling && under, "a flashback spell countered, exiled");
    std::mem::forget((registry, state));
}

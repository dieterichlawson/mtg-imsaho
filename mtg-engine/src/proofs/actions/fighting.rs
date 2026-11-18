//! Fighting, which a card proof's `record_fight` stands in for:
//! `combat::fight` (CR 701.12). The damage is `queue_damage`'s and
//! `process_pending_damage`'s, recorded here (`proofs::actions::damage`
//! says what they do).

use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::PlayerId;
use crate::proofs::cards::{acted, asked_for, blank_board, Act, Snapshot};
use crate::types::Zone;

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::damage::queue_damage, crate::proofs::cards::record_queue)]
#[kani::stub(crate::damage::process_pending_damage, crate::proofs::cards::record_deal_queued)]
/// "Fights" (CR 701.12a): each of two creatures deals damage equal to its
/// power to the other, at once (queued, then dealt), a creature of power 0
/// or less dealing none; a creature that fights itself deals that damage
/// to itself twice (701.12c). If either is no longer on the battlefield or
/// no longer a creature, neither deals any (701.12b). For two creatures of
/// either player's, on the battlefield or not, a creature or not, of any
/// power from -2 to 5, and a creature fighting itself. Nothing else
/// changes.
fn fighting_deals_each_ones_power_to_the_other() {
    let (registry, mut state, blank) = blank_board();
    let mut make = || {
        let creature: bool = kani::any();
        let power = i32::from(kani::any::<i8>());
        kani::assume((-2..=5).contains(&power));
        let on: bool = kani::any();
        let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())),
            if on { Zone::Battlefield } else { Zone::Graveyard }, creature.then_some(power), creature.then_some(1));
        (id, on && creature, u32::try_from(power.max(0)).expect("small"))
    };
    let (a, a_fights, a_power) = make();
    let (b0, b0_fights, b0_power) = make();
    let itself: bool = kani::any();
    let (b, b_fights, b_power) = if itself { (a, a_fights, a_power) } else { (b0, b0_fights, b0_power) };
    let before = Snapshot::of(&state);

    crate::combat::fight(&mut state, a, b, &registry);

    if a_fights && b_fights {
        assert!(asked_for(&[
            Act::Queue { source: a, target: DamageTarget::Object(b), amount: a_power, kind: DamageKind::NonCombat },
            Act::Queue { source: b, target: DamageTarget::Object(a), amount: b_power, kind: DamageKind::NonCombat },
            Act::DealQueued,
        ]));
    } else {
        assert!(acted() == 0);
    }
    assert!(Snapshot::of(&state) == before);
    kani::cover!(a_fights && b_fights && itself && a_power > 0, "a creature fights itself");
    std::mem::forget((registry, state));
}

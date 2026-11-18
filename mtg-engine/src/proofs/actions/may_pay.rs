//! "You may pay {cost}" as a resolving ability asks it
//! (`cards::helpers::ask_may_pay`, Mentor of the Meek's and Screeching
//! Bat's), with whether the cost can be paid any answer: the tap plan is
//! `engine::payment_plan_with_sources`', whose own proofs are the
//! payment's (`proofs::mana_payment`, `proofs::actions::paying`).

use crate::cards::{helpers, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{new_game, Blank, Snapshot};
use crate::proofs::held::Held;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Zone};

/// Whom the plan was asked for, of which cost, and whether it was found.
static mut PLANNED: Held<Option<(PlayerId, usize, bool)>> = Held::new(None, (&raw const PLANNED).cast());

/// Stands in for `engine::payment_plan_with_sources`: a plan or none,
/// any answer, and what it was asked recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn any_plan(_: &GameState, player: PlayerId, cost: &ManaCost, _: &CardRegistry) -> Option<Vec<(ObjectId, usize)>> {
    let payable: bool = kani::any();
    // SAFETY: a proof runs on one thread, and only this function and the
    // harness name PLANNED.
    unsafe {
        *std::ptr::addr_of_mut!(PLANNED.value) = Some((player, std::ptr::from_ref(cost) as usize, payable));
    }
    payable.then(Vec::new)
}

/// Stands in for `helpers::format_tap_plan_names`: the plan's words are
/// for people, and none.
#[allow(dead_code)] // named only by `kani::stub`
fn no_names(_: &GameState, _: &[(ObjectId, usize)]) -> String {
    String::new()
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::engine::payment_plan_with_sources, any_plan)]
#[kani::stub(crate::cards::helpers::format_tap_plan_names, no_names)]
/// "You may pay": the player is asked yes or no, by the ability's
/// source, its words the ones made of the tap plan, exactly when the
/// cost can be paid; when it cannot, the "may" is a no, nothing is
/// asked and nothing changes (CR 117.3: no player can choose to pay a
/// cost they cannot pay). Whether it can be paid is asked of that
/// player and that cost, once. For either player, from a source of
/// theirs on the battlefield.
fn you_may_pay_is_asked_exactly_when_the_cost_can_be_paid() {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = new_game();
    let player = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(blank, player, Zone::Battlefield, Some(1), Some(1));
    let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
    let before = Snapshot::of(&state);
    let mut described = false;

    let asked = helpers::ask_may_pay(&mut state, player, source, &cost, &registry, |names| {
        described = names.is_empty();
        String::new()
    });

    // SAFETY: as in `any_plan`.
    let planned = unsafe { *std::ptr::addr_of!(PLANNED.value) };
    let (whom, of, payable) = planned.expect("the plan asked for");
    assert!(whom == player && of == std::ptr::from_ref(&cost) as usize);
    assert!(asked == payable && described == payable);
    if payable {
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player: p, source: s, choice: ResolutionChoiceKind::YesNo { description, source_card },
        }) if *p == player && *s == source && *source_card == source && description.is_empty()));
    } else {
        assert!(state.awaiting_action.is_none() && Snapshot::of(&state) == before);
    }
    std::mem::forget((registry, state, cost));
}

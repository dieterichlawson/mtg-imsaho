//! `engine::pay_cost_with_sources` and `payment_plan_with_sources`, which a
//! card proof's `record_pay` and `any_payment_plan` stand in for: a cost a
//! resolving effect lets a player pay ("you may pay {1}"), from floating
//! mana or by tapping what they control (CR 608.2g, 605.3a).
//!
//! Which mana a payment spends is `mana::auto_pay_reserving`'s, proved in
//! `mtg-kernel` for every pool, cost and reserve, and whether a pool can
//! pay a cost is `mana::can_pay`'s, proved there too: what is proved here
//! is what the engine does around them. The boards print nothing a player
//! could pay for besides (no hand, no ability with a cost), so nothing is
//! held in reserve.

use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::mana;
use crate::proofs::cards::{blank_board, board, Snapshot};
use crate::types::{Color, ManaCost, ManaPool, ManaSymbol, ManaType, Zone};

/// Any pool of up to two white, two green and two colorless mana.
fn any_pool() -> ManaPool {
    let mut pool = ManaPool::new();
    for t in [ManaType::White, ManaType::Green, ManaType::Colorless] {
        let n: u8 = kani::any();
        kani::assume(n <= 2);
        if n > 0 {
            pool.add(t, u32::from(n));
        }
    }
    pool
}

/// The costs the proofs pay, one per call: `{1}`, `{G}`, `{1}{G}`,
/// `{W}{W}`, `{C}` and `{G}{G}`.
fn a_cost(which: u8) -> ManaCost {
    ManaCost::new(match which {
        0 => vec![ManaSymbol::Generic(1)],
        1 => vec![ManaSymbol::Colored(Color::Green)],
        2 => vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Green)],
        3 => vec![ManaSymbol::Colored(Color::White), ManaSymbol::Colored(Color::White)],
        4 => vec![ManaSymbol::Colorless(1)],
        _ => vec![ManaSymbol::Colored(Color::Green), ManaSymbol::Colored(Color::Green)],
    })
}

/// What `pool` holds once `cost` is paid from it, nothing held in reserve.
fn spent(pool: &ManaPool, cost: &ManaCost) -> ManaPool {
    let mut left = pool.clone();
    assert!(mana::auto_pay_reserving(&mut left, cost, &ManaCost::free()).is_ok());
    left
}

/// With nothing to tap, a cost is paid exactly when the floating mana pays
/// it (CR 118.3): the plan is to tap nothing, the pool spends what paying
/// spends, and the answer is that it was paid. One the pool cannot pay is
/// refused, with no plan, and nothing changes. For each of six costs and
/// any pool of up to two of three types, either player's.
///
/// `who` pays cost `which` with nothing to tap: a permanent of each
/// player's that makes no mana.
fn from_the_pool(which: u8, who: PlayerId) {
    let (registry, mut state, blank) = blank_board();
    state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    state.create_object(blank, PlayerId(1), Zone::Battlefield, None, None);
    let cost = a_cost(which);
    let pool = any_pool();
    state.players[usize::from(who.0)].mana_pool = pool.clone();
    let before = Snapshot::of(&state);
    let can = mana::can_pay(&pool, &cost);

    let plan = crate::engine::payment_plan_with_sources(&state, who, &cost, &registry);
    assert!(plan == can.then(Vec::new));
    let paid = crate::engine::pay_cost_with_sources(&mut state, who, &cost, &registry);

    assert!(paid == can);
    let expected = if can { spent(&pool, &cost) } else { pool.clone() };
    assert!(state.players[usize::from(who.0)].mana_pool == expected);
    assert!(state.players[usize::from(1 - who.0)].mana_pool.is_empty());
    assert!(Snapshot::of(&state) == before);
    kani::cover!(paid && which == 2, "one and green paid from the pool");
    std::mem::forget((registry, state, cost, pool, expected, plan));
}


/// A cost the floating mana does not pay is paid by tapping an untapped
/// land of the player's, a land's mana not waiting on summoning sickness
/// (CR 302.6 is of creatures): the plan names the Forest exactly when the
/// pool falls short and its {G} makes up the difference; then the Forest
/// is tapped, its {G} added, announced in that order, and the cost paid
/// from the pool so made. A cost the pool pays alone taps nothing, and one
/// neither pays is refused with nothing changed. For each of the six
/// costs, any pool of up to two of three types, and either player.
///
/// `who` pays cost `which` with a Forest of theirs, untapped or not, and a
/// permanent of the other player's that makes no mana.
fn tapping_a_forest(which: u8, who: PlayerId) {
    let (registry, mut state, blank, forest_card) = board(Box::new(crate::cards::forest::Forest));
    let forest = state.create_object(forest_card, who, Zone::Battlefield, None, None);
    let untapped: bool = kani::any();
    let o = state.get_object_mut(forest).expect("just created");
    o.tapped = !untapped;
    o.summoning_sick = kani::any();
    state.create_object(blank, PlayerId(1 - who.0), Zone::Battlefield, None, None);
    let cost = a_cost(which);
    let pool = any_pool();
    state.players[usize::from(who.0)].mana_pool = pool.clone();
    let before = Snapshot::of(&state);

    // The pool alone, or with the Forest's {G} (CR 605.3a: a mana ability
    // may be activated while paying), and the Forest only when the pool
    // alone falls short.
    let mut with_forest = pool.clone();
    with_forest.add(ManaType::Green, 1);
    let alone = mana::can_pay(&pool, &cost);
    let tap = !alone && untapped && mana::can_pay(&with_forest, &cost);

    let plan = crate::engine::payment_plan_with_sources(&state, who, &cost, &registry);
    let expected_plan: Option<Vec<(ObjectId, usize)>> = if alone {
        Some(Vec::new())
    } else if tap {
        Some(vec![(forest, 0)])
    } else {
        None
    };
    assert!(plan == expected_plan);
    let paid = crate::engine::pay_cost_with_sources(&mut state, who, &cost, &registry);

    assert!(paid == (alone || tap));
    let expected = if alone {
        spent(&pool, &cost)
    } else if tap {
        spent(&with_forest, &cost)
    } else {
        pool.clone()
    };
    assert!(state.players[usize::from(who.0)].mana_pool == expected);
    assert!(state.players[usize::from(1 - who.0)].mana_pool.is_empty());
    if tap {
        assert!(matches!(state.events.as_slice(), [GameEvent::Tapped { object }, GameEvent::ManaAdded { player, mana_type: ManaType::Green, amount: 1 }]
            if *object == forest && *player == who));
        assert!(Snapshot::of(&state).without_events() == before.with_tapped(forest, true).without_events());
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    kani::cover!(tap && which == 2, "one and green paid with the Forest's green");
    std::mem::forget((registry, state, cost, pool, with_forest, expected, plan, expected_plan));
}

/// A proof above for each cost and each player paying, both constants in
/// each harness: a player index left to the solver blurs both players'
/// pools into one question, a cost left to it is a list of every cost at
/// once, and the cases of one harness add up.
macro_rules! each_cost {
    ($($name:ident: $proof:ident($which:literal, $who:literal);)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        /// The proof above it, for one cost and one player.
        fn $name() {
            $proof($which, PlayerId($who));
        }
    )*};
}

each_cost! {
    one_is_paid_from_the_pool_exactly_when_it_can_be_for_the_first_player: from_the_pool(0, 0);
    one_is_paid_from_the_pool_exactly_when_it_can_be_for_the_second_player: from_the_pool(0, 1);
    green_is_paid_from_the_pool_exactly_when_it_can_be_for_the_first_player: from_the_pool(1, 0);
    green_is_paid_from_the_pool_exactly_when_it_can_be_for_the_second_player: from_the_pool(1, 1);
    one_and_green_are_paid_from_the_pool_exactly_when_they_can_be_for_the_first_player: from_the_pool(2, 0);
    one_and_green_are_paid_from_the_pool_exactly_when_they_can_be_for_the_second_player: from_the_pool(2, 1);
    two_white_are_paid_from_the_pool_exactly_when_they_can_be_for_the_first_player: from_the_pool(3, 0);
    two_white_are_paid_from_the_pool_exactly_when_they_can_be_for_the_second_player: from_the_pool(3, 1);
    colorless_is_paid_from_the_pool_exactly_when_it_can_be_for_the_first_player: from_the_pool(4, 0);
    colorless_is_paid_from_the_pool_exactly_when_it_can_be_for_the_second_player: from_the_pool(4, 1);
    two_green_are_paid_from_the_pool_exactly_when_they_can_be_for_the_first_player: from_the_pool(5, 0);
    two_green_are_paid_from_the_pool_exactly_when_they_can_be_for_the_second_player: from_the_pool(5, 1);
    one_is_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_first_player: tapping_a_forest(0, 0);
    one_is_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_second_player: tapping_a_forest(0, 1);
    green_is_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_first_player: tapping_a_forest(1, 0);
    green_is_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_second_player: tapping_a_forest(1, 1);
    one_and_green_are_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_first_player: tapping_a_forest(2, 0);
    one_and_green_are_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_second_player: tapping_a_forest(2, 1);
    two_white_are_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_first_player: tapping_a_forest(3, 0);
    two_white_are_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_second_player: tapping_a_forest(3, 1);
    colorless_is_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_first_player: tapping_a_forest(4, 0);
    colorless_is_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_second_player: tapping_a_forest(4, 1);
    two_green_are_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_first_player: tapping_a_forest(5, 0);
    two_green_are_paid_by_tapping_a_forest_only_when_the_pool_falls_short_for_the_second_player: tapping_a_forest(5, 1);
}

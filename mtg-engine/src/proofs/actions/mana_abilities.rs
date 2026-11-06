//! A mana ability activated as a tap plan's entry is run
//! (`engine::activate_mana_source_reserving`, CR 605.3a), against what the
//! planner assumed of it: the step `mana::tap_step`, of `mana::pool_after`,
//! which every plan `mana::compute_autotap_around` returns has been run
//! through and found to pay. One harness per kind of source the card pool
//! has: a basic land, a dual's second colour, Sol Ring's two mana, a
//! utility land, Shimmering Grotto's free `{C}` and its filter paid out of
//! the pool around the cost (issue #252), a mana creature with and without
//! summoning sickness (CR 302.6), Deranged Assistant's mill (CR 118.3), and
//! a source already tapped.
//!
//! What each card's abilities are is the card proofs' (`proofs::cards::mana`),
//! and which mana a payment spends is `mana::auto_pay_reserving`'s, proved
//! in `mtg-kernel`. What is proved here is that the engine's activation,
//! on the ability as the planner is given it (`engine::available_mana_abilities`,
//! which `engine::gather_mana_sources` reads), leaves the pool as the step
//! does, taps the source, announces both and changes nothing else; and
//! that one the planner could not run changes nothing. Each pool and cost
//! is a constant of its harness, the pool afterward one the rules give,
//! written out: a generic cost paid out of a pool the solver chose ran the
//! payment proofs out of memory (`proofs::actions::paying`).

use crate::cards::isd::{avacyns_pilgrim::AvacynsPilgrim, clifftop_retreat::ClifftopRetreat,
    deranged_assistant::DerangedAssistant, nephalia_drownyard::NephaliaDrownyard,
    shimmering_grotto::ShimmeringGrotto};
use crate::cards::{plains::Plains, sol_ring::SolRing, CardBehavior};
use crate::events::GameEvent;
use crate::ids::PlayerId;
use crate::mana;
use crate::proofs::cards::{asked_for, board, Act, Snapshot};
use crate::types::{Color, ManaCost, ManaPool, ManaSymbol, ManaType, Zone};

use ManaSymbol::{Colored, Generic};
use ManaType::{Black as B, Blue as U, Colorless as C, Green as G, Red as R, White as W};

/// A pool holding exactly `held`.
fn pool_of(held: &[(ManaType, u32)]) -> ManaPool {
    let mut pool = ManaPool::new();
    for &(mana_type, amount) in held {
        pool.add(mana_type, amount);
    }
    pool
}

/// Whether `pool` holds exactly `held`, of every type. By amount, not by
/// `==`: a pool keeps a type it has spent to nothing as a zero, which
/// `==` tells from one it never held.
fn holds(pool: &ManaPool, held: &[(ManaType, u32)]) -> bool {
    [W, U, B, R, G, C].iter().all(|&t| {
        pool.get(t) == held.iter().filter(|&&(h, _)| h == t).map(|&(_, n)| n).sum::<u32>()
    })
}

/// `card` on the battlefield, controlled by `who`, tapped if
/// `tapped` and summoning sick if `sick`, a card in its controller's
/// library, and their pool holding `held`: its ability `index` activated
/// toward paying `cost`. `after` is what the pool holds afterward, or
/// `None` where the ability cannot be activated (a source tapped, a
/// creature's `{T}` waiting on summoning sickness, a filter's `{1}` the
/// pool cannot pay): then the planner was given no such ability, or its
/// step fails, and nothing changes. Otherwise the step leaves `after`, the
/// engine leaves the pool the step does, the source is tapped and its mana
/// announced, in that order, and nothing else changes but what the card itself does
/// (`mills`: Deranged Assistant mills a card, CR 701.17a).
#[allow(clippy::too_many_arguments)]
fn activating(
    who: PlayerId,
    card: Box<dyn CardBehavior>,
    index: usize,
    tapped: bool,
    sick: bool,
    held: &[(ManaType, u32)],
    cost: &[ManaSymbol],
    after: Option<&[(ManaType, u32)]>,
    mills: bool,
) {
    let (registry, mut state, blank, id) = board(card);
    let source = state.create_object(id, who, Zone::Battlefield, Some(1), Some(1));
    let o = state.get_object_mut(source).expect("just created");
    o.tapped = tapped;
    o.summoning_sick = sick;
    let card_in_library = state.create_object(blank, who, Zone::Library, None, None);
    state.players[usize::from(who.0)].library_order.push(card_in_library);
    let pool = pool_of(held);
    state.players[usize::from(who.0)].mana_pool = pool.clone();
    let cost = ManaCost::new(cost.to_vec());
    let before = Snapshot::of(&state);

    // The planner's step, on the ability as the planner is given it.
    let listed = crate::engine::available_mana_abilities(&state, source, &registry);
    let mut stepped = pool.clone();
    let planned = listed.iter()
        .find(|a| a.ability_index == index)
        .and_then(|a| mana::tap_step(&mut stepped, a, &cost));

    crate::engine::activate_mana_source_reserving(&mut state, source, index, Some(&cost), &registry);

    let left = &state.players[usize::from(who.0)].mana_pool;
    assert!(state.players[usize::from(1 - who.0)].mana_pool.is_empty());
    match after {
        Some(after) => {
            assert!(planned.is_some() && holds(&stepped, after));
            assert!(*left == stepped);
            assert!(matches!(state.events.as_slice(), [GameEvent::Tapped { object }, GameEvent::ManaAdded { player, .. }]
                if *object == source && *player == who));
            assert!(Snapshot::of(&state).without_events() == before.with_tapped(source, true).without_events());
            if mills {
                assert!(asked_for(&[Act::Mill(who, 1)]));
            } else {
                assert!(asked_for(&[]));
            }
        }
        None => {
            assert!(planned.is_none());
            assert!(*left == pool);
            assert!(Snapshot::of(&state) == before);
            assert!(asked_for(&[]));
        }
    }
    std::mem::forget((registry, state, pool, cost, listed, stepped));
}

/// The proofs of a mana ability activated, one per source and pool, each
/// for either player as its controller. With the controller the solver's,
/// each write to the controller's pool and library was a choice between
/// the two players' whole records, and the proofs ran out of memory at
/// 3 GB. Not `card_proof!`, which stands in for tapping and adding mana:
/// here they are the engine's own, but for the mill Deranged Assistant's
/// cost asks for, recorded.
macro_rules! each_activation_and_player {
    ($($name:ident: $card:expr, $index:expr, $tapped:expr, $sick:expr, $held:expr, $cost:expr => $after:expr, $mills:expr;)*) => {$(
        /// [`activating`] for the source, pool and cost of its name.
        mod $name {
            use super::*;

            #[kani::proof]
            #[kani::unwind(7)]
            #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
            #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
            #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
            #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
            #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
            #[kani::stub(crate::engine::mill_cards, crate::proofs::cards::record_mill)]
            /// Its controller the first player.
            fn for_the_first_player() {
                activating(PlayerId(0), Box::new($card), $index, $tapped, $sick, &$held, &$cost, $after, $mills);
            }

            #[kani::proof]
            #[kani::unwind(7)]
            #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
            #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
            #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
            #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
            #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
            #[kani::stub(crate::engine::mill_cards, crate::proofs::cards::record_mill)]
            /// Its controller the second player.
            fn for_the_second_player() {
                activating(PlayerId(1), Box::new($card), $index, $tapped, $sick, &$held, &$cost, $after, $mills);
            }
        }
    )*};
}

each_activation_and_player! {
    a_plains_adds_white_to_what_floats: Plains, 0, false, false, [(W, 1)], [Colored(Color::White), Colored(Color::White)]
        => Some(&[(W, 2)]), false;
    a_plains_already_tapped_adds_nothing: Plains, 0, true, false, [(W, 1)], [Colored(Color::White), Colored(Color::White)]
        => None, false;
    a_duals_second_ability_adds_its_second_colour: ClifftopRetreat, 1, false, false, [], [Colored(Color::White)]
        => Some(&[(W, 1)]), false;
    sol_ring_adds_two_colorless: SolRing, 0, false, false, [(G, 1)], [Generic(2), Colored(Color::Green)]
        => Some(&[(G, 1), (C, 2)]), false;
    a_utility_lands_mana_ability_adds_colorless: NephaliaDrownyard, 0, false, false, [], [Generic(1)]
        => Some(&[(C, 1)]), false;
    the_grottos_free_ability_adds_colorless: ShimmeringGrotto, 0, false, false, [], [Generic(1)]
        => Some(&[(C, 1)]), false;
    the_grottos_filter_spends_what_the_cost_does_not_need: ShimmeringGrotto, 4, false, false, [(W, 1), (G, 1)],
        [Colored(Color::Green), Colored(Color::Red)] => Some(&[(G, 1), (R, 1)]), false;
    the_grottos_filter_spends_the_costs_colour_when_nothing_else_floats: ShimmeringGrotto, 2, false, false, [(B, 1)],
        [Colored(Color::Black), Colored(Color::Blue)] => Some(&[(U, 1)]), false;
    the_grottos_filter_with_nothing_floating_adds_nothing: ShimmeringGrotto, 1, false, false, [], [Colored(Color::White)]
        => None, false;
    a_mana_creature_adds_its_mana: AvacynsPilgrim, 0, false, false, [], [Colored(Color::White)]
        => Some(&[(W, 1)]), false;
    a_summoning_sick_mana_creature_adds_nothing: AvacynsPilgrim, 0, false, true, [], [Colored(Color::White)]
        => None, false;
    deranged_assistant_adds_colorless_and_mills: DerangedAssistant, 0, false, false, [], [Generic(1)]
        => Some(&[(C, 1)]), true;
}

//! Every card with a mana ability (CR 605.1a): what each of its mana
//! abilities costs and what it adds, against its Oracle text.
//!
//! A basic land's ability is the one its land type gives it, "{T}: Add
//! {W}" for a Plains and so on (CR 305.6). The duals, "{T}: Add {R} or
//! {W}", are two abilities, one per colour, as a choice of ability is a
//! choice of mana (605.3b). Shimmering Grotto's "{1}, {T}: Add one mana of
//! any color" is five abilities, one per colour, each costing {1}. Deranged
//! Assistant's ability costs milling a card as well as tapping (CR 118.3:
//! a cost that cannot be paid cannot be activated), so it exists exactly
//! while its controller has a card in their library to mill.
//!
//! Each proof is of the card's abilities as a set: the same abilities, in
//! any order, with nothing more. Adding the mana is the engine's
//! (`engine::activate_mana_source`), the same for every source.

use super::{blank_board, card_proof};
use crate::cards::CardBehavior;
use crate::ids::PlayerId;
use crate::types::{ManaCost, ManaSymbol, ManaType, Zone};

/// One mana ability as a text prints it: its mana cost beyond {T}, and
/// what it adds.
type Ability = (u32, ManaType, u32);

/// `card`'s mana abilities, as a sorted list of what each costs (all of
/// them tap) and adds.
fn abilities_of(card: &dyn CardBehavior) -> Vec<Ability> {
    let (registry, mut state, _) = blank_board();
    let id = state.create_object(crate::ids::CardId(1), PlayerId(0), Zone::Battlefield, None, None);
    let mut out: Vec<Ability> = card.mana_abilities(&state, id).iter()
        .map(|a| {
            assert!(a.requires_tap, "every mana ability in the pool taps");
            let generic = a.cost.symbols.iter().map(|s| match s {
                ManaSymbol::Generic(n) => *n,
                other => panic!("a mana ability costing {other:?}"),
            }).sum();
            assert!(a.produced.len() == 1, "every mana ability in the pool adds one kind");
            (generic, a.produced[0].0, a.produced[0].1)
        })
        .collect();
    out.sort_by_key(|(g, t, n)| (*g, *t as u8, *n));
    std::mem::forget((registry, state));
    out
}

/// The abilities a text prints, sorted as [`abilities_of`] sorts.
fn printed(mut abilities: Vec<Ability>) -> Vec<Ability> {
    abilities.sort_by_key(|(g, t, n)| (*g, *t as u8, *n));
    abilities
}

macro_rules! mana {
    ($($name:ident: $card:expr => [$($ability:expr),* $(,)?]),* $(,)?) => {
        $(
            card_proof! {
                fn $name() {
                    assert!(abilities_of(&$card) == printed(vec![$($ability),*]));
                }
            }
        )*
    };
}

use crate::cards as core;
use crate::cards::isd as isd;
use ManaType::{Black as B, Blue as U, Colorless as C, Green as G, Red as R, White as W};

mana! {
    plains_taps_for_white: core::plains::Plains => [(0, W, 1)],
    island_taps_for_blue: core::island::Island => [(0, U, 1)],
    swamp_taps_for_black: core::swamp::Swamp => [(0, B, 1)],
    mountain_taps_for_red: core::mountain::Mountain => [(0, R, 1)],
    forest_taps_for_green: core::forest::Forest => [(0, G, 1)],
    sol_ring_taps_for_two_colorless: core::sol_ring::SolRing => [(0, C, 2)],
    avacyns_pilgrim_taps_for_white: isd::avacyns_pilgrim::AvacynsPilgrim => [(0, W, 1)],
    clifftop_retreat_taps_for_red_or_white: isd::clifftop_retreat::ClifftopRetreat => [(0, R, 1), (0, W, 1)],
    hinterland_harbor_taps_for_blue_or_green: isd::hinterland_harbor::HinterlandHarbor => [(0, U, 1), (0, G, 1)],
    isolated_chapel_taps_for_white_or_black: isd::isolated_chapel::IsolatedChapel => [(0, W, 1), (0, B, 1)],
    sulfur_falls_taps_for_blue_or_red: isd::sulfur_falls::SulfurFalls => [(0, U, 1), (0, R, 1)],
    woodland_cemetery_taps_for_black_or_green: isd::woodland_cemetery::WoodlandCemetery => [(0, B, 1), (0, G, 1)],
    gavony_township_taps_for_colorless: isd::gavony_township::GavonyTownship => [(0, C, 1)],
    ghost_quarter_taps_for_colorless: isd::ghost_quarter::GhostQuarter => [(0, C, 1)],
    kessig_wolf_run_taps_for_colorless: isd::kessig_wolf_run::KessigWolfRun => [(0, C, 1)],
    moorland_haunt_taps_for_colorless: isd::moorland_haunt::MoorlandHaunt => [(0, C, 1)],
    nephalia_drownyard_taps_for_colorless: isd::nephalia_drownyard::NephaliaDrownyard => [(0, C, 1)],
    stensia_bloodhall_taps_for_colorless: isd::stensia_bloodhall::StensiaBloodhall => [(0, C, 1)],
    shimmering_grotto_taps_for_colorless_or_filters_one_into_any_colour: isd::shimmering_grotto::ShimmeringGrotto
        => [(0, C, 1), (1, W, 1), (1, U, 1), (1, B, 1), (1, R, 1), (1, G, 1)],
}

card_proof! {
    /// Deranged Assistant: "{T}, Mill a card: Add {C}", which exists exactly
    /// while its controller has a card in their library to mill (CR 118.3,
    /// 701.17a), and says it does more than add mana; activated, its
    /// controller mills a card.
    fn deranged_assistant_taps_and_mills_for_colorless() {
        let (registry, mut state, blank) = blank_board();
        let controller = PlayerId(u8::from(kani::any::<bool>()));
        let id = state.create_object(blank, controller, Zone::Battlefield, None, None);
        let stocked: bool = kani::any();
        if stocked {
            let card = state.create_object(blank, controller, Zone::Library, None, None);
            if controller == PlayerId(0) {
                state.players[0].library_order.push(card);
            } else {
                state.players[1].library_order.push(card);
            }
        }
        let abilities = isd::deranged_assistant::DerangedAssistant.mana_abilities(&state, id);
        if stocked {
            assert!(abilities.len() == 1);
            let a = &abilities[0];
            assert!(a.requires_tap && a.cost == ManaCost::free() && a.produced == [(C, 1)] && a.has_side_effects);
        } else {
            assert!(abilities.is_empty());
        }
        // The mill, paid as the ability is activated: its controller's.
        isd::deranged_assistant::DerangedAssistant.on_activate_mana_ability(&mut state, id, 0, &registry);
        assert!(super::asked_for(&[super::Act::Mill(controller, 1)]));
        std::mem::forget((registry, state, abilities));
    }
}

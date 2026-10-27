//! The check-lands: "This land enters tapped unless you control a <land
//! type> or a <land type>." (CR 614.1d, 305.6)
//!
//! - Clifftop Retreat: a Mountain or a Plains.
//! - Hinterland Harbor: a Forest or an Island.
//! - Isolated Chapel: a Plains or a Swamp.
//! - Sulfur Falls: an Island or a Mountain.
//! - Woodland Cemetery: a Swamp or a Forest.
//!
//! Proved for each: offered its own entering, already tapped or not, under
//! its controller, the replacement makes it enter tapped exactly when it
//! would enter untapped and its controller controls no other permanent
//! with either land type (a permanent on the battlefield, that player's,
//! with the subtype; CR 205.3i, 110.2); offered anything else's entering,
//! it leaves it alone. The other permanent's types are any of the two the
//! land names and one it does not, its controller either player's, and it
//! on the battlefield or not. "You" is the land's controller as it enters,
//! which is its owner for every land in the pool: nothing here puts a land
//! onto the battlefield under another player's control.

use super::{blank_board, card_proof};
use crate::cards::CardBehavior;
use crate::ids::PlayerId;
use crate::replacement::{EnteringPermanent, ReplaceableEvent, Replacement};
use crate::types::Zone;

fn checks(card: &dyn CardBehavior, types: [&str; 2]) {
    let (registry, mut state, blank) = blank_board();
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let land = state.create_object(blank, controller, Zone::Hand, None, None);
    let other_controller = PlayerId(u8::from(kani::any::<bool>()));
    let on: bool = kani::any();
    let other = state.create_object(blank, other_controller, if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    // The two types the land names, and one it does not.
    let decoy = if types.contains(&"Swamp") { "Mountain" } else { "Swamp" };
    let names = [types[0], types[1], decoy];
    let o = state.get_object_mut(other).expect("just created");
    o.subtypes = Vec::with_capacity(3);
    for n in names {
        if kani::any() && !o.subtypes.iter().any(|s| s == n) {
            o.subtypes.push(n.to_string());
        }
    }
    let matches = on && other_controller == controller
        && state.get_object(other).expect("on the board").subtypes.iter().any(|s| s == types[0] || s == types[1]);
    let tapped: bool = kani::any();
    let entering = EnteringPermanent { object: land, from: Some(Zone::Hand), controller, tapped, counters: Vec::new(), copy_of: None };
    let replaced = card.replace_event(&mut state, land, &ReplaceableEvent::EntersBattlefield(entering.clone()), &registry);
    if !tapped && !matches {
        let Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e))) = &replaced else { panic!("enters tapped") };
        assert!(*e == EnteringPermanent { tapped: true, ..entering.clone() });
    } else {
        assert!(replaced.is_none());
    }
    // Anything else's entering it leaves alone.
    let someone = EnteringPermanent { object: other, ..entering };
    assert!(card.replace_event(&mut state, land, &ReplaceableEvent::EntersBattlefield(someone), &registry).is_none());
    kani::cover!(!tapped && matches, "enters untapped");
    std::mem::forget((registry, state, replaced));
}

macro_rules! check_land {
    ($($name:ident: $card:expr => $types:expr),* $(,)?) => {
        $(
            card_proof! {
                fn $name() {
                    checks(&$card, $types);
                }
            }
        )*
    };
}

use crate::cards::isd as isd;

check_land! {
    clifftop_retreat_enters_tapped_without_a_mountain_or_plains: isd::clifftop_retreat::ClifftopRetreat => ["Mountain", "Plains"],
    hinterland_harbor_enters_tapped_without_a_forest_or_island: isd::hinterland_harbor::HinterlandHarbor => ["Forest", "Island"],
    isolated_chapel_enters_tapped_without_a_plains_or_swamp: isd::isolated_chapel::IsolatedChapel => ["Plains", "Swamp"],
    sulfur_falls_enters_tapped_without_an_island_or_mountain: isd::sulfur_falls::SulfurFalls => ["Island", "Mountain"],
    woodland_cemetery_enters_tapped_without_a_swamp_or_forest: isd::woodland_cemetery::WoodlandCemetery => ["Swamp", "Forest"],
}

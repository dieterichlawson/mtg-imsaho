//! Creating tokens, which a card proof's `record_tokens` and `make_tokens`
//! stand in for: `GameState::create_tokens_with_subtypes` (CR 111.2,
//! 111.3, 111.4), with the count the replacement layer gives (614.1b).
//!
//! How many the replacement layer gives is any answer here, or that none
//! are made: which replacement says what is the replacing card's own proof
//! (Parallel Lives'). The board's cards print no replacement for entering,
//! so a token enters as it was made.

use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::proofs::cards::{blank_board, Snapshot};
use crate::replacement::ReplaceableEvent;
use crate::state::{GameState, TokenFace};
use crate::types::{CardType, Color, Keyword, Zone};

/// What creating tokens asked the replacement layer, and its answer.
static mut ASKED: Option<(PlayerId, u32, Option<u32>)> = None;

/// Stands in for `replacement::apply`: creating tokens asks it how many are
/// made, and the answer is any count up to two, or that the event is
/// replaced and none are.
#[allow(dead_code)] // named only by `kani::stub`
fn any_count(_: &mut GameState, event: ReplaceableEvent, _: &CardRegistry) -> Option<ReplaceableEvent> {
    let ReplaceableEvent::CreatesTokens { controller, count } = event else {
        panic!("creating tokens asks about creating tokens");
    };
    let answer = kani::any::<bool>().then(|| {
        let m: u32 = kani::any();
        kani::assume(m <= 2);
        m
    });
    // SAFETY: a proof runs on one thread, and only this module names it.
    unsafe {
        *std::ptr::addr_of_mut!(ASKED) = Some((controller, count, answer));
    }
    answer.map(|count| ReplaceableEvent::CreatesTokens { controller, count })
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::replacement::apply, any_count)]
/// "Create N P/T [colour] [subtype] [type] tokens [with keyword]" (CR
/// 111.2): creating none asks nothing and makes nothing; otherwise the
/// replacement layer is asked once about all N, as one event (614.1b), and
/// as many as it answers are made, none if it replaces the event. Each is
/// a token of no card, on the battlefield under the player who created it
/// (111.2), summoning sick (302.6), with exactly the power, toughness,
/// colours, types, subtypes and keywords the effect gives it (111.3) as its
/// printed characteristics, and named as the effect names it or, if it
/// does not, after its subtypes (111.4); each one's entering is announced,
/// in order, and what comes back is the new tokens, in order. For either
/// player, N up to two, any power and toughness an `i8` holds, and with or
/// without a keyword, a subtype and a name. Nothing else changes.
fn creating_tokens_makes_as_many_as_the_replacements_say_with_those_characteristics() {
    let (registry, mut state, blank) = blank_board();
    let there = state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let n: u32 = kani::any();
    kani::assume(n <= 2);
    let (power, toughness) = (i32::from(kani::any::<i8>()), i32::from(kani::any::<i8>()));
    let (flying, zombie, named): (bool, bool, bool) = (kani::any(), kani::any(), kani::any());
    let keywords = if flying { vec![Keyword::Flying] } else { vec![] };
    let subtypes: Vec<String> = if zombie { vec!["Zombie".into()] } else { vec![] };
    let name = if named { "Named" } else { "" };
    let before = Snapshot::of(&state);

    let ids = state.create_tokens_with_subtypes(n, name, owner, power, toughness,
        vec![Color::Black], vec![CardType::Creature], keywords.clone(), subtypes.clone(), &registry);

    // SAFETY: as in `any_count`.
    let asked = unsafe { *std::ptr::addr_of!(ASKED) };
    if n == 0 {
        assert!(asked.is_none() && ids.is_empty() && Snapshot::of(&state) == before);
        std::mem::forget((registry, state, ids, keywords, subtypes));
        return;
    }
    let Some((controller, count, answer)) = asked else { panic!("asked how many") };
    assert!(controller == owner && count == n);
    let made = answer.unwrap_or(0) as usize;
    assert!(ids.len() == made && (0..made).all(|i| ids[i] == ObjectId(there.0 + 1 + i as u64)));
    let named_as = if named { "Named" } else if zombie { "Zombie" } else { "" };
    for id in &ids {
        let o = state.get_object(*id).expect("made");
        assert!(o.is_token && o.card_id == CardId(0) && o.zone == Zone::Battlefield && o.zone_change_count == 0);
        assert!(o.owner == owner && o.controller == owner && o.base_controller == owner && o.summoning_sick);
        assert!(o.power == Some(power) && o.toughness == Some(toughness) && o.name == named_as);
        assert!(o.colors == [Color::Black] && o.card_types == [CardType::Creature] && o.keywords == keywords && o.subtypes == subtypes);
        assert!(o.token_face.as_ref().is_some_and(|f| *f == TokenFace {
            name: named_as.into(), power: Some(power), toughness: Some(toughness), colors: vec![Color::Black],
            keywords: keywords.clone(), card_types: vec![CardType::Creature], subtypes: subtypes.clone(),
        }));
    }
    assert!(state.events.len() == made && (0..made).all(|i| matches!(state.events[i],
        GameEvent::EnteredBattlefield { object, controller } if object == ids[i] && controller == owner)));
    assert!(Snapshot::of(&state).without_events() == before.with_new_permanents(&ids, owner).without_events());
    kani::cover!(made == 2 && n == 1, "doubled");
    kani::cover!(answer.is_none(), "replaced");
    std::mem::forget((registry, state, ids, keywords, subtypes));
}

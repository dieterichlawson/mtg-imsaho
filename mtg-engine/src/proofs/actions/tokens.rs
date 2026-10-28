//! Creating tokens, which a card proof's `record_tokens`, `make_tokens`,
//! `record_token_copy` and `record_defined_token` stand in for:
//! `GameState::create_tokens_with_subtypes` (CR 111.2, 111.3, 111.4),
//! `create_token_copy` (707.2) and `create_token_with_defined_pt` (604.3),
//! each with the count the replacement layer gives (614.1b).
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
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
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

/// A 2/2 green Bear creature, {1}{G}.
struct Bear;

impl crate::cards::CardBehavior for Bear {
    fn card_data(&self) -> crate::cards::CardData {
        crate::cards::CardData {
            name: "Bear".into(),
            cost: Some(crate::types::ManaCost::new(vec![crate::types::ManaSymbol::Generic(1), crate::types::ManaSymbol::Colored(Color::Green)])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Bear".into()],
            power: Some(2),
            toughness: Some(2),
            ..crate::cards::CardData::default()
        }
    }
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::replacement::apply, any_count)]
/// "Create a token that's a copy of target creature" (CR 707.2, 111.10):
/// the token has the copied permanent's copiable values, what is printed
/// on it, and nothing an effect gave it, its counters or its status: a
/// Bear card that an effect has made a black Vampire, and that is tapped,
/// damaged and has a counter, is copied as an untapped, undamaged 2/2
/// green Bear creature named Bear with no counters, of the Bear card (so
/// that its abilities are the Bear's); a token is copied as it was made
/// (its face), not as effects have since made it. The copy is under the
/// player who made it, as many copies as the replacement layer answers
/// (none if it replaces the event), and the first is what comes back.
/// Nothing else changes.
fn a_token_copy_has_the_copiable_values_and_nothing_else() {
    let mut registry = CardRegistry::new();
    let bear = registry.register(Box::new(Bear));
    let mut state = crate::proofs::cards::new_game();
    let token: bool = kani::any();
    let from = state.create_object(if token { CardId(0) } else { bear }, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(2), Some(2));
    let o = state.get_object_mut(from).expect("just created");
    if token {
        o.is_token = true;
        o.name = "Zombie".into();
        o.token_face = Some(TokenFace {
            name: "Zombie".into(), power: Some(2), toughness: Some(2), colors: vec![Color::Black],
            keywords: vec![], card_types: vec![CardType::Creature], subtypes: vec!["Zombie".into()],
        });
    }
    // What effects and the game gave it, which a copy does not copy.
    o.subtypes = vec!["Vampire".into()];
    o.colors = vec![Color::Black, Color::Red];
    o.keywords = vec![Keyword::Flying];
    o.tapped = true;
    o.damage_marked = 1;
    o.counters.insert(crate::types::CounterType::PlusOnePlusOne, 1);
    if token {
        o.subtypes.push("Zombie".into());
        o.card_types = vec![CardType::Creature];
    }
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let before = Snapshot::of(&state);

    let first = state.create_token_copy(from, owner, &registry);

    // SAFETY: as in `any_count`.
    let Some((controller, 1, answer)) = (unsafe { *std::ptr::addr_of!(ASKED) }) else { panic!("asked about one token") };
    assert!(controller == owner);
    let made = answer.unwrap_or(0) as usize;
    let ids: [ObjectId; 2] = [ObjectId(from.0 + 1), ObjectId(from.0 + 2)];
    assert!(first == if made == 0 { ObjectId(0) } else { ids[0] });
    let (name, colour, subtype, card) = if token { ("Zombie", Color::Black, "Zombie", CardId(0)) } else { ("Bear", Color::Green, "Bear", bear) };
    for id in &ids[..made] {
        let t = state.get_object(*id).expect("made");
        assert!(t.is_token && t.card_id == card && t.zone == Zone::Battlefield && t.controller == owner && t.owner == owner);
        assert!(t.name == name && t.power == Some(2) && t.toughness == Some(2));
        assert!(t.colors == [colour] && t.card_types == [CardType::Creature] && t.subtypes.len() == 1 && t.subtypes[0] == subtype);
        assert!(t.keywords.is_empty() && !t.is_legendary && !t.is_transformed);
    }
    assert!(Snapshot::of(&state).without_events() == before.with_new_permanents(&ids[..made], owner).without_events());
    kani::cover!(made == 1 && !token, "a Bear copied");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::replacement::apply, any_count)]
/// "Create an X/X token with 'This creature's power and toughness are each
/// equal to ...'" (CR 604.3, Gutter Grime's Ooze): as many tokens as the
/// replacement layer answers, each 0/0 without the ability and carrying,
/// from the moment it is made, which permanent's ability defines its power
/// and toughness, with the colours, types and subtypes given and named for
/// its subtypes (111.4), under the player who made it. Nothing else
/// changes.
fn a_token_whose_ability_defines_its_size_knows_whose_it_is() {
    let (registry, mut state, blank) = blank_board();
    let source = state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let before = Snapshot::of(&state);

    let ids = state.create_token_with_defined_pt(source, "", owner, vec![Color::Green], vec![CardType::Creature], vec![], vec!["Ooze".into()], &registry);

    // SAFETY: as in `any_count`.
    let Some((controller, 1, answer)) = (unsafe { *std::ptr::addr_of!(ASKED) }) else { panic!("asked about one token") };
    assert!(controller == owner);
    let made = answer.unwrap_or(0) as usize;
    assert!(ids.len() == made && (0..made).all(|i| ids[i] == ObjectId(source.0 + 1 + i as u64)));
    for id in &ids {
        let t = state.get_object(*id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == owner && t.name == "Ooze");
        assert!(t.power == Some(0) && t.toughness == Some(0) && t.colors == [Color::Green] && t.card_types == [CardType::Creature]);
        assert!(t.card_state.get(crate::cards::PT_DEFINED_BY) == Some(&source) && t.card_state.len() == 1);
    }
    assert!(Snapshot::of(&state).without_events() == before.with_new_permanents(&ids, owner).without_events());
    std::mem::forget((registry, state, ids));
}

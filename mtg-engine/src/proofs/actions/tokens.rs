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

/// What the replacement layer answers in this proof, set by its harness
/// before any token is made: a count, or that the event is replaced.
static mut ANSWER: Option<u32> = None;

/// Stands in for `replacement::apply`: creating tokens asks it how many are
/// made, and the answer is the harness's, a count up to two or that the
/// event is replaced and none are (`each_answer` says why it is a constant).
#[allow(dead_code)] // named only by `kani::stub`
fn the_answer(_: &mut GameState, event: ReplaceableEvent, _: &CardRegistry) -> Option<ReplaceableEvent> {
    let ReplaceableEvent::CreatesTokens { controller, count } = event else {
        panic!("creating tokens asks about creating tokens");
    };
    // SAFETY: a proof runs on one thread, and only this module names these.
    let answer = unsafe { *std::ptr::addr_of!(ANSWER) };
    unsafe {
        *std::ptr::addr_of_mut!(ASKED) = Some((controller, count, answer));
    }
    answer.map(|count| ReplaceableEvent::CreatesTokens { controller, count })
}

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
/// player and any power and toughness an `i8` holds; N of one or two, a
/// Zombie or of no subtype, named or not, and flying or of no keyword, as
/// the harness says (N of none is `creating_no_tokens_asks_and_makes_nothing`).
/// Nothing else changes.
fn creating_tokens(n: u32, zombie: bool, named: bool, flying: bool) {
    let (registry, mut state, blank) = blank_board();
    let there = state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = (i32::from(kani::any::<i8>()), i32::from(kani::any::<i8>()));
    let keywords = if flying { vec![Keyword::Flying] } else { vec![] };
    let subtypes: Vec<String> = if zombie { vec!["Zombie".into()] } else { vec![] };
    let name = if named { "Named" } else { "" };
    let before = Snapshot::of(&state);

    let ids = state.create_tokens_with_subtypes(n, name, owner, power, toughness,
        vec![Color::Black], vec![CardType::Creature], keywords.clone(), subtypes.clone(), &registry);

    // SAFETY: as in `the_answer`.
    let Some((controller, count, answer)) = (unsafe { *std::ptr::addr_of!(ASKED) }) else { panic!("asked how many") };
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
        // Field by field, every field of the face: a face built to compare
        // with was strings and lists built and cloned to compare.
        assert!(o.token_face.as_ref().is_some_and(|f| {
            let TokenFace { name, power: p, toughness: t, colors, keywords: k, card_types, subtypes: st } = f;
            name == named_as && *p == Some(power) && *t == Some(toughness) && *colors == [Color::Black]
                && *k == keywords && *card_types == [CardType::Creature] && *st == subtypes
        }));
    }
    assert!(state.events.len() == made && (0..made).all(|i| matches!(state.events[i],
        GameEvent::EnteredBattlefield { object, controller } if object == ids[i] && controller == owner)));
    assert!(Snapshot::of(&state).without_events() == before.with_new_permanents(&ids, owner).without_events());
    kani::cover!(made == 2 && n == 1, "doubled");
    kani::cover!(answer.is_none(), "replaced");
    std::mem::forget((registry, state, ids, keywords, subtypes));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::replacement::apply, the_answer)]
/// Creating no tokens asks the replacement layer nothing, makes nothing
/// and changes nothing, whatever the tokens would have been: for either
/// player, any power and toughness an `i8` holds, a Zombie or of no
/// subtype, named or not, and flying or of no keyword. N is the constant
/// the other token proofs leave out (`each_answer` says why it is one).
fn creating_no_tokens_asks_and_makes_nothing() {
    let (registry, mut state, blank) = blank_board();
    state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = (i32::from(kani::any::<i8>()), i32::from(kani::any::<i8>()));
    let keywords = if kani::any() { vec![Keyword::Flying] } else { vec![] };
    let subtypes: Vec<String> = if kani::any() { vec!["Zombie".into()] } else { vec![] };
    let name = if kani::any() { "Named" } else { "" };
    let before = Snapshot::of(&state);

    let ids = state.create_tokens_with_subtypes(0, name, owner, power, toughness,
        vec![Color::Black], vec![CardType::Creature], keywords, subtypes, &registry);

    // SAFETY: as in `the_answer`.
    let asked = unsafe { *std::ptr::addr_of!(ASKED) };
    assert!(asked.is_none() && ids.is_empty() && Snapshot::of(&state) == before);
    std::mem::forget((registry, state, ids));
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
/// Nothing else changes. Of the Bear, or if `token` of the Zombie token.
fn copying(token: bool) {
    let mut registry = CardRegistry::new();
    let bear = registry.register(Box::new(Bear));
    let mut state = crate::proofs::cards::new_game();
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
    // What effects and the game gave it, which a copy does not copy (the
    // list built whole: a list grown stands in as a failure, `no_growing`).
    o.subtypes = if token { vec!["Vampire".into(), "Zombie".into()] } else { vec!["Vampire".into()] };
    o.colors = vec![Color::Black, Color::Red];
    o.keywords = vec![Keyword::Flying];
    o.tapped = true;
    o.damage_marked = 1;
    o.counters.insert(crate::types::CounterType::PlusOnePlusOne, 1);
    if token {
        o.card_types = vec![CardType::Creature];
    }
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let before = Snapshot::of(&state);

    let first = state.create_token_copy(from, owner, &registry);

    // SAFETY: as in `the_answer`.
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

/// "Create an X/X token with 'This creature's power and toughness are each
/// equal to ...'" (CR 604.3, Gutter Grime's Ooze): as many tokens as the
/// replacement layer answers, each 0/0 without the ability and carrying,
/// from the moment it is made, which permanent's ability defines its power
/// and toughness, with the colours, types and subtypes given and named for
/// its subtypes (111.4), under the player who made it. Nothing else
/// changes.
fn defining_its_size() {
    let (registry, mut state, blank) = blank_board();
    let source = state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let before = Snapshot::of(&state);

    let ids = state.create_token_with_defined_pt(source, "", owner, vec![Color::Green], vec![CardType::Creature], vec![], vec!["Ooze".into()], &registry);

    // SAFETY: as in `the_answer`.
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

/// Each proof above, for each answer the replacement layer gives, and each
/// token proof for each count asked for, name, subtype and keyword, each a
/// constant. An answer the solver chose was a count of any size to
/// symbolic execution, the assumption that it is at most two
/// notwithstanding (an assumption narrows the solver's choice, not the
/// walk), and the copies made in a loop that many times were walked to the
/// loop's bound; a count asked for that could be none returned an empty
/// list down that branch, and where it met the others the list made was of
/// either length to every walk of it; a keyword list of either length was,
/// in the same way, a list of any length to every clone and comparison of
/// it; and a name and a subtype list the solver chose were strings of
/// either length to compare with every token's, byte by byte. Left open,
/// they ran out of memory; the count asked for, branched on inside one
/// harness, still did.
macro_rules! each_answer {
    ($($name:ident: $proof:ident $args:tt;)*) => {$(
        mod $name {
            of_each_answer!($proof $args; when_they_replace_it: None, when_they_say_none: Some(0), when_they_say_one: Some(1), when_they_say_two: Some(2));
        }
    )*};
}

/// The harnesses of one row of `each_answer`, one for each answer.
macro_rules! of_each_answer {
    ($proof:ident $args:tt; $($fn:ident: $answer:expr),*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::replacement::apply, super::the_answer)]
        /// The proof named above it, for one answer.
        fn $fn() {
            // SAFETY: as in `the_answer`.
            unsafe { *std::ptr::addr_of_mut!(super::ANSWER) = $answer };
            super::$proof $args;
        }
    )*};
}

each_answer! {
    creating_one_unnamed_token_of_no_subtype_makes_as_many_as_the_replacements_say: creating_tokens(1, false, false, false);
    creating_one_unnamed_flying_token_of_no_subtype_makes_as_many_as_the_replacements_say: creating_tokens(1, false, false, true);
    creating_one_named_token_of_no_subtype_makes_as_many_as_the_replacements_say: creating_tokens(1, false, true, false);
    creating_one_named_flying_token_of_no_subtype_makes_as_many_as_the_replacements_say: creating_tokens(1, false, true, true);
    creating_one_unnamed_zombie_token_makes_as_many_as_the_replacements_say: creating_tokens(1, true, false, false);
    creating_one_unnamed_flying_zombie_token_makes_as_many_as_the_replacements_say: creating_tokens(1, true, false, true);
    creating_one_named_zombie_token_makes_as_many_as_the_replacements_say: creating_tokens(1, true, true, false);
    creating_one_named_flying_zombie_token_makes_as_many_as_the_replacements_say: creating_tokens(1, true, true, true);
    creating_two_unnamed_tokens_of_no_subtype_makes_as_many_as_the_replacements_say: creating_tokens(2, false, false, false);
    creating_two_unnamed_flying_tokens_of_no_subtype_makes_as_many_as_the_replacements_say: creating_tokens(2, false, false, true);
    creating_two_named_tokens_of_no_subtype_makes_as_many_as_the_replacements_say: creating_tokens(2, false, true, false);
    creating_two_named_flying_tokens_of_no_subtype_makes_as_many_as_the_replacements_say: creating_tokens(2, false, true, true);
    creating_two_unnamed_zombie_tokens_makes_as_many_as_the_replacements_say: creating_tokens(2, true, false, false);
    creating_two_unnamed_flying_zombie_tokens_makes_as_many_as_the_replacements_say: creating_tokens(2, true, false, true);
    creating_two_named_zombie_tokens_makes_as_many_as_the_replacements_say: creating_tokens(2, true, true, false);
    creating_two_named_flying_zombie_tokens_makes_as_many_as_the_replacements_say: creating_tokens(2, true, true, true);
    a_token_copy_of_a_card_has_the_copiable_values_and_nothing_else: copying(false);
    a_token_copy_of_a_token_has_its_face_and_nothing_else: copying(true);
    a_token_whose_ability_defines_its_size_knows_whose_it_is: defining_its_size();
}

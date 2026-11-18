//! A card resolving with the engine itself under it: no recorder stands in
//! for an engine action here. The card's own hook runs, and under it the
//! engine's own destruction, zone change, damage, drawing, token making
//! and continuous effects; what is asserted is the board afterwards, read
//! back through the engine's own characteristics, against the card's text.
//! `docs/plans/cards-in-the-loop.md`, B2, has the measurements these were
//! shaped by.
//!
//! What every proof still takes out: `format!` and the game log, which no
//! rule reads, and growing and freeing an allocation (`proofs::cards`
//! says why of each).
//!
//! What a proof here does not cover: the spell's trip through the stack
//! (its targets re-checked, the card put where it goes after), which is
//! the engine's and proved of it (`proofs::actions::resolving`); with the
//! engine's own code under it that step ran out of memory at 12 GB even
//! on a board with nothing open. Every card through cast, stack,
//! resolution, state-based actions and triggers together, the engine's
//! own, is `tests/card_semantics.rs` on every small board.
//!
//! What keeps one in reach, each measured: a player the solver leaves open
//! is a library or a hand it leaves open, so a card that draws has a
//! harness for each player (open, Think Twice ran out of memory at 12 GB;
//! fixed, it is under a minute and a gigabyte); and a list the solver may
//! or may not have pushed to is one of either length to every scan after,
//! so an effect that may be there is always pushed, as one kind or
//! another.

use crate::actions::Target;
use crate::ids::PlayerId;
use crate::types::{CardType, Color, Keyword, Zone};

/// What of the target Doom Blade destroys is left to the solver.
#[derive(Clone, Copy, Default)]
struct Open {
    tapped: bool,
    controller: bool,
    damage: bool,
    toughness: bool,
    shield: bool,
    indestructible: bool,
}

/// Doom Blade ("Destroy target nonblack creature.") resolving at a
/// nonblack creature on the battlefield, the engine's own destruction and
/// zone change under it: the creature is put into its owner's graveyard
/// (CR 701.7a, 400.3), unless it is indestructible and nothing happens to
/// it (702.12b), or it has a regeneration shield and is tapped, its
/// damage removed and the shield used up instead (701.15a).
fn doom_blade(open: Open) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::doom_blade::DoomBlade));
    let owner = PlayerId(1);
    let target = state.create_object(blank, owner, Zone::Battlefield, Some(2), Some(2));
    {
        let o = state.get_object_mut(target).expect("just created");
        o.card_types = vec![CardType::Creature];
        if open.tapped {
            o.tapped = kani::any();
        }
        if open.controller {
            o.controller = PlayerId(u8::from(kani::any::<bool>()));
        }
        if open.damage {
            o.damage_marked = u32::from(kani::any::<bool>());
        }
        if open.toughness {
            let t: u8 = kani::any();
            kani::assume((2..=3).contains(&t));
            o.toughness = Some(i32::from(t));
        }
        if open.shield {
            o.regeneration_shields = u32::from(kani::any::<bool>());
        }
    }
    // Indestructible as an effect gives it until end of turn: a card's
    // printed keywords are its card's, and the blank card prints none. One
    // effect either way, of indestructible or of flying: a list the solver
    // may or may not have pushed to is one of either length to every scan
    // of it after.
    let indestructible = open.indestructible && kani::any();
    if open.indestructible {
        let keyword = if indestructible { Keyword::Indestructible } else { Keyword::Flying };
        state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantKeyword { target, keyword });
    }
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = state.get_object(target).expect("just created");
    let (shield, tapped, damage) = (before.regeneration_shields > 0, before.tapped, before.damage_marked);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    if indestructible {
        assert!(o.zone == Zone::Battlefield && o.regeneration_shields == u32::from(shield) && o.tapped == tapped && o.damage_marked == damage);
    } else if shield {
        assert!(o.zone == Zone::Battlefield && o.tapped && o.damage_marked == 0 && o.regeneration_shields == 0);
    } else {
        assert!(o.zone == Zone::Graveyard && o.controller == owner && !o.tapped && o.damage_marked == 0);
        assert!(state.players[1].graveyard_order.contains(&target));
    }
    kani::cover!(o.zone == Zone::Graveyard, "destroyed");
    std::mem::forget((registry, state));
}

/// A creature card of the blank card on the battlefield under either
/// player, a 2/2 or a 2/3, tapped or not, with a point of damage or none:
/// what a spell here targets.
fn any_creature(state: &mut crate::state::GameState, blank: crate::ids::CardId) -> crate::ids::ObjectId {
    let id = state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    let o = state.get_object_mut(id).expect("just created");
    o.card_types = vec![CardType::Creature];
    o.controller = PlayerId(u8::from(kani::any::<bool>()));
    o.tapped = kani::any();
    o.damage_marked = u32::from(kani::any::<bool>());
    if kani::any() {
        o.toughness = Some(3);
    }
    id
}

/// Giant Growth ("Target creature gets +3/+3 until end of turn.")
/// resolving at a creature, read back through the engine's own power and
/// toughness (CR 613.4c): three more of each than before, and nothing else
/// about the creature changed.
fn giant_growth(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::giant_growth::GiantGrowth));
    let target = any_creature(&mut state, blank);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    assert!(state.effective_power(target, &registry) == power.map(|p| p + 3));
    assert!(state.effective_toughness(target, &registry) == toughness.map(|t| t + 3));
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    kani::cover!(toughness == Some(3), "a 2/3");
    std::mem::forget((registry, state));
}

/// Think Twice ("Draw a card.") resolving for the player `you` names
/// (each in a harness of its own: a player the solver leaves open is a
/// library it leaves open, and the draw ran out of memory), each player
/// with a library of `cards` cards: its controller's top card is in their hand
/// and no longer in their library (CR 121.1), or with none to draw they
/// have drawn from an empty library (704.5b); the other player's library
/// and hand are as they were.
fn think_twice((cards, you): (usize, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::think_twice::ThinkTwice));
    let mut tops = [None, None];
    for p in 0..2u8 {
        for k in 0..cards {
            let id = state.create_object(blank, PlayerId(p), Zone::Library, None, None);
            if p == 0 {
                state.players[0].library_order.push(id);
            } else {
                state.players[1].library_order.push(id);
            }
            if k == 0 {
                tops[p as usize] = Some(id);
            }
        }
    }
    let spell = state.create_object(card, PlayerId(u8::from(you)), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    let (mine, theirs) = if you { (1, 0) } else { (0, 1) };
    assert!(state.players[theirs].library_order.len() == cards && !state.players[theirs].has_drawn_from_empty);
    if let Some(top) = tops[theirs] {
        assert!(state.get_object(top).expect("still an object").zone == Zone::Library);
    }
    match tops[mine] {
        Some(top) => {
            let o = state.get_object(top).expect("still an object");
            assert!(o.zone == Zone::Hand && o.controller == PlayerId(u8::from(you)));
            assert!(state.players[mine].library_order.len() == cards - 1 && !state.players[mine].library_order.contains(&top));
            assert!(!state.players[mine].has_drawn_from_empty);
        }
        None => {
            assert!(state.players[mine].has_drawn_from_empty && state.players[mine].library_order.is_empty());
        }
    }
    std::mem::forget((registry, state));
}

/// Midnight Haunting ("Create two 1/1 white Spirit creature tokens with
/// flying.") resolving for either player on a board with nothing that
/// changes how tokens are made: two new permanents, each a token, a 1/1
/// white Spirit creature with flying, under the spell's controller (CR
/// 111.1, 111.2), read back through the engine's own characteristics.
fn midnight_haunting(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::midnight_haunting::MidnightHaunting));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let spell = state.create_object(card, you, Zone::Stack, None, None);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    assert!(state.objects.len() == before + 2);
    for k in 1..=2 {
        let id = crate::ids::ObjectId(spell.0 + k);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you);
        assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Flying, &registry));
        assert!(state.effective_power(id, &registry) == Some(1) && state.effective_toughness(id, &registry) == Some(1));
        assert!(t.colors == [Color::White] && t.subtypes.len() == 1 && t.subtypes[0] == "Spirit");
    }
    std::mem::forget((registry, state));
}

/// Geistflame ("Geistflame deals 1 damage to any target.") resolving at a
/// creature, by the engine's own damage: one more damage marked on it than
/// before (CR 120.3e), and it is otherwise as it was; state-based actions
/// are not checked here.
fn geistflame_at_a_creature(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::geistflame::Geistflame));
    let target = any_creature(&mut state, blank);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + 1 && o.tapped == tapped && o.controller == controller);
    std::mem::forget((registry, state));
}

/// Geistflame resolving at a player, either one: that player has one life
/// fewer (CR 120.3a), and the other as many as before.
fn geistflame_at_a_player(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::geistflame::Geistflame));
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let at = u8::from(kani::any::<bool>());
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Player(PlayerId(at))], &registry);

    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] - i32::from(p == at));
    }
    std::mem::forget((registry, state));
}

macro_rules! engine_under_card {
    ($($name:ident: $proof:ident($open:expr);)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        fn $name() {
            $proof($open);
        }
    )*};
}

const EVERY: Open = Open { tapped: true, controller: true, damage: true, toughness: true, shield: true, indestructible: true };

engine_under_card! {
    doom_blade_destroys_a_creature_with_every_scalar_open: doom_blade(EVERY);
    giant_growth_gives_its_target_three_and_three: giant_growth(());
    think_twice_draws_from_your_empty_library: think_twice((0, false));
    think_twice_draws_from_their_empty_library: think_twice((0, true));
    think_twice_draws_your_only_card: think_twice((1, false));
    think_twice_draws_their_only_card: think_twice((1, true));
    think_twice_draws_the_top_of_your_two: think_twice((2, false));
    think_twice_draws_the_top_of_their_two: think_twice((2, true));
    midnight_haunting_makes_two_spirits: midnight_haunting(());
    geistflame_deals_one_damage_to_a_creature: geistflame_at_a_creature(());
    geistflame_deals_one_damage_to_a_player: geistflame_at_a_player(());
}

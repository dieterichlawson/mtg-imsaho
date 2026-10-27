//! Destroying and sacrificing permanents, which a card proof's
//! `record_destroy`, `record_destroy_by`, `record_destroy_no_regen`,
//! `record_destroy_all`, `record_destroy_all_by` and `record_sacrifice`
//! stand in for: `destruction::try_destroy` and its forms (CR 701.7) and
//! `destruction::sacrifice_by` (701.17a).
//!
//! The move to the graveyard is `move_object`'s, recorded here (its own
//! proof says what it does), and whether a permanent is indestructible is
//! any answer (`proofs::decide`), the same each time it is asked: which
//! permanents are is `has_keyword`'s own proof.

use crate::cards::CardRegistry;
use crate::destruction::{self, DestroyResult};
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::proofs::cards::{act, acted, blank_board, Act, Snapshot};
use crate::proofs::decide::{decided, Question};
use crate::state::GameState;
use crate::types::{Keyword, Zone};

/// Stands in for `GameState::has_keyword`: destroying asks only whether a
/// permanent is indestructible.
#[allow(dead_code)] // named only by `kani::stub`
fn is_indestructible(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    assert!(keyword == Keyword::Indestructible, "destroying asks only about indestructible");
    decided(Question::IsIndestructible, id)
}

/// An object a destruction is asked about, as it was before.
#[derive(Clone, Copy)]
struct Asked {
    id: ObjectId,
    on_battlefield: bool,
    controller: PlayerId,
    /// Its toughness, if it is a creature.
    toughness: Option<i32>,
    tapped: bool,
    shields: u32,
}

/// A permanent of the blank card's, of either player's and controlled by
/// either, a creature of any toughness or not, tapped or not, with damage
/// (deathtouch damage or not) and up to two regeneration shields; or a card
/// of it in its owner's graveyard.
fn a_permanent(state: &mut GameState, blank: CardId) -> Asked {
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let on_battlefield: bool = kani::any();
    let toughness = kani::any::<bool>().then(|| i32::from(kani::any::<i8>()));
    let zone = if on_battlefield { Zone::Battlefield } else { Zone::Graveyard };
    let id = state.create_object(blank, owner, zone, toughness, toughness);
    let obj = state.get_object_mut(id).expect("just created");
    obj.name = "Blank".into();
    if on_battlefield {
        obj.controller = if kani::any() { PlayerId(1 - owner.0) } else { owner };
        obj.tapped = kani::any();
        obj.damage_marked = u32::from(kani::any::<u8>());
        obj.dealt_deathtouch_damage = kani::any();
        obj.regeneration_shields = u32::from(kani::any::<u8>() % 3);
    }
    Asked {
        id,
        on_battlefield,
        controller: obj.controller,
        toughness,
        tapped: obj.tapped,
        shields: obj.regeneration_shields,
    }
}

/// What destroying it is by the rules: nothing, if it is not a permanent
/// (CR 701.7a); nothing, if it is indestructible (702.12b); if it has a
/// regeneration shield and may be regenerated, its regeneration instead
/// (701.15a); else it dies.
fn by_the_rules(a: &Asked, may_regenerate: bool) -> DestroyResult {
    if !a.on_battlefield {
        DestroyResult::NotAPermanent
    } else if decided(Question::IsIndestructible, a.id) {
        DestroyResult::Indestructible
    } else if may_regenerate && a.shields > 0 {
        DestroyResult::Regenerated
    } else {
        DestroyResult::Died
    }
}

/// Whether `e` is the death of `a`, as it was before it died (CR 608.2g).
fn is_death_of(e: Option<&GameEvent>, a: &Asked, blank: CardId) -> bool {
    matches!(e, Some(GameEvent::CreatureDied {
        object, name, card_id, controller, damaged_by, last_known_toughness, is_token, subtypes,
    }) if *object == a.id && name == "Blank" && *card_id == blank && *controller == a.controller
        && damaged_by.is_empty() && Some(*last_known_toughness) == a.toughness && !*is_token
        && subtypes.is_empty())
}

/// That each of `asked` came to its outcome, and nothing else happened. A
/// regenerated permanent is tapped (announced, if it was not already),
/// loses its damage and one shield (CR 701.15a); a permanent that died
/// was put into its owner's graveyard, and if it was a creature its death
/// was announced first, as it was (608.2g), and a creature died this turn
/// (the morbid flag). Regenerations come first, then deaths, each in the
/// order asked.
fn came_to(state: &GameState, before: &Snapshot, asked: &[Asked], outcomes: &[DestroyResult], blank: CardId) {
    let mut expected = before.without_events();
    for (a, r) in asked.iter().zip(outcomes) {
        if *r == DestroyResult::Regenerated {
            expected = expected.with_tapped(a.id, true).with_damage(a.id, 0);
            let obj = state.get_object(a.id).expect("still in the game");
            assert!(obj.regeneration_shields == a.shields - 1 && !obj.dealt_deathtouch_damage);
        }
    }
    assert!(Snapshot::of(state).without_events() == expected);

    let mut n = 0;
    for (a, r) in asked.iter().zip(outcomes) {
        if *r == DestroyResult::Regenerated && !a.tapped {
            assert!(matches!(state.events.get(n), Some(GameEvent::Tapped { object }) if *object == a.id));
            n += 1;
        }
    }
    let mut moves = 0;
    let mut died = false;
    for (a, r) in asked.iter().zip(outcomes) {
        if *r == DestroyResult::Died {
            if a.toughness.is_some() {
                assert!(is_death_of(state.events.get(n), a, blank));
                n += 1;
                died = true;
            }
            assert!(Some(&Act::Move(a.id, Zone::Graveyard)) == act(moves));
            moves += 1;
        }
    }
    assert!(state.events.len() == n && acted() == moves);
    assert!(state.creature_died_this_turn == died);
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
#[kani::stub(crate::state::GameState::has_keyword, is_indestructible)]
/// "Destroy target permanent" (CR 701.7a): a permanent that is not
/// indestructible (702.12b) and has no regeneration shield is put into
/// its owner's graveyard, and a creature's death is announced; one with a
/// shield regenerates instead (701.15a), unless it can't be regenerated;
/// an indestructible one, or a card no longer on the battlefield, is left
/// as it was. What comes back says which. Through `try_destroy`,
/// `try_destroy_by`, `try_destroy_no_regen` (which "can't be
/// regenerated") and, ignoring indestructible and shields alike,
/// `sacrifice_by` (701.17a), whose answer is whether it was sacrificed;
/// for a permanent as `a_permanent` makes it, or an object no longer in
/// the game. Nothing else changes.
fn destroying_a_permanent_destroys_regenerates_or_leaves_it() {
    let (registry, mut state, blank) = blank_board();
    let mut a = a_permanent(&mut state, blank);
    if kani::any() {
        // One that has left the game: a token that ceased to exist.
        a.id = ObjectId(a.id.0 + 1);
        a.on_battlefield = false;
    }
    let before = Snapshot::of(&state);

    let way: u8 = kani::any();
    let (result, expected) = match way {
        0 => (destruction::try_destroy(&mut state, a.id, &registry), by_the_rules(&a, true)),
        1 => (destruction::try_destroy_by(&mut state, a.id, "a source", &registry), by_the_rules(&a, true)),
        2 => (destruction::try_destroy_no_regen(&mut state, a.id, &registry), by_the_rules(&a, false)),
        _ => {
            let sacrificed = destruction::sacrifice_by(&mut state, a.id, "a reason", &registry);
            assert!(sacrificed == a.on_battlefield);
            let r = if a.on_battlefield { DestroyResult::Died } else { DestroyResult::NotAPermanent };
            (r, r)
        }
    };

    assert!(result == expected);
    came_to(&state, &before, &[a], &[result], blank);
    kani::cover!(result == DestroyResult::Regenerated && !a.tapped, "an untapped creature regenerates");
    kani::cover!(result == DestroyResult::Died && a.toughness.is_some(), "a creature dies");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
#[kani::stub(crate::state::GameState::has_keyword, is_indestructible)]
/// "Destroy all creatures" (CR 701.7a, 700.2c): each permanent comes to
/// the outcome destroying it alone would, decided before any of them has
/// moved, and the outcomes come back in the order asked. For two
/// permanents as `a_permanent` makes them, through `try_destroy_all` and
/// `try_destroy_all_by`. Nothing else changes.
fn destroying_permanents_together_destroys_each_as_alone() {
    let (registry, mut state, blank) = blank_board();
    let asked = [a_permanent(&mut state, blank), a_permanent(&mut state, blank)];
    let ids = [asked[0].id, asked[1].id];
    let before = Snapshot::of(&state);

    let outcomes = if kani::any() {
        destruction::try_destroy_all(&mut state, &ids, &registry)
    } else {
        destruction::try_destroy_all_by(&mut state, &ids, "a source", &registry)
    };

    let expected = [by_the_rules(&asked[0], true), by_the_rules(&asked[1], true)];
    assert!(outcomes.len() == 2);
    assert!(outcomes[0] == (ids[0], expected[0]) && outcomes[1] == (ids[1], expected[1]));
    came_to(&state, &before, &asked, &expected, blank);
    kani::cover!(expected == [DestroyResult::Regenerated, DestroyResult::Died], "one regenerates, one dies");
    std::mem::forget((registry, state, outcomes));
}

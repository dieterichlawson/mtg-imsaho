//! The spells that change a target until end of turn, tap or untap it,
//! put counters on it, or make a player lose life.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Giant Growth, {G} Instant: "Target creature gets +3/+3 until end of
//!   turn."
//! - Moment of Heroism, {1}{W} Instant: "Target creature gets +2/+2 and
//!   gains lifelink until end of turn."
//! - Ranger's Guile, {G} Instant: "Target creature you control gets +1/+1
//!   and gains hexproof until end of turn."
//! - Spidery Grasp, {2}{G} Instant: "Untap target creature. It gets +2/+4
//!   and gains reach until end of turn."
//! - Feeling of Dread, {1}{W} Instant: "Tap up to two target creatures.
//!   Flashback {1}{U}"
//! - Nightbird's Clutches, {1}{R} Sorcery: "Up to two target creatures
//!   can't block this turn. Flashback {3}{R}"
//! - Travel Preparations, {1}{G} Sorcery: "Put a +1/+1 counter on each of
//!   up to two target creatures. Flashback {1}{W}"
//! - Bump in the Night, {B} Sorcery: "Target opponent loses 3 life.
//!   Flashback {5}{R}"
//!
//! Each proof is over the targets the spell could hold as it resolves: a
//! permanent or card of any characteristics, on the battlefield or gone to
//! a graveyard (a target that is no longer a permanent is one the effect
//! does nothing to, CR 608.2b, 400.7), or a target gone illegal; for the
//! "up to two" spells, any number of two such from none to both. What the
//! effects then do, until the cleanup step ends them (CR 514.2), is the
//! engine's, proved in `proofs::actions`.

use super::{acted, any_object, board, card_proof, each_is, picked, ANY_PERMANENT, Form, Snapshot};
use crate::actions::Target;
use crate::cards::{CardBehavior, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, TemporaryEffect as T};
use crate::types::Zone;

/// `card` resolving holding `targets` (picked from two objects of the
/// board, each a permanent or card of any characteristics), cast by player
/// 0: the game after, the objects, and the board before.
fn resolve_holding(card: Box<dyn CardBehavior>, targets: impl FnOnce([ObjectId; 2]) -> Vec<Target>) -> (crate::cards::CardRegistry, GameState, [ObjectId; 2], Vec<Target>, Snapshot) {
    let (registry, mut state, blank, card) = board(card);
    let objects = [any_object(&mut state, blank, &ANY_PERMANENT, Form::Card), any_object(&mut state, blank, &ANY_PERMANENT, Form::Card)];
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let targets = targets(objects);
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, spell, &targets, &registry);
    (registry, state, objects, targets, before)
}

/// Whether `card` targets "up to two target creatures" (CR 115.1, 601.2c),
/// read as a pattern (`proofs::cards::prints_effects` says why).
fn up_to_two_creatures(card: &dyn CardBehavior) -> bool {
    let r = std::mem::ManuallyDrop::new(card.target_requirement());
    matches!(&*r, TargetRequirement::UpToTargets(2, each) if matches!(**each, TargetRequirement::Creature))
}

/// Up to two: none, either, or both, in either order.
fn up_to_two(objects: [ObjectId; 2]) -> Vec<Target> {
    up_to_two_of(kani::any::<u8>() % 5, objects)
}

/// [`up_to_two`]'s choice `which`: none, the first, the second, both, or
/// both the other way round.
fn up_to_two_of(which: u8, objects: [ObjectId; 2]) -> Vec<Target> {
    match which {
        0 => vec![],
        1 => vec![Target::Object(objects[0])],
        2 => vec![Target::Object(objects[1])],
        3 => vec![Target::Object(objects[0]), Target::Object(objects[1])],
        _ => vec![Target::Object(objects[1]), Target::Object(objects[0])],
    }
}

/// The permanents among `targets` on the battlefield, in order.
fn on_battlefield(state: &GameState, targets: &[Target]) -> Vec<ObjectId> {
    picked(targets, |t| match t {
        Target::Object(id) if state.get_object(*id).is_some_and(|o| o.zone == Zone::Battlefield) => Some(*id),
        _ => None,
    })
}

card_proof! {
    /// Nightbird's Clutches: "up to two target creatures", each still a
    /// permanent unable to block this turn, and nothing else.
    fn nightbirds_clutches_stops_up_to_two_blocking() {
        assert!(up_to_two_creatures(&crate::cards::isd::nightbirds_clutches::NightbirdsClutches));
        let (registry, state, _, targets, before) = resolve_holding(Box::new(crate::cards::isd::nightbirds_clutches::NightbirdsClutches), up_to_two);
        let added = &state.until_end_of_turn[before.until_end_of_turn()..];
        let ids = on_battlefield(&state, &targets);
        assert!(each_is(added, ids.len(), |i| T::CantBlock { target: ids[i] }) && acted() == 0);
        assert!(Snapshot::of(&state).without_until_end_of_turn() == before.without_until_end_of_turn());
        std::mem::forget((registry, state));
    }
}


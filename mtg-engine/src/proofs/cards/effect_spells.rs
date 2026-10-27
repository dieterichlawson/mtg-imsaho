//! The spells that change a target until end of turn, tap or untap it,
//! put counters on it, or make a player lose life.
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

use super::{acted, any_object, asked_for, board, card_proof, each_is, Act, Form, Snapshot, ANY_PERMANENT};
use crate::actions::Target;
use crate::cards::{CardBehavior, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, TemporaryEffect as T};
use crate::types::{CounterType, Keyword, Zone};

/// `card` resolving holding `targets` (picked from two objects of the
/// board, each a permanent or card of any characteristics), cast by player
/// 0: the game after, the objects, and the board before.
fn resolve_holding(card: Box<dyn CardBehavior>, targets: fn([ObjectId; 2]) -> Vec<Target>) -> (crate::cards::CardRegistry, GameState, [ObjectId; 2], Vec<Target>, Snapshot) {
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

/// The first object, or a target gone illegal.
fn one(objects: [ObjectId; 2]) -> Vec<Target> {
    vec![if kani::any() { Target::Object(objects[0]) } else { Target::Illegal }]
}

/// Up to two: none, either, or both, in either order.
fn up_to_two(objects: [ObjectId; 2]) -> Vec<Target> {
    match kani::any::<u8>() % 5 {
        0 => vec![],
        1 => vec![Target::Object(objects[0])],
        2 => vec![Target::Object(objects[1])],
        3 => vec![Target::Object(objects[0]), Target::Object(objects[1])],
        _ => vec![Target::Object(objects[1]), Target::Object(objects[0])],
    }
}

/// The permanents among `targets` on the battlefield, in order.
fn on_battlefield(state: &GameState, targets: &[Target]) -> Vec<ObjectId> {
    targets.iter()
        .filter_map(|t| match t { Target::Object(id) => Some(*id), _ => None })
        .filter(|id| state.get_object(*id).is_some_and(|o| o.zone == Zone::Battlefield))
        .collect()
}

/// `card`, holding one target, adds exactly `effects(target)` until end of
/// turn when that target is a permanent, and nothing otherwise; asks for
/// `acts(target)` then, and for nothing otherwise; and changes nothing
/// else.
fn lasts_the_turn<const N: usize>(card: Box<dyn CardBehavior>, effects: fn(ObjectId) -> [T; N], acts: fn(ObjectId) -> Vec<Act>) {
    let (registry, state, _, targets, before) = resolve_holding(card, one);
    let added = &state.until_end_of_turn[before.until_end_of_turn()..];
    match on_battlefield(&state, &targets).as_slice() {
        [id] => {
            let expected = std::mem::ManuallyDrop::new(effects(*id));
            assert!(added == *expected);
            assert!(asked_for(&acts(*id)));
            assert!(Snapshot::of(&state).without_until_end_of_turn() == before.without_until_end_of_turn());
        }
        _ => {
            assert!(added.is_empty() && acted() == 0);
            assert!(Snapshot::of(&state) == before);
        }
    }
    std::mem::forget((registry, state));
}

card_proof! {
    /// Giant Growth: "target creature" (CR 115.1), +3/+3 until end of turn.
    fn giant_growth_gives_plus_three_plus_three() {
        assert!(crate::cards::giant_growth::GiantGrowth.target_requirement() == TargetRequirement::Creature);
        lasts_the_turn(Box::new(crate::cards::giant_growth::GiantGrowth),
            |id| [T::ModifyPT { target: id, power_mod: 3, toughness_mod: 3 }], |_| vec![]);
    }
}

card_proof! {
    /// Moment of Heroism: "target creature", +2/+2 and lifelink until end
    /// of turn.
    fn moment_of_heroism_gives_plus_two_plus_two_and_lifelink() {
        assert!(crate::cards::isd::moment_of_heroism::MomentOfHeroism.target_requirement() == TargetRequirement::Creature);
        lasts_the_turn(Box::new(crate::cards::isd::moment_of_heroism::MomentOfHeroism),
            |id| [T::ModifyPT { target: id, power_mod: 2, toughness_mod: 2 }, T::GrantKeyword { target: id, keyword: Keyword::Lifelink }],
            |_| vec![]);
    }
}

card_proof! {
    /// Ranger's Guile: "target creature you control", +1/+1 and hexproof
    /// until end of turn.
    fn rangers_guile_gives_plus_one_plus_one_and_hexproof() {
        assert!(crate::cards::isd::rangers_guile::RangersGuile.target_requirement()
            == TargetRequirement::CreatureWithFilter(TargetFilter::YouControl));
        lasts_the_turn(Box::new(crate::cards::isd::rangers_guile::RangersGuile),
            |id| [T::ModifyPT { target: id, power_mod: 1, toughness_mod: 1 }, T::GrantKeyword { target: id, keyword: Keyword::Hexproof }],
            |_| vec![]);
    }
}

card_proof! {
    /// Spidery Grasp: "target creature", untapped, then +2/+4 and reach
    /// until end of turn.
    fn spidery_grasp_untaps_and_gives_plus_two_plus_four_and_reach() {
        assert!(crate::cards::isd::spidery_grasp::SpideryGrasp.target_requirement() == TargetRequirement::Creature);
        lasts_the_turn(Box::new(crate::cards::isd::spidery_grasp::SpideryGrasp),
            |id| [T::ModifyPT { target: id, power_mod: 2, toughness_mod: 4 }, T::GrantKeyword { target: id, keyword: Keyword::Reach }],
            |id| vec![Act::Untap(id)]);
    }
}

card_proof! {
    /// Feeling of Dread: "up to two target creatures" (CR 115.1, 601.2c),
    /// each still a permanent tapped (701.21a), in order, and nothing else.
    fn feeling_of_dread_taps_up_to_two() {
        assert!(up_to_two_creatures(&crate::cards::isd::feeling_of_dread::FeelingOfDread));
        let (registry, state, _, targets, before) = resolve_holding(Box::new(crate::cards::isd::feeling_of_dread::FeelingOfDread), up_to_two);
        let tapped: Vec<Act> = on_battlefield(&state, &targets).into_iter().map(Act::Tap).collect();
        assert!(asked_for(&tapped) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
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

card_proof! {
    /// Travel Preparations: "up to two target creatures", a +1/+1 counter
    /// on each, in order, and nothing else. A target no longer a permanent
    /// takes none, which `add_counters` decides (CR 122.1, and its proof in
    /// `proofs::actions`), so the card asks for each.
    fn travel_preparations_counts_up_to_two() {
        assert!(up_to_two_creatures(&crate::cards::isd::travel_preparations::TravelPreparations));
        let (registry, state, _, targets, before) = resolve_holding(Box::new(crate::cards::isd::travel_preparations::TravelPreparations), up_to_two);
        let countered: Vec<Act> = targets.iter()
            .map(|t| match t { Target::Object(id) => Act::Counters(*id, CounterType::PlusOnePlusOne, 1), _ => unreachable!() })
            .collect();
        assert!(asked_for(&countered) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Bump in the Night: "target opponent" (CR 115.1, 102.2), who loses 3
    /// life (119.3); with its target gone illegal, nothing.
    fn bump_in_the_night_makes_its_target_lose_three() {
        assert!(crate::cards::isd::bump_in_the_night::BumpInTheNight.target_requirement() == TargetRequirement::OpponentOnly);
        let (registry, state, _, targets, before) = resolve_holding(Box::new(crate::cards::isd::bump_in_the_night::BumpInTheNight), |_| {
            vec![match kani::any::<u8>() % 3 {
                0 => Target::Player(PlayerId(0)),
                1 => Target::Player(PlayerId(1)),
                _ => Target::Illegal,
            }]
        });
        match targets[0] {
            Target::Player(p) => {
                assert!(asked_for(&[Act::Life(p, -3)]));
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

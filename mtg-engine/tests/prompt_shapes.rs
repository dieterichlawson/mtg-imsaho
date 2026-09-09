//! Every card is asked for its targets in a shape a person can answer.
//!
//! The pool's targeting vocabulary is small, but two of its words used to be
//! spelled as *one cast per way of filling them* — one `CastSpell` action per
//! subset, per pair, per mode-and-subset. That is a menu that grows as
//! `C(n, k)`, `|a| x |b|` or `n + C(n, 2)`: 78 rows for Ghoulcaller's Chant
//! over a twelve-Zombie graveyard, 256 for Into the Maw of Hell over eight
//! lands and eight creatures a side, and both of them saying the same dozen
//! objects over and over.
//!
//! They are questions now. This is the sweep that says a NEW card cannot
//! quietly go back — it reads every requirement in the pool, cards and
//! activated abilities alike, and fails on one whose shape nothing has
//! decided about.
//!
//! It is deliberately a sweep and not a list of card names. The doc comment
//! of `every_flashback_card_is_offered_from_the_graveyard` explains why:
//! naming the cards is how a claim about "all of them" comes to be false.

mod common;
use common::*;
use mtg_engine::cards::TargetRequirement as R;
use mtg_engine::types::*;

/// How many targets a requirement takes, when that is one fixed number.
/// `None` for a count that is itself a choice.
fn fixed_arity(req: &R) -> Option<usize> {
    match req {
        R::None => Some(0),
        R::UpToTargets(..) | R::ModalChoice(..) => None,
        R::TwoTargets(a, b) => Some(fixed_arity(a)? + fixed_arity(b)?),
        _ => Some(1),
    }
}

/// What the engine does with a requirement, and whether that is a decision
/// someone made.
enum Shape {
    /// One row per candidate. The honest cardinality of "target creature".
    OneRowPerCandidate,
    /// The cast asks: a set, or a slot at a time. No row per combination.
    AskedByTheCast,
    /// Memory's Journey: the second slot's candidates depend on the first, so
    /// the first is still enumerated — over the two players, which is two
    /// rows. Any other card of this shape needs a look.
    EnumeratedFirstSlot,
    /// Nothing has decided about this one.
    Unhandled(String),
}

fn shape_of(req: &R) -> Shape {
    match req {
        R::None => Shape::OneRowPerCandidate,
        R::UpToTargets(..) => Shape::AskedByTheCast,
        R::TwoTargets(_, second) if matches!(**second, R::UpToTargets(..)) =>
            Shape::EnumeratedFirstSlot,
        R::TwoTargets(a, b) => match (fixed_arity(a), fixed_arity(b)) {
            (Some(1), Some(1)) => Shape::AskedByTheCast,
            _ => Shape::Unhandled(
                "a two-slot spell whose slots are not one target each".into()),
        },
        R::ModalChoice(modes) => {
            let arities: Option<Vec<usize>> = modes.iter().map(fixed_arity).collect();
            let Some(arities) = arities else {
                return Shape::Unhandled(
                    "a modal mode whose own size is a choice: the count can no \
                     longer say which mode was meant".into());
            };
            let mut distinct = arities.clone();
            distinct.sort_unstable();
            distinct.dedup();
            if distinct.len() != arities.len() {
                return Shape::Unhandled(format!(
                    "two modes taking the same number of targets ({arities:?}): the \
                     count cannot say which was meant, so this would enumerate"));
            }
            if distinct.iter().all(|n| *n <= 1) {
                Shape::OneRowPerCandidate
            } else {
                Shape::AskedByTheCast
            }
        }
        _ => Shape::OneRowPerCandidate,
    }
}

/// No card or ability in the pool asks for its targets in a shape that
/// becomes a menu of combinations.
///
/// A failure here is not necessarily a bug in the new card — it is a shape
/// nobody has decided about yet, and the decision belongs in
/// `targeting::set_slot` (which asks) or here (which says why enumerating it
/// is fine).
#[test]
fn no_card_offers_a_menu_of_target_combinations() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mut unhandled: Vec<String> = Vec::new();
    let mut enumerated_first_slot: Vec<String> = Vec::new();
    let mut asked = 0;
    let mut checked = 0;

    let names: Vec<String> = reg.all_names().into_iter().map(String::from).collect();
    for name in &names {
        let Some(card_id) = reg.get_id_by_name(name) else { continue };
        let Some(behavior) = reg.get(card_id) else { continue };

        let mut look = |what: String, req: &R| {
            checked += 1;
            match shape_of(req) {
                Shape::OneRowPerCandidate => {}
                Shape::AskedByTheCast => asked += 1,
                Shape::EnumeratedFirstSlot => enumerated_first_slot.push(what),
                Shape::Unhandled(why) => unhandled.push(format!("{what}: {why}")),
            }
        };

        look(name.clone(), &behavior.target_requirement());

        let obj = state.create_object(card_id, P0, Zone::Battlefield, None, None);
        for ability in behavior.activated_abilities(&state, obj, &reg) {
            if let Some(req) = &ability.target_requirement {
                look(format!("{name} :: ability {}", ability.ability_index), req);
            }
        }
    }

    assert!(unhandled.is_empty(),
        "these ask for targets in a shape nothing has decided about, so they \
         are enumerated one row per combination:\n  {}", unhandled.join("\n  "));

    // The one shape whose first slot is still a menu. It is a menu over the
    // two players, so it is two rows — but a card that narrowed a second slot
    // by something wider than "which player" would be a product again.
    assert_eq!(enumerated_first_slot, vec!["Memory's Journey".to_string()],
        "a card whose second slot depends on its first still enumerates the \
         first. That is fine over the two players and not over a board; \
         decide about this one before adding it");

    // Guard against the sweep passing because it swept nothing.
    assert!(checked > 100, "only {checked} requirements read; the pool is bigger");
    assert!(asked >= 5, "only {asked} requirements are asked for by the cast; \
        the pool has at least Feeling of Dread, Nightbird's Clutches, Travel \
        Preparations, Prey Upon, Into the Maw of Hell, Lost in the Mist and \
        Ghoulcaller's Chant");
}

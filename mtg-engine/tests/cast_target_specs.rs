//! The collapsed casting view's description of what a spell wants targeted.
//!
//! `legal_actions` hands a player two things for every castable spell: the
//! flat list of `CastSpell` actions, one per legal target set, and one
//! `CastableSpell` carrying a `CastTargetSpec` — the same offer as a shape a
//! client can walk slot by slot. Both clients build their cast from the
//! second, so a spec that says the wrong shape is a spell the interactive and
//! LLM players cannot cast correctly even though the flat list is right.
//!
//! One test per shape, because each is a separate arm and the fall-through is
//! silent: every arm that goes missing collapses to `SingleTarget`, which for
//! most of these means an empty list of options.

mod common;
use common::*;
use mtg_engine::actions::{CastTargetSpec, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn base() -> (GameState, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    state.priority_player = Some(P0);
    (state, reg)
}

/// The spec the engine offers for casting `spell`.
#[track_caller]
fn spec_for(state: &GameState, reg: &CardRegistry, spell: ObjectId) -> CastTargetSpec {
    mtg_engine::engine::legal_actions(state, reg).castable_spells.iter()
        .find(|c| c.object_id == spell && c.alternative_cost.is_none())
        .unwrap_or_else(|| panic!("no castable-spell entry for {spell:?}"))
        .target_spec.clone()
}

/// A spell with nothing to target says so, rather than offering an empty
/// list of things to choose from — the two read the same to a client that
/// only counts options, and only one of them can be cast.
#[test]
fn a_spell_that_targets_nothing_asks_for_no_target() {
    let (mut state, reg) = base();
    named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    assert!(matches!(spec_for(&state, &reg, bears), CastTargetSpec::NoTargets),
        "expected NoTargets, got {:?}", spec_for(&state, &reg, bears));
    assert_eq!(offered_target_sets(&state, &reg, bears), vec![Vec::<Target>::new()],
        "and exactly one way to cast it, naming nothing");
}

/// "Up to two target creatures" carries both halves: the ceiling and the
/// candidates. Dropping either turns a two-creature Feeling of Dread into a
/// one-creature one, or into no creature at all.
#[test]
fn an_up_to_two_spell_carries_its_ceiling_and_its_candidates() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    let dread = castable_spell(&mut state, &reg, "Feeling of Dread", P0);

    let CastTargetSpec::UpToTargets { max, options } = spec_for(&state, &reg, dread) else {
        panic!("expected an UpToTargets spec, got {:?}", spec_for(&state, &reg, dread));
    };
    assert_eq!(max, 2);
    assert_eq!(options.len(), 2, "both creatures are candidates: {options:?}");
    assert!(options.contains(&Target::Object(mine)) && options.contains(&Target::Object(theirs)),
        "either creature can be tapped: {options:?}");

    // The flat list agrees: none, either one, or both.
    let mut sizes: Vec<usize> = offered_target_sets(&state, &reg, dread)
        .iter().map(Vec::len).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, vec![0, 1, 1, 2]);
}

/// Two separate instances of the word "target" are two slots, each with its
/// own candidates — and the second slot's list is the one that goes with the
/// chosen first target, not a flat list of everything.
#[test]
fn a_two_target_spell_pairs_each_first_choice_with_its_own_seconds() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    let prey = castable_spell(&mut state, &reg, "Prey Upon", P0);

    let CastTargetSpec::TwoTargets { first, second, second_min, second_max } =
        spec_for(&state, &reg, prey)
    else {
        panic!("expected a TwoTargets spec, got {:?}", spec_for(&state, &reg, prey));
    };
    // "Target creature you control fights target creature you don't control."
    assert_eq!(first, vec![Target::Object(mine)]);
    assert_eq!(second, vec![vec![Target::Object(theirs)]]);
    assert_eq!(second.len(), first.len(), "the second lists are parallel to the first");
    assert_eq!((second_min, second_max), (1, 1), "both slots are mandatory and singular");
}

/// A "up to N" second slot is the one place a `TwoTargets` spell can be cast
/// naming fewer than two things.
#[test]
fn an_up_to_n_second_slot_may_be_left_empty() {
    let (mut state, reg) = base();
    let card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);
    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    let CastTargetSpec::TwoTargets { first, second, second_min, second_max } =
        spec_for(&state, &reg, journey)
    else {
        panic!("expected a TwoTargets spec, got {:?}", spec_for(&state, &reg, journey));
    };
    assert_eq!((second_min, second_max), (0, 3), "up to three, and none is a choice");
    // "from THEIR graveyard": the cards offered depend on the player chosen.
    let p1_slot = first.iter().position(|t| *t == Target::Player(P1))
        .expect("the opponent is a legal first target");
    let p0_slot = first.iter().position(|t| *t == Target::Player(P0))
        .expect("so is the caster");
    assert_eq!(second[p1_slot], vec![Target::Object(card)]);
    assert!(second[p0_slot].is_empty(), "p0's graveyard is empty: {:?}", second[p0_slot]);
}

/// A modal spell's spec offers the candidates of every mode at once — the
/// mode itself is read back from which of them the player names
/// (`detect_modal_choice_mode`), so a spec that offered only one mode's
/// candidates would make the other mode unreachable.
#[test]
fn a_modal_spell_offers_what_each_of_its_modes_can_name() {
    let (mut state, reg) = base();
    let ghoul = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    let bears = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);

    // "Return target creature card from your graveyard to your hand, or return
    // two target Zombie creature cards from your graveyard to your hand."
    let CastTargetSpec::SingleTarget(options) = spec_for(&state, &reg, chant) else {
        panic!("expected a SingleTarget spec, got {:?}", spec_for(&state, &reg, chant));
    };
    assert!(options.contains(&Target::Object(ghoul)) && options.contains(&Target::Object(bears)),
        "both creature cards are namable under mode one: {options:?}");

    // And the flat list keeps the second mode, which the spec's flat union
    // cannot express: the Zombie appears alone and in a pair.
    let sets = offered_target_sets(&state, &reg, chant);
    assert!(sets.contains(&vec![Target::Object(ghoul)]), "mode one on the Zombie: {sets:?}");
    assert!(sets.contains(&vec![Target::Object(bears)]), "mode one on the Bears: {sets:?}");
}

/// CR 601.2c: one instance of the word "target" cannot name the same thing
/// twice. The offered actions respect it by construction; a list a client
/// assembles slot by slot does not, and the second slot of a `TwoTargets`
/// is checked as its own instance.
#[test]
fn a_duplicate_in_a_two_target_spells_second_slot_is_dropped() {
    let (mut state, reg) = base();
    let a = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);
    let b = named_card_in_graveyard(&mut state, &reg, "Ambush Viper", P1);
    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    let cast = cast_action(journey, vec![
        Target::Player(P1), Target::Object(a), Target::Object(a), Target::Object(b),
    ]);
    let after = mtg_engine::engine::submit_action(&state, &cast, &reg);

    assert!(after.stack.iter().any(|e| e.as_spell() == Some(journey)),
        "the cast happened — the duplicate is dropped, not refused");
    let on_stack = after.get_object(journey).expect("the spell is on the stack");
    assert_eq!(on_stack.targets,
        vec![Target::Player(P1), Target::Object(a), Target::Object(b)],
        "the repeat of {a:?} is gone and nothing else moved");
}

/// The same, one slot over: the first slot is its own instance too, and a
/// spell whose slots want different things keeps both orderings.
#[test]
fn each_slot_of_a_two_target_spell_is_its_own_instance() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    let prey = castable_spell(&mut state, &reg, "Prey Upon", P0);

    // The one legal pair is offered once, and naming it is a cast.
    assert_eq!(offered_target_sets(&state, &reg, prey),
        vec![vec![Target::Object(mine), Target::Object(theirs)]]);
    let after = mtg_engine::engine::submit_action(
        &state,
        &cast_action(prey, vec![Target::Object(mine), Target::Object(theirs)]),
        &reg);
    assert_eq!(after.get_object(prey).expect("on the stack").targets,
        vec![Target::Object(mine), Target::Object(theirs)],
        "neither slot deduped against the other — they are different creatures");
}

/// "Up to N target X" draws from the same pool as "target X" (CR 601.2c
/// chooses the number first and the targets out of one pool after), and one
/// place answers that: `valid_targets_for_req`. Every caller hands it the
/// requirement whole rather than peeling the wrapper off first, so a spell
/// whose slot is "up to N" and one whose slot is a plain "target" offer the
/// same candidates.
#[test]
fn up_to_n_offers_the_same_candidates_as_one() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    let dread = castable_spell(&mut state, &reg, "Feeling of Dread", P0);
    let bolt = castable_spell(&mut state, &reg, "Geistflame", P0);

    let CastTargetSpec::UpToTargets { options: up_to, .. } = spec_for(&state, &reg, dread) else {
        panic!("Feeling of Dread is an up-to-two spell");
    };
    let CastTargetSpec::SingleTarget(single) = spec_for(&state, &reg, bolt) else {
        panic!("Geistflame names one thing");
    };
    for creature in [mine, theirs] {
        assert!(up_to.contains(&Target::Object(creature)),
            "up-to-two offers {creature:?}: {up_to:?}");
        assert!(single.contains(&Target::Object(creature)),
            "the single-target spell offers it too: {single:?}");
    }
}

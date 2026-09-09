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

/// The target prompt the cast raises when submitted with nothing named.
///
/// A `ChosenAtCast` spec says the candidates are not in the spec; this is
/// where they are instead, and it is what a client actually sees.
#[track_caller]
fn asked_for(state: &GameState, reg: &CardRegistry, spell: ObjectId)
    -> (Vec<Target>, usize, usize, Vec<Target>)
{
    let asked = cast_onto_stack(state, reg, spell, vec![]);
    match &asked.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTargetSet {
                options, min, max, fixed, .. }, .. }) =>
            (options.clone(), *min, *max, fixed.clone()),
        other => panic!("expected a target prompt for {spell:?}, got {other:?}"),
    }
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

    assert!(matches!(spec_for(&state, &reg, dread), CastTargetSpec::ChosenAtCast),
        "the cast asks for the slot, so the spec says so and carries no options");
    let (options, min, max, _) = asked_for(&state, &reg, dread);
    assert_eq!((min, max), (0, 2), "up to two, and none is a choice");
    assert_eq!(options.len(), 2, "both creatures are candidates: {options:?}");
    assert!(options.contains(&Target::Object(mine)) && options.contains(&Target::Object(theirs)),
        "either creature can be tapped: {options:?}");

    // The flat list is ONE cast with the slot empty: which targets it gets
    // is asked afterwards, on the screen that asks for a set (issue #360).
    // It used to be one action per subset — `sum(C(n, k))`, which is 4 here
    // and about 1,150 for Memory's Journey over a fifteen-card graveyard.
    let sets = offered_target_sets(&state, &reg, dread);
    assert_eq!(sets, vec![Vec::<Target>::new()], "one cast, targets unchosen: {sets:?}");
}

/// Two separate instances of the word "target" are two slots, asked one at a
/// time, each with its own candidates.
///
/// Enumerating the pairs is `|a| x |b|` rows — 256 for Into the Maw of Hell
/// over eight lands and eight creatures a side — and the two questions are
/// `|a| + |b|`. It cannot be one marking screen, because which target went
/// in which slot is part of the answer.
#[test]
fn a_two_target_spell_asks_for_one_slot_at_a_time() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    let prey = castable_spell(&mut state, &reg, "Prey Upon", P0);

    assert!(matches!(spec_for(&state, &reg, prey), CastTargetSpec::ChosenAtCast));

    // "Target creature you control fights target creature you don't control."
    let (slot1, min1, max1, fixed1) = asked_for(&state, &reg, prey);
    assert_eq!((min1, max1, fixed1.len()), (1, 1, 0), "one target, nothing in front of it");
    assert_eq!(slot1, vec![Target::Object(mine)], "the first slot is yours");

    // Answering the first raises the second, which knows what came before.
    let after = mtg_engine::engine::submit_action(
        &cast_onto_stack(&state, &reg, prey, vec![]),
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTargetSet(
                vec![Target::Object(mine)]) },
        &reg);
    let Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        choice: mtg_engine::state::ResolutionChoiceKind::ChooseTargetSet {
            options, min, max, fixed, .. }, .. }) = &after.awaiting_action else {
        panic!("expected the second slot's prompt, got {:?}", after.awaiting_action);
    };
    assert_eq!((*min, *max), (1, 1), "the second slot is mandatory and singular");
    assert_eq!(*options, vec![Target::Object(theirs)], "and it is theirs");
    assert_eq!(*fixed, vec![Target::Object(mine)],
        "the prompt carries what is already named, or the list has no context");
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

/// CR 601.2c: an "up to N" slot is chosen through a prompt the cast raises,
/// and the cast resumes with what comes back. Nothing is paid and the card
/// does not move until it does, so backing out costs nothing.
#[test]
fn an_up_to_slot_is_chosen_through_a_prompt_the_cast_raises() {
    use mtg_engine::actions::{Action, ResolvedChoice};
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    let dread = castable_spell(&mut state, &reg, "Feeling of Dread", P0);
    state.priority_player = Some(P0);
    let hand_before = state.objects_in_zone(Zone::Hand, P0).len();
    let pool_before = state.get_player(P0).mana_pool.total();
    assert!(pool_before > 0, "test precondition: the spell is payable");

    let cast = Action::CastSpell {
        object_id: dread, targets: vec![], sacrifice: None, exile_count: None,
        exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
    };
    let asked = mtg_engine::engine::submit_action(&state, &cast, &reg);
    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseTargetSet { options, min, max, .. }, .. })
        = &asked.awaiting_action else {
        panic!("expected a target-set prompt, got {:?}", asked.awaiting_action);
    };
    assert_eq!((*min, *max), (0, 2), "up to two, and none is a choice");
    assert_eq!(options.len(), 2, "both creatures: {options:?}");
    // Nothing has happened yet: the card is still in hand and the mana is
    // still in the pool.
    assert_eq!(asked.objects_in_zone(Zone::Hand, P0).len(), hand_before);
    assert_eq!(asked.get_player(P0).mana_pool.total(), pool_before);

    // Answering with a set finishes the cast, with those targets.
    let answer = Action::ResolveChoice {
        choice: ResolvedChoice::ChosenTargetSet(vec![Target::Object(mine), Target::Object(theirs)]),
    };
    let cast_done = mtg_engine::engine::submit_action(&asked, &answer, &reg);
    let on_stack = cast_done.get_object(dread).expect("the spell exists");
    assert_eq!(on_stack.zone, Zone::Stack, "the cast finished");
    assert_eq!(on_stack.targets.len(), 2, "with both targets: {:?}", on_stack.targets);

    // And marking none is a real cast of an "up to" spell (CR 601.2c),
    // not a cancel — the silent no-op of issue #49.
    let none = Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(vec![]) };
    let cast_none = mtg_engine::engine::submit_action(&asked, &none, &reg);
    assert_eq!(cast_none.get_object(dread).unwrap().zone, Zone::Stack, "still a cast");
    assert!(cast_none.get_object(dread).unwrap().targets.is_empty());

    // Backing out leaves the game exactly where it was.
    let cancel = Action::ResolveChoice { choice: ResolvedChoice::CancelCast };
    let backed_out = mtg_engine::engine::submit_action(&asked, &cancel, &reg);
    assert_eq!(backed_out.get_object(dread).unwrap().zone, Zone::Hand);
    assert_eq!(backed_out.get_player(P0).mana_pool.total(), pool_before, "nothing was paid");
    assert!(backed_out.pending_spell_cast.is_none() && backed_out.awaiting_action.is_none());
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
    // One Zombie beside one non-Zombie: mode one can name either, mode two
    // cannot be filled, and the count says so.
    assert!(matches!(spec_for(&state, &reg, chant), CastTargetSpec::ChosenAtCast));
    let (options, min, max, _) = asked_for(&state, &reg, chant);
    assert!(options.contains(&Target::Object(ghoul)) && options.contains(&Target::Object(bears)),
        "both creature cards are namable under mode one: {options:?}");
    assert_eq!((min, max), (1, 1), "only one Zombie, so mode two is not on offer");

    // A second Zombie puts mode two back, and the ceiling is where it shows.
    let second = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let (options, min, max, _) = asked_for(&state, &reg, chant);
    assert_eq!((min, max), (1, 2), "one card or two: {options:?}");
    assert!(options.contains(&Target::Object(second)));

    // And one cast is offered, not one per mode per subset.
    assert_eq!(offered_target_sets(&state, &reg, chant), vec![Vec::<Target>::new()]);
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

    // One announcement, and the slots are asked one at a time: the first
    // offers only yours, the second only theirs.
    assert_eq!(offered_target_sets(&state, &reg, prey), vec![Vec::<Target>::new()]);
    let (slot1, min1, max1, fixed1) = asked_for(&state, &reg, prey);
    assert_eq!((min1, max1, fixed1.len()), (1, 1, 0), "one target, nothing in front");
    assert_eq!(slot1, vec![Target::Object(mine)], "the first slot is yours");
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

    let (up_to, ..) = asked_for(&state, &reg, dread);
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

/// CR 601.2b: the mode is chosen as the spell is cast, and the spell on the
/// stack records which one — the invariant checker asks it back, and a
/// resolution that depended on the mode would read it there.
///
/// The engine reads the mode off the targets rather than being told it,
/// because neither client sends one: it takes the first mode whose candidates
/// contain everything named. Ghoulcaller's Chant is "return target creature
/// card from your graveyard" or "return two target Zombie creature cards",
/// so one Zombie is mode one and two Zombies is mode two.
#[test]
fn a_modal_spells_chosen_mode_is_read_back_off_its_targets() {
    let (mut state, reg) = base();
    let a = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    let b = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);

    let mode_for = |targets: Vec<Target>| {
        let after = mtg_engine::engine::submit_action(
            &state, &cast_action(chant, targets), &reg);
        after.get_object(chant).expect("on the stack").chosen_mode
    };

    assert_eq!(mode_for(vec![Target::Object(a)]), Some(0),
        "one creature card is the first mode");
    assert_eq!(mode_for(vec![Target::Object(a), Target::Object(b)]), Some(1),
        "two Zombie cards is the second — and only the second can hold both");
}

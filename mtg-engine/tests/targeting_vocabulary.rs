//! Every word in the targeting vocabulary is answered.
//!
//! `TargetRequirement` is what a card says it wants targeted, and
//! `valid_targets_for_req` is the one place that turns each of them into
//! candidates. Its match ends in `_ => vec![]`, and an empty candidate list is
//! not an error anywhere downstream — it is how "this spell has no legal
//! target" is spelled, so the spell is quietly dropped from the offer
//! (CR 601.2c) and a trigger is quietly removed from the stack (CR 603.3c).
//! A requirement the match does not know is therefore a card that silently
//! cannot be cast.
//!
//! That is not hypothetical: a second copy of this match lived in
//! `generate_ability_targets` and knew nine of these seventeen words, and the
//! only reason it never fired was that no activated ability in the pool used
//! one of the eight it was missing.
//!
//! Several of the words are used by no card yet, so this is the only thing
//! that asks. Each case builds the one board where the requirement has a
//! candidate and reads the offer back through `legal_actions`, which is what a
//! client would see.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::{CardBehavior, CardData, CardRegistry, TargetFilter, TargetRequirement};
use mtg_engine::state::StackEntry;
use mtg_engine::types::*;

/// A spell that costs nothing and wants exactly what it is told to want.
struct Probe(TargetRequirement);

impl CardBehavior for Probe {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Probe".into(),
            cost: Some(ManaCost::free()),
            card_types: vec![CardType::Instant],
            ..Default::default()
        }
    }
    fn target_requirement(&self) -> TargetRequirement {
        self.0.clone()
    }
}

/// A board with the probe in p0's hand, ready to cast.
///
/// The registry is the real card pool plus this one card, so the rest of the
/// board can be built out of real cards.
fn probing(req: TargetRequirement) -> (GameState, CardRegistry, ObjectId) {
    let mut reg = registry();
    let card_id = reg.register(Box::new(Probe(req)));
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    state.priority_player = Some(P0);
    let probe = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(probe).expect("just created").name = "Probe".into();
    (state, reg, probe)
}

/// The targets the offer names for the probe, deduplicated and sorted so a
/// case can compare against a set without minding the enumeration order.
///
/// Deduplicated because a requirement that offers one target per action names
/// each candidate once, but a `ModalChoice` names one across several modes;
/// where the count is the point the case reads `offered_target_sets` instead.
fn offered(state: &GameState, reg: &CardRegistry, probe: ObjectId) -> Vec<Target> {
    let mut seen: Vec<Target> = Vec::new();
    for t in offered_targets(state, reg, probe) {
        if !seen.contains(&t) {
            seen.push(t);
        }
    }
    sorted(seen)
}

fn sorted(mut v: Vec<Target>) -> Vec<Target> {
    v.sort_by_key(|t| format!("{t:?}"));
    v
}

#[test]
fn none_asks_for_nothing() {
    let (state, reg, probe) = probing(TargetRequirement::None);
    assert_eq!(offered_target_sets(&state, &reg, probe), vec![Vec::<Target>::new()]);
}

#[test]
fn any_target_reaches_creatures_planeswalkers_and_players() {
    let (mut state, reg, probe) = probing(TargetRequirement::AnyTarget);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    named_permanent(&mut state, &reg, "Forest", P0);
    assert_eq!(offered(&state, &reg, probe), sorted(vec![
        Target::Object(bear), Target::Object(garruk),
        Target::Player(P0), Target::Player(P1),
    ]));
}

#[test]
fn creature_reaches_creatures_and_nothing_else() {
    let (mut state, reg, probe) = probing(TargetRequirement::Creature);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    named_permanent(&mut state, &reg, "Forest", P0);
    assert_eq!(offered(&state, &reg, probe), vec![Target::Object(bear)]);
}

#[test]
fn creature_with_filter_narrows_to_the_filter() {
    let (mut state, reg, probe) = probing(
        TargetRequirement::CreatureWithFilter(TargetFilter::YouControl));
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    named_permanent(&mut state, &reg, "Ambush Viper", P1);
    assert_eq!(offered(&state, &reg, probe), vec![Target::Object(mine)]);
}

#[test]
fn player_only_reaches_both_seats_the_caster_first() {
    let (state, reg, probe) = probing(TargetRequirement::PlayerOnly);
    // Order matters and is not the seat order: the chooser reads this list as
    // "You / Opponent" (issue #138).
    assert_eq!(offered_targets(&state, &reg, probe),
        vec![Target::Player(P0), Target::Player(P1)]);
}

/// CR 102.1: "target opponent" is every player but the controller — which is
/// what makes it a different word from "target player".
#[test]
fn opponent_only_leaves_the_caster_out() {
    let (state, reg, probe) = probing(TargetRequirement::OpponentOnly);
    assert_eq!(offered(&state, &reg, probe), vec![Target::Player(P1)]);
}

#[test]
fn player_or_planeswalker_reaches_both() {
    let (mut state, reg, probe) = probing(TargetRequirement::PlayerOrPlaneswalker);
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P1);
    named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    assert_eq!(offered(&state, &reg, probe), sorted(vec![
        Target::Object(garruk), Target::Player(P0), Target::Player(P1),
    ]));
}

/// CR 111.4: only a spell on the stack, and never this one — a spell cannot
/// counter itself.
#[test]
fn spell_reaches_the_stack_but_not_the_counterspell_itself() {
    let (mut state, reg, probe) = probing(TargetRequirement::Spell);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(other, Zone::Stack, &reg);
    state.stack.push(StackEntry::Spell(other));
    assert_eq!(offered(&state, &reg, probe), vec![Target::Object(other)]);
}

#[test]
fn permanent_with_filter_reaches_any_permanent_the_filter_admits() {
    let (mut state, reg, probe) = probing(
        TargetRequirement::PermanentWithFilter(TargetFilter::YouDontControl));
    let theirs = named_permanent(&mut state, &reg, "Forest", P1);
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    assert_eq!(offered(&state, &reg, probe), vec![Target::Object(theirs)]);
}

/// Two slots, asked one at a time: each candidate is named once, in the slot
/// it belongs to, rather than once per pair it could appear in.
#[test]
fn two_targets_asks_for_each_slot_in_turn() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let (mut state, reg, probe) = probing(TargetRequirement::TwoTargets(
        Box::new(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
        Box::new(TargetRequirement::CreatureWithFilter(TargetFilter::YouDontControl)),
    ));
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);

    assert_eq!(offered_target_sets(&state, &reg, probe), vec![Vec::<Target>::new()],
        "one announcement, both slots still to be asked for");

    let ask = |s: &GameState| match &s.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTargetSet { options, min, max, .. }, .. }) =>
            (options.clone(), *min, *max),
        other => panic!("expected a slot prompt, got {other:?}"),
    };

    let first = cast_onto_stack(&state, &reg, probe, vec![]);
    assert_eq!(ask(&first), (vec![Target::Object(mine)], 1, 1), "the slot you control");

    let second = mtg_engine::engine::submit_action(&first,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTargetSet(
                vec![Target::Object(mine)]) }, &reg);
    assert_eq!(ask(&second), (vec![Target::Object(theirs)], 1, 1), "then the one you don't");
}

/// "Up to N" is one offer and one question, not a subset per announcement:
/// the candidates a client sees are the prompt's options, and the ceiling is
/// its `max`.
#[test]
fn up_to_targets_offers_its_candidates_once_under_the_ceiling() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let (mut state, reg, probe) = probing(
        TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature)));
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);

    assert_eq!(offered_target_sets(&state, &reg, probe), vec![Vec::<Target>::new()],
        "one announcement, with the slot still to be filled");

    let asked = cast_onto_stack(&state, &reg, probe, vec![]);
    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseTargetSet { options, min, max, .. }, .. })
        = &asked.awaiting_action else {
        panic!("expected a target-set prompt, got {:?}", asked.awaiting_action);
    };
    assert_eq!((*min, *max), (0, 2), "up to two, and none is a choice (CR 601.2c)");
    assert_eq!(sorted(options.clone()),
        sorted(vec![Target::Object(mine), Target::Object(theirs)]),
        "every creature, each once");
}

/// The ceiling drops to what is actually there — "up to two" with one
/// creature on the board cannot ask for two.
#[test]
fn up_to_targets_asks_for_no_more_than_the_board_holds() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let (mut state, reg, probe) = probing(
        TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature)));
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);

    let asked = cast_onto_stack(&state, &reg, probe, vec![]);
    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseTargetSet { options, min, max, .. }, .. })
        = &asked.awaiting_action else {
        panic!("expected a target-set prompt, got {:?}", asked.awaiting_action);
    };
    assert_eq!((*min, *max, options.len()), (0, 1, 1));
}

#[test]
fn graveyard_card_reaches_every_graveyard_and_only_graveyards() {
    let (mut state, reg, probe) = probing(TargetRequirement::GraveyardCard);
    let mine = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_card_in_graveyard(&mut state, &reg, "Ambush Viper", P1);
    named_permanent(&mut state, &reg, "Forest", P0);
    spell_in_hand(&mut state, &reg, "Geistflame", P0);
    assert_eq!(offered(&state, &reg, probe),
        sorted(vec![Target::Object(mine), Target::Object(theirs)]));
}

#[test]
fn graveyard_creature_is_your_graveyard_and_creatures_only() {
    let (mut state, reg, probe) = probing(TargetRequirement::GraveyardCreature);
    let mine = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    named_card_in_graveyard(&mut state, &reg, "Ambush Viper", P1);
    named_card_in_graveyard(&mut state, &reg, "Geistflame", P0);
    assert_eq!(offered(&state, &reg, probe), vec![Target::Object(mine)]);
}

#[test]
fn graveyard_creature_of_subtype_narrows_to_the_subtype() {
    let (mut state, reg, probe) = probing(
        TargetRequirement::GraveyardCreatureOfSubtype("Zombie".into()));
    let zombie = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P1);
    assert_eq!(offered(&state, &reg, probe), vec![Target::Object(zombie)]);
}

#[test]
fn graveyard_card_owned_by_caster_is_your_graveyard_whatever_the_card() {
    let (mut state, reg, probe) = probing(TargetRequirement::GraveyardCardOwnedByCaster);
    let creature = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let instant = named_card_in_graveyard(&mut state, &reg, "Geistflame", P0);
    named_card_in_graveyard(&mut state, &reg, "Ambush Viper", P1);
    named_permanent(&mut state, &reg, "Forest", P0);
    assert_eq!(offered(&state, &reg, probe),
        sorted(vec![Target::Object(creature), Target::Object(instant)]));
}

#[test]
fn graveyard_card_owned_by_opponent_is_the_other_graveyard() {
    let (mut state, reg, probe) = probing(TargetRequirement::GraveyardCardOwnedByOpponent);
    named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_card_in_graveyard(&mut state, &reg, "Ambush Viper", P1);
    named_permanent(&mut state, &reg, "Forest", P1);
    assert_eq!(offered(&state, &reg, probe), vec![Target::Object(theirs)]);
}

/// Standing alone the requirement has no player to be measured against, so it
/// is every graveyard; the narrowing happens where the co-target is known.
/// Every graveyard is still only the graveyards — the board and the hands are
/// not among the candidates.
#[test]
fn graveyard_card_owned_by_target_player_is_unconstrained_on_its_own() {
    let (mut state, reg, probe) = probing(TargetRequirement::GraveyardCardOwnedByTargetPlayer);
    let mine = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_card_in_graveyard(&mut state, &reg, "Ambush Viper", P1);
    named_permanent(&mut state, &reg, "Forest", P0);
    spell_in_hand(&mut state, &reg, "Geistflame", P1);
    assert_eq!(offered(&state, &reg, probe),
        sorted(vec![Target::Object(mine), Target::Object(theirs)]));
}

#[test]
fn exile_card_is_your_own_exile() {
    let (mut state, reg, probe) = probing(TargetRequirement::ExileCard);
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(mine, Zone::Exile, &reg);
    let theirs = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    state.move_object(theirs, Zone::Exile, &reg);
    named_card_in_graveyard(&mut state, &reg, "Geistflame", P0);
    assert_eq!(offered(&state, &reg, probe), vec![Target::Object(mine)]);
}

#[test]
fn modal_choice_offers_each_mode() {
    let (mut state, reg, probe) = probing(TargetRequirement::ModalChoice(vec![
        TargetRequirement::Creature,
        TargetRequirement::PlayerOnly,
    ]));
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let sets = offered_target_sets(&state, &reg, probe);
    assert!(sets.contains(&vec![Target::Object(bear)]), "mode one: {sets:?}");
    assert!(sets.contains(&vec![Target::Player(P0)]), "mode two: {sets:?}");
    assert!(sets.contains(&vec![Target::Player(P1)]), "mode two, other seat: {sets:?}");
}

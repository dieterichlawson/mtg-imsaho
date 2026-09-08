//! CR 616.1: when two or more replacement and/or prevention effects apply to
//! one damage event, the affected player — the damaged player, or the
//! damaged permanent's controller — chooses which applies first, and the
//! rest are considered again on the event as modified.
//!
//! Issue #323: the pipeline ran a fixed order (prevention, then Inquisitor's
//! Flail, then any card's replacement) and asked nobody. With a Flail on a
//! Zombie and an Undead Alchemist watching, it doubled first and milled
//! four; the defending player, who owns the choice, would mill two.

mod common;
use common::*;
use mtg_engine::actions::{Action, ResolvedChoice};
use mtg_engine::events::DamageTarget;
use mtg_engine::ids::{CardId, ObjectId, PlayerId};
use mtg_engine::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use mtg_engine::types::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// `count` nondescript cards on top of `player`'s library, to be milled.
fn stock_library(state: &mut GameState, player: PlayerId, count: usize) {
    for _ in 0..count {
        let id = state.create_object(CardId(9999), player, Zone::Library, None, None);
        state.get_player_mut(player).library_order.push(id);
    }
}

fn library_size(state: &GameState, player: PlayerId) -> usize {
    state.get_player(player).library_order.len()
}

/// The open CR 616.1 prompt: who is asked and what they are offered.
fn effect_prompt(state: &GameState) -> Option<(PlayerId, Vec<String>, String)> {
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            player, choice: ResolutionChoiceKind::ChooseDamageEffect { options, description, .. }, ..
        }) => Some((*player, options.clone(), description.clone())),
        _ => None,
    }
}

/// Answer the open prompt with the option whose label contains `word`.
fn choose(state: &GameState, reg: &mtg_engine::cards::CardRegistry, word: &str) -> GameState {
    let (_, options, _) = effect_prompt(state).expect("a damage-effect prompt is open");
    let index = options.iter().position(|o| o.contains(word))
        .unwrap_or_else(|| panic!("no option mentions {word:?}: {options:?}"));
    mtg_engine::engine::submit_action(state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(index, options[index].clone()),
    }, reg)
}

fn log_has(state: &GameState, needle: &str) -> bool {
    state.game_log.iter().any(|e| e.message.contains(needle))
}

/// A Zombie wearing a Flail attacks a player whose opponent controls an
/// Undead Alchemist: the two effects on that damage are the defender's to
/// order.
fn flail_and_alchemist() -> (GameState, mtg_engine::cards::CardRegistry, ObjectId) {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
    state.get_object_mut(flail).unwrap().attached_to = Some(corpse);
    stock_library(&mut state, P1, 10);
    attacks_unblocked(&mut state, corpse, P1);
    (state, reg, corpse)
}

// ---------------------------------------------------------------------------
// Defect 1 of issue #323: Inquisitor's Flail and Undead Alchemist.
// ---------------------------------------------------------------------------

/// The defending player is asked, and is told what each effect would do.
#[test]
fn the_defender_is_asked_which_effect_applies_first() {
    let (mut state, reg, corpse) = flail_and_alchemist();
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let (player, options, description) = effect_prompt(&state)
        .expect("two effects apply to the Corpse's damage, so the defender is asked");
    assert_eq!(player, P1, "the affected player is the one being damaged (CR 616.1)");
    assert_eq!(options.len(), 2, "{options:?}");
    assert!(options.iter().any(|o| o.contains("Inquisitor's Flail") && o.contains("double it to 4")),
        "the Flail's option says what doubling comes to: {options:?}");
    assert!(options.iter().any(|o| o.contains("Undead Alchemist") && o.contains("mills 2 cards")),
        "the Alchemist's option says how many cards: {options:?}");
    assert!(description.contains(&format!("Walking Corpse (#{})", corpse.0))
        && description.contains("2 combat damage to you"),
        "the description names the event, addressed to the chooser: {description}");
    assert_eq!(state.get_player(P1).life, 20, "nothing is dealt while the choice is open");
    assert_eq!(library_size(&state, P1), 10, "and nothing is milled");

    // The flat answers offered are one per effect, in order.
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let indices: Vec<usize> = legal.actions.iter().map(|a| match a {
        Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(i, _) } => *i,
        other => panic!("a damage-effect prompt offers index answers, not {other:?}"),
    }).collect();
    assert_eq!(indices, vec![0, 1]);
    assert_eq!(legal.context.as_deref(), Some(description.as_str()));
}

/// Undead Alchemist first: the damage event is replaced outright, so there
/// is nothing left for the Flail to double. The defender mills two.
#[test]
fn the_alchemist_first_mills_two() {
    let (mut state, reg, corpse) = flail_and_alchemist();
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let state = choose(&state, &reg, "Undead Alchemist");

    assert!(state.awaiting_action.is_none(), "one choice settles the event");
    assert_eq!(library_size(&state, P1), 8, "milled two, not four");
    assert_eq!(state.get_player(P1).life, 20, "the damage never happened");
    assert!(state.pending_damage.is_empty(), "nothing is left waiting");
    assert!(log_has(&state, &format!(
        "p1: Undead Alchemist (#{}) applies first to Walking Corpse (#{})'s 2 combat damage to p1 (CR 616.1)",
        state.objects.values().find(|o| o.name == "Undead Alchemist").unwrap().id.0, corpse.0)),
        "the choice is on the log:\n{}", state.game_log.iter().map(|e| e.message.as_str()).collect::<Vec<_>>().join("\n"));
}

/// The Flail first: the damage becomes 4, and the Alchemist — now the only
/// effect left that applies — turns that into a mill of four without a
/// second question.
#[test]
fn the_flail_first_mills_four_with_no_second_prompt() {
    let (mut state, reg, _) = flail_and_alchemist();
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let state = choose(&state, &reg, "Inquisitor's Flail");

    assert!(state.awaiting_action.is_none(),
        "with one effect left there is no choice, so nothing more is asked");
    assert_eq!(library_size(&state, P1), 6, "doubled to 4, then milled 4");
    assert_eq!(state.get_player(P1).life, 20);
}

/// An answer that names no offered effect is refused and the question stands.
#[test]
fn an_answer_off_the_list_is_refused() {
    let (mut state, reg, _) = flail_and_alchemist();
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let refused = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(7, "nothing".into()),
    }, &reg);

    assert!(effect_prompt(&refused).is_some(), "the prompt is still up");
    assert_eq!(refused.pending_damage, state.pending_damage, "and the damage still waits");
    assert_eq!(library_size(&refused, P1), 10);
}

// ---------------------------------------------------------------------------
// Defect 2 of issue #323: Ghostly Possession and Undead Alchemist.
// ---------------------------------------------------------------------------

fn possession_and_alchemist() -> (GameState, mtg_engine::cards::CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let possession = named_permanent(&mut state, &reg, "Ghostly Possession", P0);
    state.get_object_mut(possession).unwrap().attached_to = Some(corpse);
    stock_library(&mut state, P1, 10);
    attacks_unblocked(&mut state, corpse, P1);
    (state, reg)
}

/// A prevention effect is one of the effects CR 616.1 covers, and a defender
/// who wants the mill — Zombie fuel across the table — can have it.
#[test]
fn a_defender_may_take_the_mill_over_the_prevention() {
    let (mut state, reg) = possession_and_alchemist();
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let (player, options, _) = effect_prompt(&state).expect("prevention or replacement: the defender's call");
    assert_eq!(player, P1);
    assert!(options.iter().any(|o| o.contains("Ghostly Possession") && o.contains("on Walking Corpse")
        && o.contains("prevent all of it")), "{options:?}");

    let milled = choose(&state, &reg, "Undead Alchemist");
    assert_eq!(library_size(&milled, P1), 8, "the defender chose the mill");
    assert_eq!(milled.get_player(P1).life, 20);
    assert!(!log_has(&milled, "prevented"), "nothing was prevented; the event was replaced");

    let prevented = choose(&state, &reg, "Ghostly Possession");
    assert_eq!(library_size(&prevented, P1), 10, "the defender chose the prevention");
    assert_eq!(prevented.get_player(P1).life, 20);
    assert!(log_has(&prevented, "2 combat damage from Walking Corpse"), "the prevention is on the log");
    assert!(log_has(&prevented, "prevented by Ghostly Possession"));
}

/// Moonmist's turn-wide prevention is an effect with no permanent behind
/// it; it is offered by the card's name all the same.
#[test]
fn a_turn_wide_prevention_is_offered_by_name() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P1);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    stock_library(&mut state, P1, 10);
    stock_library(&mut state, P0, 3);
    let moonmist = castable_spell(&mut state, &reg, "Moonmist", P1);
    let mut state = cast_and_resolve(&state, &reg, moonmist, vec![]);
    state.active_player = P0;
    state.priority_player = Some(P0);
    attacks_unblocked(&mut state, corpse, P1);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let (_, options, _) = effect_prompt(&state).expect("Moonmist or the Alchemist");
    assert!(options.iter().any(|o| o == "Moonmist: prevent all of it"), "{options:?}");
    let prevented = choose(&state, &reg, "Moonmist");
    assert_eq!(library_size(&prevented, P1), 10);
    assert!(log_has(&prevented, "prevented by Moonmist"));
}

// ---------------------------------------------------------------------------
// When the order cannot matter, nobody is asked.
// ---------------------------------------------------------------------------

/// The Flail and Ghostly Possession on one attacker: doubled or not, all of
/// it is prevented. No prompt, and the prevention says so.
#[test]
fn a_flail_under_a_shield_is_no_question() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
    state.get_object_mut(flail).unwrap().attached_to = Some(corpse);
    let possession = named_permanent(&mut state, &reg, "Ghostly Possession", P0);
    state.get_object_mut(possession).unwrap().attached_to = Some(corpse);
    attacks_unblocked(&mut state, corpse, P1);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert!(state.awaiting_action.is_none(), "every order ends the same way");
    assert_eq!(state.get_player(P1).life, 20);
    assert!(log_has(&state, "prevented by Ghostly Possession"));
    assert!(state.pending_damage.is_empty());
}

/// Two Undead Alchemists are one effect twice over: whichever is first, the
/// event is replaced once (CR 614.5). No prompt; mill two.
#[test]
fn two_alchemists_are_no_question() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    stock_library(&mut state, P1, 10);
    attacks_unblocked(&mut state, corpse, P1);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert!(state.awaiting_action.is_none());
    assert_eq!(library_size(&state, P1), 8, "one replacement, two cards");
}

/// Unbreathing Horde blocking a Flailed attacker: doubled or not, the damage
/// is prevented and a counter comes off. No prompt.
#[test]
fn a_horde_blocking_a_flail_is_no_question() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
    state.get_object_mut(flail).unwrap().attached_to = Some(attacker);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P1);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);
    attacks_blocked_by(&mut state, attacker, P1, &[horde]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert!(state.awaiting_action.is_none());
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0, "prevented");
    assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 2, "one counter removed");
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 6,
        "the Horde's 3 damage to the Flailed attacker is doubled");
}

// ---------------------------------------------------------------------------
// Noncombat damage: protection against Unbreathing Horde's own effect.
// ---------------------------------------------------------------------------

/// Spare from Evil gives the Horde protection from non-Human creatures; a
/// fight with one then has two effects on the Horde's damage — protection,
/// which prevents it and nothing else, and the Horde's own, which also
/// removes a counter. The Horde's controller chooses (CR 616.1), and the
/// counter stays or goes with the choice.
#[test]
fn the_hordes_controller_chooses_between_protection_and_the_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);
    let brute = ready_creature(&mut state, P1, 4, 4);
    let spare = castable_spell(&mut state, &reg, "Spare from Evil", P0);
    let state = cast_and_resolve(&state, &reg, spare, vec![]);
    assert!(state.has_protection_from(horde, brute, &reg), "test setup: the Horde is protected");

    let mut fought = state.clone();
    mtg_engine::combat::fight(&mut fought, horde, brute, &reg);

    let (player, options, _) = effect_prompt(&fought).expect("protection or the Horde's own effect");
    assert_eq!(player, P0, "the affected player is the damaged permanent's controller");
    assert!(options.iter().any(|o| o.contains("protection from") && o.contains("prevent all of it")), "{options:?}");
    assert!(options.iter().any(|o| o.contains("Unbreathing Horde") && o.contains("remove a +1/+1 counter")
        && o.contains("(3 on it)")), "{options:?}");
    assert_eq!(fought.get_object(brute).unwrap().damage_marked, 0,
        "the Horde's half of the fight waits with the choice too (CR 701.12a)");

    let kept = choose(&fought, &reg, "protection");
    assert_eq!(counters_of(&kept, horde, CounterType::PlusOnePlusOne), 3, "protection first: the counter stays");
    assert_eq!(kept.get_object(horde).unwrap().damage_marked, 0);
    assert_eq!(kept.get_object(brute).unwrap().damage_marked, 3, "and the Horde's own damage lands");

    // "Unbreathing Horde" is in the protection option too (it is the Horde's
    // protection), so the Horde's own effect is picked by what it does.
    let spent = choose(&fought, &reg, "remove a +1/+1 counter");
    assert_eq!(counters_of(&spent, horde, CounterType::PlusOnePlusOne), 2, "the Horde's effect first: a counter comes off");
    assert_eq!(spent.get_object(horde).unwrap().damage_marked, 0);
    assert_eq!(spent.get_object(brute).unwrap().damage_marked, 3);
}

// ---------------------------------------------------------------------------
// A batch is dealt together, after every choice in it.
// ---------------------------------------------------------------------------

/// Two attackers, one of them the subject of a choice: none of the step's
/// damage lands until the choice is made, then all of it does (CR 510.2).
#[test]
fn the_rest_of_the_step_waits_for_the_choice() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
    state.get_object_mut(flail).unwrap().attached_to = Some(corpse);
    let brute = ready_creature(&mut state, P0, 3, 3);
    stock_library(&mut state, P1, 10);
    declare_combat(&mut state, &[(corpse, P1, &[]), (brute, P1, &[])]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert!(effect_prompt(&state).is_some());
    assert_eq!(state.get_player(P1).life, 20, "the brute's 3 waits with the Corpse's choice");
    assert_eq!(state.pending_damage.len(), 2);

    let state = choose(&state, &reg, "Undead Alchemist");
    assert_eq!(state.get_player(P1).life, 17, "then the brute's 3 lands");
    assert_eq!(library_size(&state, P1), 8);
    assert!(state.pending_damage.is_empty());
    assert!(state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::CombatDamageDealt { source, target: DamageTarget::Player(P1), amount: 3 }
            if *source == brute)), "the brute's damage is an event of the answering action");
}

/// Through the turn machinery: the combat damage step raises the prompt,
/// the answer deals the damage, and the invariant checker is content with
/// the state on both sides of it.
#[test]
fn the_combat_damage_step_waits_on_the_prompt_and_the_checker_agrees() {
    let (mut state, reg, _) = flail_and_alchemist();
    if let Some(c) = state.combat.as_mut() {
        c.any_attackers_declared = true;
    }

    mtg_engine::engine::advance_step(&mut state, &reg);

    assert_eq!(state.step, Step::CombatDamage);
    assert!(effect_prompt(&state).is_some(), "the step is waiting on the defender");
    let complaints: Vec<String> = mtg_engine::invariants::check_settled(&state, &reg).into_iter()
        .filter(|c| c.contains("damage"))
        .collect();
    assert!(complaints.is_empty(), "{complaints:?}");

    let state = choose(&state, &reg, "Undead Alchemist");
    assert!(state.awaiting_action.is_none());
    assert_eq!(library_size(&state, P1), 8);
    let complaints: Vec<String> = mtg_engine::invariants::check_settled(&state, &reg).into_iter()
        .filter(|c| c.contains("damage"))
        .collect();
    assert!(complaints.is_empty(), "{complaints:?}");
}

/// Damage left queued at a decision point with no choice open is a state
/// the checker refuses: it is damage the game has lost.
#[test]
fn the_checker_refuses_damage_left_waiting() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let source = ready_creature(&mut state, P0, 2, 2);
    state.pending_damage.push(mtg_engine::damage::PendingDamage {
        source, target: DamageTarget::Player(P1), amount: 2,
        kind: mtg_engine::damage::DamageKind::NonCombat, applied: vec![], settled: true,
    });

    let complaints = mtg_engine::invariants::check_settled(&state, &reg);
    assert!(complaints.iter().any(|c| c.contains("queued with no damage-effect choice open")), "{complaints:?}");
}

/// The prompt state survives a save and a load: the queue and the offered
/// effects are part of the game state, not of the process.
#[test]
fn the_prompt_and_the_queue_round_trip_through_a_save() {
    let (mut state, reg, _) = flail_and_alchemist();
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let json = serde_json::to_string(&state).expect("serializes");
    let loaded: GameState = serde_json::from_str(&json).expect("deserializes");

    assert_eq!(loaded.pending_damage, state.pending_damage);
    assert_eq!(effect_prompt(&loaded), effect_prompt(&state));
    let answered = choose(&loaded, &reg, "Undead Alchemist");
    assert_eq!(library_size(&answered, P1), 8);
}

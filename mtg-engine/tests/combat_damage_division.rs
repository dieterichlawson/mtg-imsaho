//! CR 510.1c-d: the attacking player divides a blocked attacker's combat
//! damage among its blockers (issue #637). Each blocker must be assigned
//! lethal damage before the next is assigned any, but the attacker may put
//! more than lethal on an earlier blocker, and a trampler may send nothing
//! past its blockers. The engine used to make that division itself.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat::divide_combat_damage;
use mtg_engine::ids::{ObjectId, PlayerId};
use mtg_engine::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use mtg_engine::types::*;

/// The open division prompt: (player, blocker, min, max, options, first-strike step).
fn division_prompt(state: &GameState) -> Option<(PlayerId, ObjectId, u32, u32, Vec<String>, bool)> {
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            player,
            choice: ResolutionChoiceKind::AssignCombatDamage { blocker, min, max, options, first_strike_only, .. },
            ..
        }) => Some((*player, *blocker, *min, *max, options.clone(), *first_strike_only)),
        _ => None,
    }
}

/// Answer the open division prompt by assigning `amount`.
fn assign(state: &GameState, amount: u32, registry: &CardRegistry) -> GameState {
    let (_, _, min, _, options, _) = division_prompt(state).expect("a division prompt is open");
    let index = (amount - min) as usize;
    mtg_engine::engine::submit_action(state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(index, options[index].clone()),
    }, registry)
}

/// A P0 attacker of `power` blocked by P1 creatures of `toughnesses`, in
/// that damage assignment order, at the start of the combat damage step.
fn blocked(power: i32, toughnesses: &[i32], keywords: &[Keyword], registry: &CardRegistry)
    -> (GameState, ObjectId, Vec<ObjectId>)
{
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, power, power);
    state.get_object_mut(attacker).unwrap().keywords.extend_from_slice(keywords);
    let blockers: Vec<ObjectId> = toughnesses.iter()
        .map(|&t| ready_creature(&mut state, P1, 1, t))
        .collect();
    submit_declare_attackers(&mut state, &[(attacker, P1)], registry);
    let blocks: Vec<(ObjectId, ObjectId)> = blockers.iter().map(|&b| (b, attacker)).collect();
    submit_declare_blockers(&mut state, P1, &blocks, registry);
    if blockers.len() > 1 {
        state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenOrder((0..blockers.len()).collect()),
        }, registry);
    }
    assert!(state.awaiting_action.is_none(), "blocks and order settled: {:?}", state.awaiting_action);
    state.priority_player = None;
    (state, attacker, blockers)
}

fn enter_damage_step(state: &mut GameState, registry: &CardRegistry) {
    mtg_engine::engine::advance_step(state, registry);
    assert_eq!(state.step, Step::CombatDamage);
}

/// The invariant checker has nothing to say about the combat or the
/// division. (The anonymous test creatures draw complaints of their own,
/// and state-based actions are the game loop's to run, not the answer's.)
fn assert_combat_settled(state: &GameState, registry: &CardRegistry) {
    let complaints: Vec<String> = mtg_engine::invariants::check_settled(state, registry).into_iter()
        .filter(|c| c.contains("division") || c.contains("combat") || c.contains("queued"))
        .collect();
    assert!(complaints.is_empty(), "{complaints:?}");
}

fn marked(state: &GameState, id: ObjectId) -> u32 {
    state.get_object(id).map_or(0, |o| o.damage_marked)
}

/// The issue's case: a 5-power attacker double-blocked by two 2-toughness
/// creatures has one damage to spare, and it is the attacking player's to
/// place. Asked at the first blocker, with lethal as the least.
#[test]
fn a_surplus_over_two_blockers_is_the_attacking_players_to_place() {
    let reg = registry();
    let (mut state, _, blockers) = blocked(5, &[2, 2], &[], &reg);
    enter_damage_step(&mut state, &reg);

    let (player, blocker, min, max, options, first_strike) = division_prompt(&state)
        .expect("the combat damage step asks how to divide the surplus");
    assert_eq!(player, P0, "the attacking player divides the damage (CR 510.1c)");
    assert_eq!(blocker, blockers[0], "asked in damage assignment order");
    assert_eq!((min, max), (2, 5), "at least lethal, at most everything");
    assert_eq!(options.len(), 4);
    assert!(!first_strike);
    assert_combat_settled(&state, &reg);
    assert_eq!(marked(&state, blockers[0]), 0, "nothing is dealt while the division is open");

    // All five on the first: the second blocker gets nothing and lives.
    let state = assign(&state, 5, &reg);
    assert!(division_prompt(&state).is_none(), "nothing is left to divide");
    assert_eq!(marked(&state, blockers[0]), 5);
    assert_eq!(marked(&state, blockers[1]), 0, "over-assigning the first blocker is legal (CR 510.1c)");
    assert!(state.game_log.iter().any(|e| e.message.contains("assigned 5 of")),
        "the division is logged");
    assert_combat_settled(&state, &reg);
}

/// Index 0 is today's answer, so a seat answering with the minimum changes
/// nothing: lethal to the first, the rest to the second.
#[test]
fn the_default_answer_is_exactly_lethal_then_forward() {
    let reg = registry();
    let (mut state, _, blockers) = blocked(5, &[2, 2], &[], &reg);
    enter_damage_step(&mut state, &reg);
    let state = assign(&state, 2, &reg);
    assert!(division_prompt(&state).is_none(), "the last blocker without trample takes the rest");
    assert_eq!((marked(&state, blockers[0]), marked(&state, blockers[1])), (2, 3));
}

/// More than lethal for the first blocker is a question even when it is no
/// more than the blockers' total lethal. 4 power against two 2-toughness
/// blockers may go 2-2, 3-1 or 4-0 (CR 510.1c: the second blocker may be
/// assigned damage once the first has lethal, and need not be); the engine
/// used to assign 2-2 silently, as if it were the only division, and the
/// completeness proof in `mtg-kernel` found the two it never offered.
#[test]
fn more_than_the_first_blockers_lethal_is_a_question_even_short_of_the_total() {
    let reg = registry();
    for (answer, want) in [(2, (2, 2)), (3, (3, 1)), (4, (4, 0))] {
        let (mut state, _, blockers) = blocked(4, &[2, 2], &[], &reg);
        enter_damage_step(&mut state, &reg);
        let (_, blocker, min, max, _, _) = division_prompt(&state).expect("4 power over a lethal of 2 is a choice");
        assert_eq!((blocker, min, max), (blockers[0], 2, 4));
        let state = assign(&state, answer, &reg);
        assert!(division_prompt(&state).is_none(), "the last blocker without trample takes the rest");
        assert_eq!((marked(&state, blockers[0]), marked(&state, blockers[1])), want);
    }
}

/// No choice, no question: nothing left over after the first blocker's
/// lethal, or a lone blocker without trample, which takes everything.
#[test]
fn no_choice_is_no_question() {
    let reg = registry();
    let (mut state, _, blockers) = blocked(2, &[2, 2], &[], &reg);
    enter_damage_step(&mut state, &reg);
    assert!(division_prompt(&state).is_none(), "exactly lethal for the first and nothing left");
    assert_eq!((marked(&state, blockers[0]), marked(&state, blockers[1])), (2, 0));

    let (mut state, _, blockers) = blocked(5, &[2], &[], &reg);
    enter_damage_step(&mut state, &reg);
    assert!(division_prompt(&state).is_none(), "a lone blocker without trample is no choice");
    assert_eq!(marked(&state, blockers[0]), 5);
}

/// CR 702.19b: a trampler may assign more than lethal to its blocker and
/// trample over less — or nothing.
#[test]
fn a_trampler_may_keep_its_damage_on_the_blocker() {
    let reg = registry();
    let (mut state, _, blockers) = blocked(5, &[2], &[Keyword::Trample], &reg);
    enter_damage_step(&mut state, &reg);
    let (_, blocker, min, max, ..) = division_prompt(&state).expect("a trampler with a surplus is asked");
    assert_eq!((blocker, min, max), (blockers[0], 2, 5));

    let kept = assign(&state, 5, &reg);
    assert_eq!(marked(&kept, blockers[0]), 5);
    assert_eq!(kept.players[1].life, 20, "nothing trampled over");

    let default = assign(&state, 2, &reg);
    assert_eq!(marked(&default, blockers[0]), 2);
    assert_eq!(default.players[1].life, 17, "the default tramples the rest over");
}

/// An out-of-range answer is refused and the prompt stands.
#[test]
fn an_amount_outside_the_range_is_refused() {
    let reg = registry();
    let (mut state, ..) = blocked(5, &[2, 2], &[], &reg);
    enter_damage_step(&mut state, &reg);
    let refused = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(4, "6".into()),
    }, &reg);
    assert!(division_prompt(&refused).is_some(), "the prompt is still open");
}

/// Each damage step divides its own damage, against what is already
/// marked (CR 510.4): a double striker is asked in the first-strike step,
/// and its answer does not carry into the regular one.
#[test]
fn each_damage_step_asks_its_own_division() {
    let reg = registry();
    let (mut state, _, blockers) = blocked(5, &[2, 2], &[Keyword::DoubleStrike], &reg);
    enter_damage_step(&mut state, &reg);
    let (_, blocker, min, max, _, first_strike) = division_prompt(&state)
        .expect("the first-strike step asks");
    assert!(first_strike);
    assert_eq!((blocker, min, max), (blockers[0], 2, 5));
    let state = assign(&state, 2, &reg);
    assert_eq!((marked(&state, blockers[0]), marked(&state, blockers[1])), (2, 3));
    assert!(state.combat.as_ref().unwrap().chosen_damage.is_empty(), "the step spent its answers");

}

/// The regular step's lethal counts what the first-strike step marked
/// (CR 510.1c, 510.4): a 4-power double-striking trampler blocked by a
/// 6-toughness creature has no surplus in the first step, and 2 of it to
/// place in the second.
#[test]
fn the_regular_step_measures_lethal_against_damage_already_marked() {
    let reg = registry();
    let (mut state, _, blockers) = blocked(4, &[6], &[Keyword::DoubleStrike, Keyword::Trample], &reg);
    enter_damage_step(&mut state, &reg);
    assert!(division_prompt(&state).is_none(), "4 is short of lethal for a 6-toughness blocker");
    assert_eq!(marked(&state, blockers[0]), 4);

    state.priority_player = None;
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    let (_, blocker, min, max, _, first_strike) = division_prompt(&state)
        .expect("the regular step has a surplus to place");
    assert!(!first_strike);
    assert_eq!((blocker, min, max), (blockers[0], 2, 4), "lethal is what the first step left");
    let state = assign(&state, 3, &reg);
    assert_eq!(marked(&state, blockers[0]), 7);
    assert_eq!(state.players[1].life, 19, "the one not assigned trampled over");
}

/// The division, over every small board: whatever the attacking player
/// chooses, CR 510.1c holds — no blocker is assigned damage while an
/// earlier one is short of lethal, all the power is assigned somewhere,
/// and only a trampler sends any past its blockers — and every division
/// the rules allow is one the player can choose. With nothing chosen, the
/// division is exactly lethal and forward. (`mtg-kernel` proves all of this
/// for every size; this checks the engine's call into it.)
#[test]
fn every_division_obeys_cr_510_1c() {
    let ids: Vec<ObjectId> = (1..=3).map(ObjectId).collect();
    for power in 0..=7u32 {
        for n in 0..=3usize {
            for code in 0..5u32.pow(n as u32) {
                let lethals: Vec<u32> = (0..n).map(|i| (code / 5u32.pow(i as u32)) % 5).collect();
                let blockers: Vec<(ObjectId, u32)> = ids.iter().copied().zip(lethals.iter().copied()).collect();
                for trample in [false, true] {
                    check_division(power, &blockers, trample);
                }
            }
        }
    }
}

/// CR 510.1c-d, stated directly: is `amounts` (with `overflow` past the
/// blockers) a legal division of `power`?
fn is_legal(power: u32, blockers: &[(ObjectId, u32)], trample: bool, amounts: &[u32], overflow: u32) -> bool {
    let assigned: u32 = amounts.iter().sum();
    let total_ok = if blockers.is_empty() && !trample { assigned + overflow == 0 } else { assigned + overflow == power };
    let order_ok = (0..amounts.len()).all(|k| amounts[k] == 0 || (0..k).all(|j| amounts[j] >= blockers[j].1));
    let overflow_ok = overflow == 0 || (trample && amounts.iter().zip(blockers).all(|(&a, &(_, l))| a >= l));
    total_ok && order_ok && overflow_ok
}

fn check_division(power: u32, blockers: &[(ObjectId, u32)], trample: bool) {
    let what = format!("power {power}, lethals {blockers:?}, trample {trample}");
    let choice = !blockers.is_empty() && (trample || blockers.len() >= 2) && power > blockers[0].1;

    // The default: exactly lethal, the rest forward.
    let d = divide_combat_damage(power, blockers, trample, &[]);
    let mut left = power;
    for (k, (&(b, lethal), &(db, amount))) in blockers.iter().zip(&d.to_blockers).enumerate() {
        assert_eq!(b, db, "{what}");
        let want = if k + 1 == blockers.len() && !trample { left } else { left.min(lethal) };
        assert_eq!(amount, want, "default for blocker {k}: {what}");
        left -= amount;
    }
    assert_eq!(d.open.is_some(), choice, "asked iff there is a choice: {what}");

    // Every walk of answers the prompts allow: each ends legal, and between
    // them they reach every legal division.
    let mut reached = std::collections::BTreeSet::new();
    let mut stack: Vec<Vec<(ObjectId, u32)>> = vec![vec![]];
    while let Some(chosen) = stack.pop() {
        let d = divide_combat_damage(power, blockers, trample, &chosen);
        let amounts: Vec<u32> = d.to_blockers.iter().map(|b| b.1).collect();
        assert!(is_legal(power, blockers, trample, &amounts, d.overflow),
            "an illegal division: {what} {chosen:?} {d:?}");
        if let Some((b, min, max)) = d.open {
            assert!(min < max, "an open division is a choice: {what}");
            for a in min..=max {
                let mut next = chosen.clone();
                next.push((b, a));
                stack.push(next);
            }
        } else {
            reached.insert((amounts, d.overflow));
        }
    }
    let mut legal = std::collections::BTreeSet::new();
    for code in 0..(power + 1).pow(blockers.len() as u32) {
        let amounts: Vec<u32> = (0..blockers.len()).map(|i| (code / (power + 1).pow(i as u32)) % (power + 1)).collect();
        let assigned: u32 = amounts.iter().sum();
        if assigned > power {
            continue;
        }
        let overflow = if trample { power - assigned } else { 0 };
        if is_legal(power, blockers, trample, &amounts, overflow) {
            legal.insert((amounts, overflow));
        }
    }
    assert_eq!(reached, legal, "the divisions the prompts reach are the legal ones: {what}");
}

/// The open division prompt's description.
fn division_description(state: &GameState) -> String {
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::AssignCombatDamage { description, .. }, ..
        }) => description.clone(),
        other => panic!("no division prompt is open: {other:?}"),
    }
}

/// CR 510.4: an attacker without first or double strike deals no damage in
/// the first-strike step, so it has nothing to divide there. A first-strike
/// *blocker* makes the step happen; the attacker's division is asked once,
/// in the regular step.
#[test]
fn an_attacker_without_first_strike_divides_only_in_the_regular_step() {
    let reg = registry();
    let (mut state, attacker, blockers) = blocked(5, &[2, 2], &[], &reg);
    grant_keyword(&mut state, blockers[0], Keyword::FirstStrike);
    enter_damage_step(&mut state, &reg);
    assert!(division_prompt(&state).is_none(),
        "the first-strike step asked the attacker how to divide damage it does not deal: {:?}",
        state.awaiting_action);
    assert_eq!(marked(&state, attacker), 1, "the first-striking blocker dealt its damage");

    state.priority_player = None;
    mtg_engine::engine::advance_step(&mut state, &reg);
    let (_, blocker, min, max, _, first_strike) = division_prompt(&state)
        .expect("the regular step asks the division");
    assert!(!first_strike);
    assert_eq!((blocker, min, max), (blockers[0], 2, 5));
}

/// The question says where a surplus goes: on to the next blocker in the
/// order, and past the last one to what a trampler attacks. The least
/// amount, and only it, is the lethal row.
#[test]
fn the_division_prompt_names_where_the_rest_goes() {
    let reg = registry();
    let (mut state, _, blockers) = blocked(6, &[2, 2, 2], &[], &reg);
    enter_damage_step(&mut state, &reg);
    let (_, blocker, min, _, options, _) = division_prompt(&state).expect("a surplus to place");
    assert_eq!(blocker, blockers[0]);
    let next = state.obj_name(blockers[1]);
    let description = division_description(&state);
    assert!(description.ends_with(&format!("the rest goes on to {next}")),
        "the rest goes to the second blocker: {description}");
    assert!(options.iter().all(|o| o.ends_with(&format!("on to {next}"))), "{options:?}");
    for (amount, option) in (min..).zip(&options) {
        assert_eq!(option.contains("(lethal)"), amount == min, "{options:?}");
    }

    let (mut state, _, _) = blocked(5, &[2], &[Keyword::Trample], &reg);
    enter_damage_step(&mut state, &reg);
    let description = division_description(&state);
    assert!(description.ends_with("the rest goes on to p1"),
        "past its last blocker a trampler's rest goes to the defending player: {description}");
}

/// Two blockers that read the same in the order prompt are told apart by
/// their ids; a blocker whose label is already unique is not cluttered
/// with one. Every label carries the P/T it is ordered by.
#[test]
fn only_blockers_that_read_alike_carry_an_id_in_the_order_prompt() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 6, 6);
    let twins = [ready_creature(&mut state, P1, 2, 2), ready_creature(&mut state, P1, 2, 2)];
    let odd = ready_creature(&mut state, P1, 1, 3);
    for &b in &twins {
        state.get_object_mut(b).unwrap().name = "Twin".into();
    }
    state.get_object_mut(odd).unwrap().name = "Odd".into();
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(twins[0], attacker), (twins[1], attacker), (odd, attacker)], &reg);
    let (remaining, options) = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseDamageAssignmentOrder { remaining, options, .. }, ..
        }) => (remaining.clone(), options.clone()),
        other => panic!("three blockers are an order to announce: {other:?}"),
    };
    for (id, label) in remaining.iter().zip(&options) {
        if *id == odd {
            assert_eq!(label, "Odd 1/3", "a unique label needs no id: {options:?}");
        } else {
            assert_eq!(label, &format!("Twin 2/2 [#{}]", id.0), "{options:?}");
        }
    }
}

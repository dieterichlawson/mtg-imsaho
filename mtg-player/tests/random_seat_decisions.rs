//! The random seat rolls a DECISION, then a way of taking it (#472), on real
//! boards the engine enumerates. These cases hold its grouping key to the
//! rules' idea of one decision where the engine emits one entry per slot or
//! per copy.

#[path = "../../mtg-engine/tests/common/mod.rs"]
#[allow(dead_code, unused_imports)]
mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::legal_actions;
use mtg_engine::types::*;
use mtg_engine::view::GameView;
use mtg_player::random::RandomPlayer;
use mtg_player::Player;
use std::collections::BTreeMap;

const DRAWS: usize = 6000;

fn main_phase() -> (GameState, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    state.priority_player = Some(P0);
    (state, reg)
}

/// The share of draws each label takes.
fn shares(state: &GameState, reg: &CardRegistry, label: impl Fn(&Action) -> String) -> BTreeMap<String, f64> {
    let legal = legal_actions(state, reg);
    let view = GameView::for_player(state, P0, reg);
    let mut seat = RandomPlayer::with_seed("r", 42);
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for _ in 0..DRAWS {
        *out.entry(label(&seat.choose_action(&view, &legal))).or_insert(0) += 1;
    }
    out.into_iter().map(|(k, v)| (k, v as f64 / DRAWS as f64)).collect()
}

fn about(share: f64, expected: f64) -> bool {
    (share - expected).abs() < 0.05
}

/// Issue #664: a loyalty ability is one decision whatever it targets (CR
/// 606.3, 602.2b). The engine emits one `ActivateLoyaltyAbility` per
/// target, and keyed on the entry Garruk Relentless's fight took 80% of
/// the draws on an eight-creature board against his Wolf's 10%.
#[test]
fn a_loyalty_ability_is_one_decision_however_many_targets_it_has() {
    let (mut state, reg) = main_phase();
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    for _ in 0..4 { named_permanent(&mut state, &reg, "Grizzly Bears", P0); }
    for _ in 0..4 { named_permanent(&mut state, &reg, "Grizzly Bears", P1); }
    let s = shares(&state, &reg, |a| match a {
        Action::PassPriority => "pass".into(),
        Action::ActivateLoyaltyAbility { object_id, ability_index, .. } if *object_id == garruk =>
            format!("ability {ability_index}"),
        Action::Concede => "concede".into(),
        other => format!("other {other:?}"),
    });
    assert_eq!(s.len(), 3, "pass and Garruk's two abilities, nothing else: {s:?}");
    for (k, v) in &s {
        assert!(about(*v, 1.0 / 3.0), "{k} is one of three decisions, not {v:.3}: {s:?}");
    }
}

/// Issue #667: which copy of a card in hand is cast is not a decision. The
/// engine already offers an untargeted spell once however many copies are
/// held; a targeted one is offered per copy and per target, and keyed on
/// the object three Lightning Bolts took 60% of draws against three
/// Kalonian Tuskers' 20%.
#[test]
fn copies_of_a_targeted_spell_in_hand_are_one_decision() {
    let (mut state, reg) = main_phase();
    for _ in 0..3 { castable_spell(&mut state, &reg, "Lightning Bolt", P0); }
    for _ in 0..3 { castable_spell(&mut state, &reg, "Kalonian Tusker", P0); }
    let s = shares(&state, &reg, |a| match a {
        Action::PassPriority => "pass".into(),
        Action::CastSpell { object_id, .. } => format!("cast {}", state.get_object(*object_id).unwrap().name),
        Action::Concede => "concede".into(),
        other => format!("other {other:?}"),
    });
    assert_eq!(s.len(), 3, "{s:?}");
    for (k, v) in &s {
        assert!(about(*v, 1.0 / 3.0), "{k} is one of three decisions, not {v:.3}: {s:?}");
    }
}

/// Issue #665: every interactive seat answers an ordering prompt with the
/// whole order (`ChosenOrder`, #325), and the engine runs that answer
/// through arms of its own. The random seat answered only from
/// `legal.actions`, which enumerates "this one next", so no fuzz game
/// reached the whole-order path. It now rolls both shapes, and the order
/// it sends is one the engine accepts (CR 509.2).
#[test]
fn an_ordering_prompt_is_sometimes_answered_with_the_whole_order() {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::engine::submit_action;
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blockers: Vec<_> = (0..4).map(|_| ready_creature(&mut state, P1, 1, 1)).collect();
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    let pairs: Vec<_> = blockers.iter().map(|&b| (b, attacker)).collect();
    submit_declare_blockers(&mut state, P1, &pairs, &reg);
    let legal = legal_actions(&state, &reg);
    assert!(matches!(legal.resolution_prompt,
        Some(mtg_engine::state::ResolutionChoiceKind::ChooseDamageAssignmentOrder { .. })), "{:?}", legal.resolution_prompt);

    let view = GameView::for_player(&state, P0, &reg);
    let mut seat = RandomPlayer::with_seed("r", 1);
    let (mut one, mut whole) = (0, 0);
    for _ in 0..400 {
        let answer = seat.choose_action(&view, &legal);
        match &answer {
            Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(..) } => one += 1,
            Action::ResolveChoice { choice: ResolvedChoice::ChosenOrder(order) } => {
                whole += 1;
                let after = submit_action(&state, &answer, &reg);
                let placed = after.combat.as_ref()
                    .and_then(|c| c.damage_assignment_order.get(&attacker))
                    .cloned().unwrap_or_default();
                let expected: Vec<_> = order.iter().map(|&i| blockers[i]).collect();
                assert_eq!(placed, expected, "the engine took the order {order:?} as sent");
            }
            other => panic!("an ordering answer, not {other:?}"),
        }
    }
    assert!(one > 100 && whole > 100,
        "both answer shapes are rolled: {one} one-at-a-time, {whole} whole orders");
}

/// Issue #666: a modal set's count is its mode, and each mode names from its
/// own candidates. Rolling a count over the union, the seat drew a legal
/// two-Zombie pair for Ghoulcaller's Chant 1 time in 28 and nearly half its
/// answers were refused. It now rolls the mode and names from that mode's
/// list, so every answer it gives is a cast.
#[test]
fn a_modal_set_is_answered_one_mode_at_a_time() {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::engine::submit_action;
    let (mut state, reg) = main_phase();
    named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    for _ in 0..6 { named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0); }
    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let asked = submit_action(&state, &cast_action(chant, vec![]), &reg);
    let legal = legal_actions(&asked, &reg);
    let view = GameView::for_player(&asked, P0, &reg);
    let mut seat = RandomPlayer::with_seed("r", 42);
    let mut modes: BTreeMap<String, usize> = BTreeMap::new();
    for _ in 0..600 {
        let answer = seat.choose_action(&view, &legal);
        if matches!(answer, Action::ResolveChoice { choice: ResolvedChoice::CancelCast }) {
            continue;
        }
        let after = submit_action(&asked, &answer, &reg);
        let key = match after.get_object(chant) {
            Some(o) if o.zone == Zone::Stack => format!("mode {:?}", o.chosen_mode),
            _ => format!("refused {answer:?}"),
        };
        *modes.entry(key).or_insert(0) += 1;
    }
    assert!(modes.keys().all(|k| k.starts_with("mode")), "every answer is a cast: {modes:?}");
    let two = modes.get("mode Some(1)").copied().unwrap_or(0);
    assert!(two > 150, "mode two is one of two modes, not a 1-in-28 accident: {modes:?}");
}

/// Issue #637: dividing combat damage among blockers (CR 510.1c-d) is
/// rolled over the whole range, not answered with the minimum. Index 0 is
/// exactly lethal — the division the engine makes without asking — so a
/// seat that took it would never put more than lethal on a blocker, and
/// the fuzzer would never reach the over-assignment the prompt exists for.
#[test]
fn a_combat_damage_division_is_rolled_over_its_whole_range() {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::engine::submit_action;
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 6, 6);
    let first = ready_creature(&mut state, P1, 1, 2);
    let second = ready_creature(&mut state, P1, 1, 2);
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(first, attacker), (second, attacker)], &reg);
    state = submit_action(&state, &Action::ResolveChoice { choice: ResolvedChoice::ChosenOrder(vec![0, 1]) }, &reg);
    state.priority_player = None;
    mtg_engine::engine::advance_step(&mut state, &reg);
    let legal = legal_actions(&state, &reg);
    let Some(mtg_engine::state::ResolutionChoiceKind::AssignCombatDamage { min, max, .. }) = legal.resolution_prompt
    else { panic!("the damage step asks for the division: {:?}", legal.resolution_prompt) };
    assert_eq!((min, max), (2, 6));

    let view = GameView::for_player(&state, P0, &reg);
    let mut seat = RandomPlayer::with_seed("r", 3);
    let mut seen = std::collections::BTreeMap::<u32, usize>::new();
    for _ in 0..2000 {
        let answer = seat.choose_action(&view, &legal);
        let after = submit_action(&state, &answer, &reg);
        assert!(after.awaiting_action.is_none(), "the engine took {answer:?}: {:?}", after.awaiting_action);
        *seen.entry(after.get_object(first).unwrap().damage_marked).or_default() += 1;
    }
    assert_eq!(seen.keys().copied().collect::<Vec<_>>(), (2..=6).collect::<Vec<_>>(),
        "every amount from lethal to all of it is reached: {seen:?}");
    for (amount, n) in &seen {
        assert!(about(*n as f64 / 2000.0, 0.2), "{amount} is one of five amounts, drawn {n}/2000: {seen:?}");
    }
}

/// Issue #670: Skirsdag High Priest's "tap two untapped creatures you
/// control" is asked as one set. The seat activates it as one decision,
/// answers with a random pair the engine takes — not always the same two —
/// and now and then backs out, which is an engine path of its own.
#[test]
fn a_tap_creatures_cost_is_answered_with_a_random_pair_or_backed_out_of() {
    use mtg_engine::engine::submit_action;
    let (mut state, reg) = main_phase();
    let priest = named_permanent(&mut state, &reg, "Skirsdag High Priest", P0);
    let others: Vec<_> = (0..4).map(|_| named_permanent(&mut state, &reg, "Grizzly Bears", P0)).collect();
    state.creature_died_this_turn = true;

    let offers: Vec<Action> = legal_actions(&state, &reg).actions.into_iter()
        .filter(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == priest))
        .collect();
    assert_eq!(offers.len(), 1, "one decision, not one per pair");
    let asked = submit_action(&state, &offers[0], &reg);
    let legal = legal_actions(&asked, &reg);
    assert!(matches!(legal.resolution_prompt,
        Some(mtg_engine::state::ResolutionChoiceKind::ChooseObjectSet { .. })), "{:?}", legal.resolution_prompt);

    let view = GameView::for_player(&asked, P0, &reg);
    let mut seat = RandomPlayer::with_seed("r", 9);
    let (mut paid, mut cancelled) = (0, 0);
    let mut pairs = std::collections::BTreeSet::new();
    for _ in 0..300 {
        let answer = seat.choose_action(&view, &legal);
        let after = submit_action(&asked, &answer, &reg);
        assert!(after.awaiting_action.is_none(), "the engine took {answer:?}");
        if after.stack.is_empty() {
            cancelled += 1;
            assert!(!after.get_object(priest).unwrap().tapped, "a cancel pays nothing");
        } else {
            paid += 1;
            let tapped: Vec<_> = others.iter().copied().filter(|&o| after.get_object(o).unwrap().tapped).collect();
            assert_eq!(tapped.len(), 2);
            pairs.insert(tapped);
        }
    }
    assert!(paid > 200 && cancelled > 0, "paid {paid}, cancelled {cancelled}");
    assert_eq!(pairs.len(), 6, "every pair of the four is reached: {pairs:?}");
}

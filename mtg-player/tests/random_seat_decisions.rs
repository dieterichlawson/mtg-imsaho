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

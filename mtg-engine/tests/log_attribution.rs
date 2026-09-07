//! What the game log has to be able to explain.
//!
//! The log is the only record a resumed seat, an LLM reading the recap, or a
//! human reading `--log` afterwards has. Four events used to leave it unable
//! to answer an obvious question:
//!
//! * a sacrifice paid as a cost said only `<name> died` — not that it was a
//!   sacrifice, not who sacrificed it, and with two eligible creatures not
//!   which one paid (issue #263);
//! * an alternative-cost cast read exactly like a paid one, so a Rooftop
//!   Storm free cast and a hard cast were the same line (issue #264);
//! * "enters with N counters" said nothing at all, so a permanent entering
//!   with 0 counters was `resolved` then `died` with no cause (issue #299);
//! * a random discard said nothing, so a card moved hand→graveyard and the
//!   log said only "drew 2 cards" (issue #301).

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::state::GameState;
use mtg_engine::types::*;

fn log_lines(state: &GameState) -> Vec<String> {
    state.game_log.iter().map(|e| e.message.clone()).collect()
}

fn index_of(lines: &[String], needle: &str) -> Option<usize> {
    lines.iter().position(|l| l.contains(needle))
}

fn assert_line(lines: &[String], needle: &str) {
    assert!(index_of(lines, needle).is_some(),
        "expected a line containing {needle:?}; log was {lines:#?}");
}

// ---------------------------------------------------------------------------
// #263 — a sacrifice names the sacrificing player, the permanent and the cost
// ---------------------------------------------------------------------------

/// Selfless Cathar: "{1}{W}, Sacrifice this creature: Creatures you control
/// get +1/+1 until end of turn." The only trace used to be `Selfless Cathar
/// died`.
#[test]
fn an_abilitys_own_sacrifice_cost_names_the_player_and_the_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cathar = named_permanent(&mut state, &reg, "Selfless Cathar", P0);
    add_mana(&mut state, P0, &[(ManaType::White, 1), (ManaType::Colorless, 1)]);

    let after = activate(&state, &reg, cathar, 0, vec![]);
    let lines = log_lines(&after);

    assert_line(&lines, "p0 sacrificed Selfless Cathar");
    assert_line(&lines, "to pay for its own ability");
    // CR 701.17a is "a player sacrifices a permanent they control": the
    // sacrifice is announced before the permanent leaves, so the log stops
    // reading "it died, and then it was sacrificed".
    let sac = index_of(&lines, "p0 sacrificed Selfless Cathar").expect("sacrifice line");
    let died = index_of(&lines, "Selfless Cathar died").expect("death line");
    assert!(sac < died, "the sacrifice is announced before the death; log was {lines:#?}");
}

/// "Sacrifice a creature" with more than one eligible creature: the action
/// menu said which one would pay and the log did not.
#[test]
fn a_sacrifice_a_creature_cost_names_the_creature_that_paid() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    let bystander = named_permanent(&mut state, &reg, "Ashmouth Hound", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 1)]);

    let after = activate_sacrificing(
        &state, &reg, cultist, 0, vec![Target::Player(P1)], bystander);
    let lines = log_lines(&after);

    assert_line(&lines, "p0 sacrificed Ashmouth Hound");
    assert_line(&lines, "to pay for Skirsdag Cultist");
    assert!(!lines.iter().any(|l| l.contains("Skirsdag Cultist died")),
        "the Cultist did not pay; log was {lines:#?}");
}

/// The spell path already had a line, but printed it after the death line.
#[test]
fn an_additional_cost_sacrifice_is_announced_before_the_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hound = named_permanent(&mut state, &reg, "Ashmouth Hound", P0);
    let plunge = spell_in_hand(&mut state, &reg, "Infernal Plunge", P0);
    add_mana_for(&mut state, &reg, "Infernal Plunge", P0);

    let after = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: plunge, targets: vec![], sacrifice: Some(hound),
            exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
        },
        &reg,
    );
    let lines = log_lines(&after);

    assert_line(&lines, "p0 sacrificed Ashmouth Hound");
    assert_line(&lines, "as an additional cost of Infernal Plunge");
    let sac = index_of(&lines, "p0 sacrificed Ashmouth Hound").expect("sacrifice line");
    let died = index_of(&lines, "Ashmouth Hound died").expect("death line");
    assert!(sac < died, "the sacrifice is announced before the death; log was {lines:#?}");
}

// ---------------------------------------------------------------------------
// #299 — "enters with N counters", including N = 0
// ---------------------------------------------------------------------------

/// Unbreathing Horde with nothing to count enters as a 0/0 and dies to CR
/// 704.5f. Without this line the log is "resolved" then "died", with no
/// cause anywhere in between.
#[test]
fn entering_with_zero_counters_is_still_announced() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let horde = spell_in_hand(&mut state, &reg, "Unbreathing Horde", P0);
    state.move_object(horde, Zone::Battlefield, &reg);

    let lines = log_lines(&state);
    assert_line(&lines, "enters with 0 +1/+1 counters");
    assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 0,
        "no counters were actually added");
}

#[test]
fn entering_with_one_counter_says_counter_not_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    let horde = spell_in_hand(&mut state, &reg, "Unbreathing Horde", P0);
    state.move_object(horde, Zone::Battlefield, &reg);

    assert_line(&log_lines(&state), "enters with 1 +1/+1 counter");
    assert!(!log_lines(&state).iter().any(|l| l.contains("1 +1/+1 counters")),
        "one counter, singular");
}

#[test]
fn entering_with_counters_reports_the_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    let horde = spell_in_hand(&mut state, &reg, "Unbreathing Horde", P0);
    state.move_object(horde, Zone::Battlefield, &reg);

    let lines = log_lines(&state);
    assert_line(&lines, "enters with 2 +1/+1 counters");
    assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 2);
}

/// Somberwald Spider's counters are conditional ("if a creature died this
/// turn"), so with no creature dead the replacement does not apply at all and
/// there is nothing to announce.
#[test]
fn a_replacement_that_does_not_apply_says_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let spider = spell_in_hand(&mut state, &reg, "Somberwald Spider", P0);
    state.move_object(spider, Zone::Battlefield, &reg);

    let lines = log_lines(&state);
    assert!(!lines.iter().any(|l| l.contains("enters with")),
        "no enters-with-counters replacement applied; log was {lines:#?}");
}

// ---------------------------------------------------------------------------
// #301 — every discard writes a line, at the chokepoint
// ---------------------------------------------------------------------------

/// Desperate Ravings: "Draw two cards, then discard a card at random." The
/// discard was the one path in the engine that logged nothing.
#[test]
fn a_random_discard_names_the_card_and_the_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    stock_library(&mut state, &reg, P0, 5);
    let ravings = spell_in_hand(&mut state, &reg, "Desperate Ravings", P0);
    add_mana_for(&mut state, &reg, "Desperate Ravings", P0);

    let after = cast_and_resolve(&state, &reg, ravings, vec![]);
    let lines = log_lines(&after);

    assert_line(&lines, "Desperate Ravings (at random): p0 discarded");
}

/// One line per discard, written where the card moves — the cleanup path used
/// to write a second set beside it, so every hand-size discard appeared twice.
#[test]
fn a_discard_is_logged_exactly_once() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let card = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);

    state.discard_card(card, &reg);
    let lines = log_lines(&state);

    let discards: Vec<&String> = lines.iter()
        .filter(|l| l.contains("discarded")).collect();
    assert_eq!(discards.len(), 1, "exactly one discard line; got {discards:#?}");
    assert!(discards[0].contains("p0 discarded Walking Corpse"),
        "the line names the player and the card; got {discards:#?}");
}

// ---------------------------------------------------------------------------
// #264 — which cost was paid
// ---------------------------------------------------------------------------

/// Rooftop Storm: "You may pay {0} rather than pay the mana cost for Zombie
/// creature spells you cast." CR 118.9 makes that a distinct thing from
/// casting without paying the mana cost, so the log prints the cost.
#[test]
fn an_alternative_cost_cast_says_which_cost_was_paid() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Rooftop Storm", P0);
    let ghoul = spell_in_hand(&mut state, &reg, "Diregraf Ghoul", P0);

    let after = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: ghoul, targets: vec![], sacrifice: None, exile_count: None,
            exile_ids: vec![], alternative_cost: Some(ManaCost::free()),
            tap_plan: vec![],
        },
        &reg,
    );
    let lines = log_lines(&after);

    assert_line(&lines, "p0 cast Diregraf Ghoul");
    assert_line(&lines, "alternative cost");
}

/// End to end: the free cast the engine actually *offers* carries the marker,
/// not just a hand-built action. `legal_actions` is where the alternative
/// cost is attached, and the log line is what a reader gets.
#[test]
fn the_offered_free_cast_is_annotated_in_the_log() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Rooftop Storm", P0);
    let ghoul = spell_in_hand(&mut state, &reg, "Diregraf Ghoul", P0);

    let free_cast = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a,
            Action::CastSpell { object_id, alternative_cost: Some(_), .. } if *object_id == ghoul))
        .expect("Rooftop Storm offers a free cast for a Zombie creature spell");

    let after = mtg_engine::engine::submit_action(&state, &free_cast, &reg);
    let cast_line = log_lines(&after).into_iter()
        .find(|l| l.contains("cast Diregraf Ghoul"))
        .expect("cast line");

    assert!(cast_line.contains("alternative cost"),
        "the free cast is marked as one; got {cast_line:?}");
}

/// A hard cast of the same card carries no cost annotation — the marker has
/// to mean something.
#[test]
fn a_plain_cast_carries_no_cost_annotation() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ghoul = spell_in_hand(&mut state, &reg, "Diregraf Ghoul", P0);
    add_mana_for(&mut state, &reg, "Diregraf Ghoul", P0);

    let after = cast_onto_stack(&state, &reg, ghoul, vec![]);
    let cast_line = log_lines(&after).into_iter()
        .find(|l| l.contains("cast Diregraf Ghoul"))
        .expect("cast line");

    assert!(!cast_line.contains("alternative cost") && !cast_line.contains("flashback"),
        "a plain cast is unannotated; got {cast_line:?}");
}

/// A cost *reduction* had the same weakness in a milder form: the reader had
/// to count the tap lines to notice the {2} came off.
#[test]
fn a_reduced_cast_reports_what_was_actually_paid() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Heartless Summoning", P0);
    let corpse = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    add_mana(&mut state, P0, &[(ManaType::Black, 1)]);

    let after = cast_onto_stack(&state, &reg, corpse, vec![]);
    let cast_line = log_lines(&after).into_iter()
        .find(|l| l.contains("cast Walking Corpse"))
        .expect("cast line");

    assert!(cast_line.contains("reduced from"),
        "Heartless Summoning took {{2}} off; got {cast_line:?}");
}

/// A cast from the graveyard under the card's own permission (CR 601.3a) is
/// not an alternative cost — Skaab Ruinator pays its printed {1}{U}{U}. The
/// menu row said "(alternative cost {1}{U}{U})", which is the card's own cost
/// announced as a replacement for itself, and the log recorded it exactly as
/// a hand cast (issue #300).
#[test]
fn a_graveyard_cast_names_its_zone_and_claims_no_alternative_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }
    add_mana(&mut state, P0, &[(ManaType::Blue, 2), (ManaType::Colorless, 1)]);

    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == ruinator)
        .expect("the graveyard copy is castable");

    assert!(cs.from_graveyard, "the row has to say which zone it casts from");
    assert!(!cs.is_flashback, "this is permission to cast, not flashback");
    assert!(cs.alternative_cost.is_none(),
        "the printed mana cost is not an alternative cost; got {:?}", cs.alternative_cost);
    assert!(cs.additional_cost_label.is_some(),
        "the exile cost is what makes the two ways to cast non-interchangeable");

    // And the same in the log.
    let cast = legal.actions.iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == ruinator))
        .expect("a cast action for the graveyard copy")
        .clone();
    let after = mtg_engine::engine::submit_action(&state, &cast, &reg);
    let after = resolve_exile_choice_max_power(&after, &reg);
    let cast_line = log_lines(&after).into_iter()
        .find(|l| l.contains("cast Skaab Ruinator"))
        .expect("cast line");

    assert!(cast_line.contains("from graveyard"), "got {cast_line:?}");
    assert!(!cast_line.contains("alternative cost"), "got {cast_line:?}");
}

/// The same card in hand casts without the zone marker, so the two rows are
/// distinguishable — which is the whole point.
#[test]
fn a_hand_cast_of_the_same_card_says_nothing_about_a_zone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let in_hand = spell_in_hand(&mut state, &reg, "Skaab Ruinator", P0);
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }
    add_mana(&mut state, P0, &[(ManaType::Blue, 2), (ManaType::Colorless, 1)]);

    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == in_hand)
        .expect("the hand copy is castable");
    assert!(!cs.from_graveyard);
    assert!(cs.alternative_cost.is_none());
}

/// Flashback keeps its own name, and is not doubled up with the generic
/// alternative-cost marker.
#[test]
fn a_flashback_cast_still_says_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let think = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);
    stock_library(&mut state, &reg, P0, 3);
    add_mana(&mut state, P0, &[(ManaType::Blue, 1), (ManaType::Colorless, 2)]);

    let after = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: think, targets: vec![], sacrifice: None, exile_count: None,
            exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
        },
        &reg,
    );
    let cast_line = log_lines(&after).into_iter()
        .find(|l| l.contains("cast Think Twice"))
        .expect("cast line");

    assert!(cast_line.contains("(flashback)"), "got {cast_line:?}");
    assert!(!cast_line.contains("alternative cost"),
        "flashback is named once, not twice; got {cast_line:?}");
}

/// A mill that runs the library out says so.
///
/// Issue #86: the failed draw at the end of a mill race was silent, and the
/// loss it leads to (CR 704.5b) appeared from nowhere. The same is true one
/// step earlier — "mill three" against a one-card library is a mill of one
/// and the end of that library, and both halves are the news. A player
/// reading the log afterwards cannot reconstruct either from "milled 1 card".
#[test]
fn a_mill_says_when_the_library_ran_out_under_it() {
    let reg = registry();
    let cast_dream_twist = |state: &GameState| {
        let mut s = state.clone();
        let twist = castable_spell(&mut s, &reg, "Dream Twist", P0);
        let s = cast_and_resolve(&s, &reg, twist, vec![Target::Player(P1)]);
        log_lines(&s)
    };

    // One card against "mill three".
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let only = state.create_object(
        reg.get_id_by_name("Walking Corpse").unwrap(), P1, Zone::Library, None, None);
    state.get_object_mut(only).unwrap().name = "Walking Corpse".into();
    state.get_player_mut(P1).library_order.push(only);

    let lines = cast_dream_twist(&state);
    assert!(index_of(&lines, "Dream Twist: p1 milled 1 card (of 3 — library ran out)").is_some(),
        "the line says how many went AND that the library ran out under it: {lines:?}");

    // And a mill into an already-empty library is not silence either.
    let empty = game_at_step(Step::PrecombatMain, P0);
    let lines = cast_dream_twist(&empty);
    assert!(index_of(&lines, "Dream Twist: p1 has an empty library, nothing to mill").is_some(),
        "an empty library is the news, not a reason to say nothing: {lines:?}");

    // A mill that took every card it asked for carries no such aside.
    let mut full = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..3 {
        let id = full.create_object(
            reg.get_id_by_name("Walking Corpse").unwrap(), P1, Zone::Library, None, None);
        full.get_object_mut(id).unwrap().name = "Walking Corpse".into();
        full.get_player_mut(P1).library_order.push(id);
    }
    let lines = cast_dream_twist(&full);
    assert!(index_of(&lines, "Dream Twist: p1 milled 3 cards").is_some(), "{lines:?}");
    assert!(index_of(&lines, "library ran out").is_none(),
        "nothing ran out: {lines:?}");
}

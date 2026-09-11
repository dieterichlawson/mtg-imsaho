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
//!   log said only "drew 2 cards" (issue #301);
//! * a trigger-ordering decision among same-named triggers logged N
//!   byte-identical lines, a death line had no object id, and a counter
//!   placed by a resolving trigger was not logged at all — so the order the
//!   player chose under CR 603.3b could not be read back (issue #326).


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
    let died = index_of(&lines, &format!("Selfless Cathar (#{}) died", cathar.0)).expect("death line");

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
    assert!(!lines.iter().any(|l| l.contains("Skirsdag Cultist") && l.ends_with(" died")),
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
    let died = index_of(&lines, &format!("Ashmouth Hound (#{}) died", hound.0)).expect("death line");

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

/// A discard that runs the hand out says so — "discard two" against a
/// one-card hand is a discard of one, and the log is where a resumed seat or
/// an LLM reading the recap finds out which.
#[test]
fn a_discard_says_when_the_hand_ran_out_under_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // "Target player discards two cards" — Brain Weevil's is the two-card ask.
    spell_in_hand(&mut state, &reg, "Geistflame", P1);
    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);
    state.get_object_mut(weevil).unwrap().summoning_sick = false;

    mtg_engine::engine::discard_cards(&mut state, P1, 2, weevil, "Brain Weevil", &reg);
    let lines = log_lines(&state);
    assert!(index_of(&lines, "Brain Weevil: p1 discarded 1 of 2 — their hand ran out").is_some(),
        "one card went and the ask was two, and the log says both: {lines:?}");

    // An empty hand is its own line, not this one.
    state.game_log.clear();
    mtg_engine::engine::discard_cards(&mut state, P1, 2, weevil, "Brain Weevil", &reg);
    let lines = log_lines(&state);
    assert!(index_of(&lines, "Brain Weevil: p1 has no cards to discard").is_some(), "{lines:?}");
    assert!(index_of(&lines, "hand ran out").is_none(),
        "nothing ran out under a discard that never started: {lines:?}");

    // And a discard that took everything it asked for carries no such aside.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    spell_in_hand(&mut state, &reg, "Geistflame", P1);
    spell_in_hand(&mut state, &reg, "Dream Twist", P1);
    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);
    mtg_engine::engine::discard_cards(&mut state, P1, 2, weevil, "Brain Weevil", &reg);
    assert!(index_of(&log_lines(&state), "hand ran out").is_none(),
        "two cards for a two-card ask: {:?}", log_lines(&state));
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

/// "Target player discards two cards" is two choices, not one. The engine
/// asks for the first, and asks again for the second once the answer to the
/// first has been applied — CR 701.8a, one card at a time, each chosen by the
/// discarding player.
///
/// The chain runs only when there is more to ask for and nothing else is
/// mid-flight, and every other test of a multi-card discard uses a hand small
/// enough that no choice arises at all.
#[test]
fn a_two_card_discard_asks_twice() {
    use mtg_engine::actions::{Action, ResolvedChoice};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hand: Vec<_> = ["Geistflame", "Dream Twist", "Grizzly Bears"].iter()
        .map(|n| spell_in_hand(&mut state, &reg, n, P1))
        .collect();
    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);

    mtg_engine::engine::discard_cards(&mut state, P1, 2, weevil, "Brain Weevil", &reg);
    assert!(state.awaiting_action.is_some(),
        "three cards and an ask for two is a choice, so the player is asked");

    let after_first = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(hand[0]),
    }, &reg);
    assert_eq!(after_first.get_object(hand[0]).unwrap().zone, Zone::Graveyard,
        "the first card they named went");
    assert!(after_first.awaiting_action.is_some(),
        "and they are asked again for the second: {:?}", after_first.awaiting_action);

    let after_second = mtg_engine::engine::submit_action(&after_first, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(hand[1]),
    }, &reg);
    assert_eq!(after_second.get_object(hand[1]).unwrap().zone, Zone::Graveyard);
    assert_eq!(after_second.get_object(hand[2]).unwrap().zone, Zone::Hand,
        "and the third card stays: the ask was for two");
    assert!(after_second.awaiting_action.is_none(), "nothing further is asked");
}

// ---------------------------------------------------------------------------
// #326 — an ordering decision, a death and a counter are all readable back
// ---------------------------------------------------------------------------

/// A sweeper's four "Unruly Mob died" were indistinguishable from each other,
/// and from the twelve triggers they produced. The line names the object the
/// way the lines around it do: name, then id.
#[test]
fn a_death_line_carries_the_object_id() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let a = named_permanent(&mut state, &reg, "Unruly Mob", P0);
    let b = named_permanent(&mut state, &reg, "Unruly Mob", P0);
    state.get_object_mut(a).unwrap().damage_marked = 5;
    state.get_object_mut(b).unwrap().damage_marked = 5;
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    let lines = log_lines(&state);
    assert_line(&lines, &format!("Unruly Mob (#{}) died", a.0));
    assert_line(&lines, &format!("Unruly Mob (#{}) died", b.0));
    assert!(!lines.iter().any(|l| l == "Unruly Mob died"),
        "no death line without an id; log was {lines:#?}");
}

/// Two Unruly Mobs watch a third creature die: two distinguishable triggers,
/// so their controller orders them (CR 603.3b). The line recording the
/// choice, and the line recording the push, both name the source by id — the
/// same id the prompt's option carried — so which one went on next is on the
/// record. Then each resolution says what it did to the board.
#[test]
fn a_trigger_ordering_decision_and_its_resolution_name_the_source_by_id() {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mob_a = named_permanent(&mut state, &reg, "Unruly Mob", P0);
    let mob_b = named_permanent(&mut state, &reg, "Unruly Mob", P0);
    let victim = ready_creature(&mut state, P0, 1, 1);
    kill_by_damage(&mut state, &reg, victim);
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseTriggerOrder { options, .. }, ..
    }) = state.awaiting_action.clone() else {
        panic!("two distinguishable triggers are ordered by their controller: {:?}", state.awaiting_action);
    };
    assert_eq!(options.len(), 2, "{options:?}");
    // Choose mob_b's trigger to go on the stack first, whichever option it is.
    let (index, label) = options.iter().enumerate()
        .find(|(_, o)| o.ends_with(&format!("#{}]", mob_b.0)))
        .map(|(i, o)| (i, o.clone()))
        .expect("the prompt names mob_b by id");
    let mut state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(index, label),
    }, &reg);

    let lines = log_lines(&state);
    let chosen = format!("p0: put Unruly Mob (#{})'s triggered ability (put a +1/+1 counter on Unruly Mob) on the stack", mob_b.0);
    assert_line(&lines, &chosen);
    assert!(!lines.iter().any(|l| l.starts_with("p0: put Unruly Mob's")),
        "the choice line names the source by id, never bare; log was {lines:#?}");
    let pushed_b = index_of(&lines, &format!("p0's Unruly Mob (#{})'s triggered ability", mob_b.0))
        .expect("mob_b's push line");
    let pushed_a = index_of(&lines, &format!("p0's Unruly Mob (#{})'s triggered ability", mob_a.0))
        .expect("mob_a's push line: the one trigger left needs no prompt");
    assert!(pushed_b < pushed_a, "mob_b's trigger went on first, as chosen; log was {lines:#?}");

    // Resolve both (CR 608.1: the one put on last resolves first) and read
    // the board changes back off the log.
    assert_eq!(state.stack.len(), 2);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    let lines = log_lines(&state);
    let got_a = index_of(&lines, &format!("Unruly Mob (#{}) gets a +1/+1 counter (now 1)", mob_a.0))
        .expect("mob_a's counter is logged");
    let got_b = index_of(&lines, &format!("Unruly Mob (#{}) gets a +1/+1 counter (now 1)", mob_b.0))
        .expect("mob_b's counter is logged");
    assert!(got_a < got_b, "LIFO: mob_a's trigger, put on last, resolved first; log was {lines:#?}");
    assert_eq!(counters_of(&state, mob_a, CounterType::PlusOnePlusOne), 1);
    assert_eq!(counters_of(&state, mob_b, CounterType::PlusOnePlusOne), 1);
}

/// The counter line carries the running total, so a second counter reads
/// "now 2" and a plural reads as one.
#[test]
fn counter_lines_carry_the_count_and_the_total() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.add_counters(bear, CounterType::PlusOnePlusOne, 1);
    state.add_counters(bear, CounterType::PlusOnePlusOne, 2);
    state.remove_counters(bear, CounterType::PlusOnePlusOne, 1);
    // "Remove three" from a permanent holding two removes two, and says two.
    state.remove_counters(bear, CounterType::PlusOnePlusOne, 3);

    let lines = log_lines(&state);
    let expected = [
        format!("Grizzly Bears (#{}) gets a +1/+1 counter (now 1)", bear.0),
        format!("Grizzly Bears (#{}) gets 2 +1/+1 counters (now 3)", bear.0),
        format!("Grizzly Bears (#{}) loses a +1/+1 counter (now 2)", bear.0),
        format!("Grizzly Bears (#{}) loses 2 +1/+1 counters (now 0)", bear.0),
    ];
    let counter_lines: Vec<&String> = lines.iter()
        .filter(|l| l.contains("+1/+1 counter"))
        .collect();
    assert_eq!(counter_lines, expected.iter().collect::<Vec<_>>());
}

/// A counter aimed at a permanent that has left the battlefield lands
/// nowhere (CR 121.1), and a line saying it landed would be a lie.
#[test]
fn a_counter_that_lands_nowhere_is_not_logged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(bear, Zone::Graveyard, &reg);
    let before = log_lines(&state).len();
    state.add_counters(bear, CounterType::PlusOnePlusOne, 1);
    state.remove_counters(bear, CounterType::PlusOnePlusOne, 1);
    assert_eq!(log_lines(&state).len(), before, "log was {:#?}", log_lines(&state));
}

/// Entering with counters is one event (CR 614.1c) with one line (#299) —
/// not an entry followed by a placement.
#[test]
fn entering_with_counters_is_one_line_not_two() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let horde = spell_in_hand(&mut state, &reg, "Unbreathing Horde", P0);
    state.move_object(horde, Zone::Battlefield, &reg);

    let lines = log_lines(&state);
    assert_line(&lines, "enters with 2 +1/+1 counters");
    assert!(!lines.iter().any(|l| l.contains("gets 2 +1/+1 counters")),
        "the placement is the entry; log was {lines:#?}");
}

/// A loyalty ability's cost is a loyalty change, and the log says which way
/// and to what (CR 606.3). The old -N path wrote the counters directly and
/// said nothing.
#[test]
fn a_loyalty_ability_logs_the_loyalty_change_with_the_total() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);

    let plus = mtg_engine::engine::submit_action(&state, &Action::ActivateLoyaltyAbility {
        object_id: liliana, ability_index: 0, targets: vec![],
    }, &reg);
    assert_line(&log_lines(&plus), &format!("Liliana of the Veil (#{}) gets a loyalty counter (now 4)", liliana.0));

    let minus = mtg_engine::engine::submit_action(&state, &Action::ActivateLoyaltyAbility {
        object_id: liliana, ability_index: 1, targets: vec![Target::Player(P1)],
    }, &reg);
    assert_line(&log_lines(&minus), &format!("Liliana of the Veil (#{}) loses 2 loyalty counters (now 1)", liliana.0));
}

// ---------------------------------------------------------------------------
// #467 — a destroy reports what happened, and names the cause before it
// ---------------------------------------------------------------------------

/// Cast Slayer of the Wicked and answer its ETB trigger by choosing `victim`.
///
/// Slayer is one of the two cards whose destruction runs through the shared
/// `PendingEffect::Destroy` handler, which is where the false line was
/// written; Reaper from the Abyss is the other.
fn slayer_destroys(state: &GameState, reg: &mtg_engine::cards::CardRegistry,
                   victim: mtg_engine::ids::ObjectId) -> GameState {
    let mut state = state.clone();
    let slayer = castable_spell(&mut state, reg, "Slayer of the Wicked", P0);
    let mut state = cast_and_resolve(&state, reg, slayer, vec![]);
    mtg_engine::triggers::process_triggers(&mut state, reg);
    assert!(state.awaiting_action.is_some(), "Slayer should ask what to destroy");
    mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(victim))),
        },
        reg,
    )
}

/// A regenerated permanent is still on the battlefield (CR 701.15a), and the
/// shared destroy handler announced it destroyed anyway — the log contradicted
/// the line above it and the board in front of the player.
#[test]
fn a_destroy_a_regeneration_shield_answered_is_not_reported_as_a_kill() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    state.get_object_mut(corpse).unwrap().regeneration_shields = 1;

    let after = slayer_destroys(&state, &reg, corpse);

    assert_eq!(after.get_object(corpse).unwrap().zone, Zone::Battlefield,
        "regeneration replaces the destruction");
    let lines = log_lines(&after);
    assert_line(&lines, "Slayer of the Wicked could not destroy Walking Corpse");
    assert_line(&lines, "it regenerated");
    assert!(!lines.iter().any(|l| l.contains("Slayer of the Wicked destroyed")),
        "nothing was destroyed, so no line may say it was; log was {lines:#?}");
}

/// The same for indestructible (CR 701.7b), which is the worse of the two:
/// nothing else in the log contradicts the false line, so it is the reader's
/// only account of the event.
#[test]
fn a_destroy_an_indestructible_creature_shrugged_off_is_not_reported_as_a_kill() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    grant_keyword(&mut state, corpse, Keyword::Indestructible);

    let after = slayer_destroys(&state, &reg, corpse);

    assert_eq!(after.get_object(corpse).unwrap().zone, Zone::Battlefield,
        "indestructible prevents the destruction");
    let lines = log_lines(&after);
    assert_line(&lines, "Slayer of the Wicked could not destroy Walking Corpse");
    assert_line(&lines, "it is indestructible");
    assert!(!lines.iter().any(|l| l.contains("Slayer of the Wicked destroyed")),
        "nothing was destroyed, so no line may say it was; log was {lines:#?}");
}

/// And when the creature really does die, the line that names the cause comes
/// before the line that records the consequence — the log read
/// "Walking Corpse died" and only then "Slayer of the Wicked destroyed Walking
/// Corpse". `sacrifice_by` was fixed for the same reason (#263).
#[test]
fn a_destroy_names_its_cause_before_the_death_it_caused() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P1);

    let after = slayer_destroys(&state, &reg, corpse);

    assert_eq!(after.get_object(corpse).unwrap().zone, Zone::Graveyard);
    let lines = log_lines(&after);
    let cause = index_of(&lines, "Slayer of the Wicked destroyed Walking Corpse")
        .unwrap_or_else(|| panic!("expected the destroy line; log was {lines:#?}"));
    let died = index_of(&lines, &format!("Walking Corpse (#{}) died", corpse.0))
        .unwrap_or_else(|| panic!("expected the death line; log was {lines:#?}"));
    assert!(cause < died,
        "the cause is announced before the consequence; log was {lines:#?}");
}

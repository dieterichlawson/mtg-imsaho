//! The auto-tapper keeps payable more than the spells in hand.
//!
//! A plan for one cost should not leave something else the player could pay
//! for unpayable when another plan for the same cost would not. That promise
//! used to be computed from the hand alone, so three kinds of "something
//! else" were spent freely:
//!
//! - an activated ability of a permanent, including one that taps its own
//!   source — a utility land's `{T}: Add {C}` was the first thing spent on
//!   generic, and its own ability went with it (issue #837);
//! - a card castable from the graveyard by flashback (issue #838);
//! - and everything at all while something resolves: Mentor of the Meek's
//!   `{1}` and Screeching Bat's upkeep planned against nothing (issue #840).

mod common;
use common::*;

use mtg_engine::actions::{Action, ResolvedChoice};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

fn names(state: &GameState, plan: &[(ObjectId, usize)]) -> Vec<String> {
    plan.iter().map(|(id, _)| state.get_object(*id).unwrap().name.clone()).collect()
}

fn lands(state: &mut GameState, reg: &CardRegistry, names: &[&str]) -> Vec<ObjectId> {
    names.iter().map(|n| named_permanent(state, reg, n, P0)).collect()
}

/// The offered `CastSpell` for `spell` (its first one, with the plan the
/// menu shows).
fn offered_cast(state: &GameState, reg: &CardRegistry, spell: ObjectId) -> Action {
    engine::legal_actions(state, reg).actions.into_iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, alternative_cost: None, .. } if *object_id == spell))
        .unwrap_or_else(|| panic!("no cast of {spell:?} offered"))
}

fn plan_of(action: &Action) -> Vec<(ObjectId, usize)> {
    match action {
        Action::CastSpell { tap_plan, .. } => tap_plan.clone(),
        other => panic!("not a cast: {other:?}"),
    }
}

/// Issue #837: Nephalia Drownyard, Island, two Swamps, two Forests. Ambush
/// Viper's `{1}{G}` used to tap Forest and the Drownyard — and with it the
/// Drownyard's `{1}{U}{B}, {T}` mill — while Forest and a second Swamp pay
/// the same and keep the mill.
#[test]
fn a_cast_does_not_tap_a_utility_land_whose_own_ability_another_plan_keeps() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ids = lands(&mut state, &reg,
        &["Nephalia Drownyard", "Island", "Swamp", "Swamp", "Forest", "Forest"]);
    let drownyard = ids[0];
    let viper = spell_in_hand(&mut state, &reg, "Ambush Viper", P0);
    assert!(offers_ability_of(&state, &reg, drownyard), "the mill is payable before the cast");

    let cast = offered_cast(&state, &reg, viper);
    let plan = plan_of(&cast);
    assert!(!plan.iter().any(|(id, _)| *id == drownyard),
        "Ambush Viper's plan spends the Drownyard's own ability: {:?}", names(&state, &plan));

    let state = engine::submit_action(&state, &cast, &reg);
    assert!(offers_ability_of(&state, &reg, drownyard),
        "after the cast the Drownyard's mill is still payable");
}

/// Issue #837, the other board: Gavony Township's `{2}{G}{W}, {T}` over a
/// Brimstone Volley `{2}{R}`: eight lands, and plans that pay the Volley
/// from the basics leave the Township its four.
/// (The issue's board had one Island, and there no plan keeps both: the
/// six other lands cannot make seven mana.)
#[test]
fn gavony_township_is_not_spent_on_a_generic_pip_it_needs_kept() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ids = lands(&mut state, &reg,
        &["Gavony Township", "Plains", "Forest", "Forest", "Mountain", "Mountain", "Island", "Island"]);
    let township = ids[0];
    named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let volley = spell_in_hand(&mut state, &reg, "Brimstone Volley", P0);

    let cast = offered_cast(&state, &reg, volley);
    let plan = plan_of(&cast);
    let tapped = names(&state, &plan);
    assert!(!plan.iter().any(|(id, _)| *id == township),
        "Brimstone Volley's plan spends Gavony Township: {tapped:?}");
    let state = engine::submit_action(&state, &cast, &reg);
    assert!(offers_ability_of(&state, &reg, township),
        "after the cast Gavony Township's ability is still payable");
}

/// Issue #838: nine lands, Think Twice `{1}{U}` in hand, Moan of the
/// Unhallowed in the graveyard with flashback `{5}{B}{B}`. Island and a
/// Swamp left one black source; Island and a Plains keep the flashback.
#[test]
fn a_cast_keeps_a_flashback_in_the_graveyard_payable() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    lands(&mut state, &reg, &["Swamp", "Swamp", "Island", "Island",
        "Plains", "Plains", "Plains", "Plains", "Mountain"]);
    let think = spell_in_hand(&mut state, &reg, "Think Twice", P0);
    let moan = named_card_in_graveyard(&mut state, &reg, "Moan of the Unhallowed", P0);
    stock_library(&mut state, &reg, P0, 5);
    let flashback_offered = |s: &GameState| engine::legal_actions(s, &reg).actions.iter()
        .any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == moan));
    assert!(flashback_offered(&state), "Moan's flashback is payable before Think Twice");

    let cast = offered_cast(&state, &reg, think);
    let plan = plan_of(&cast);
    assert!(!names(&state, &plan).iter().any(|n| n == "Swamp"),
        "Think Twice's plan spends a Swamp Moan's {{B}}{{B}} needs: {:?}", names(&state, &plan));

    let mut state = engine::submit_action(&state, &cast, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(flashback_offered(&state), "after Think Twice, Moan's flashback is still payable");
}

/// Issue #840: a payment asked for while something resolves keeps the hand
/// castable. Mountain, Island, Plains, Geistflame `{R}` in hand: Mentor of
/// the Meek's `{1}` is paid by anything but the Mountain — whichever order
/// the lands sit in, which is what decided it before.
#[test]
fn a_resolution_payment_keeps_the_hand_castable() {
    for order in [["Mountain", "Island", "Plains"], ["Island", "Plains", "Mountain"]] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let ids = lands(&mut state, &reg, &order);
        let mountain = ids[order.iter().position(|n| *n == "Mountain").unwrap()];
        spell_in_hand(&mut state, &reg, "Geistflame", P0);

        let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
        let plan = engine::payment_plan_with_sources(&state, P0, &cost, &reg)
            .expect("{1} is payable");
        assert!(!plan.iter().any(|(id, _)| *id == mountain),
            "board {order:?}: paying {{1}} taps the Mountain Geistflame needs: {:?}",
            names(&state, &plan));
    }
}

/// Issue #840, Screeching Bat's upkeep `{2}{B}{B}` (its own planner):
/// two Swamps, Mountain, Forest, Island and a Geistflame in hand. The
/// payment used to tap the Mountain; Forest and Island pay the generic.
#[test]
fn screeching_bats_upkeep_payment_keeps_the_hand_castable() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);
    let ids = lands(&mut state, &reg, &["Swamp", "Swamp", "Mountain", "Forest", "Island"]);
    let mountain = ids[2];
    spell_in_hand(&mut state, &reg, "Geistflame", P0);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);
    assert!(state.awaiting_action.is_some(), "the Bat asks whether to pay");
    let state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );
    assert!(state.get_object(bat).unwrap().is_transformed, "the Bat paid and transformed");
    assert!(!state.get_object(mountain).unwrap().tapped,
        "the upkeep payment tapped the Mountain Geistflame needs");
}

/// X is funded around the same demands (the propagation review of #837 and
/// #840). Devil's Play `{X}{R}` with two Mountains and a Swamp and a
/// Geistflame `{R}` in hand: the cast's plan spends one Mountain on `{R}`,
/// and X = 1 used to take the other — "Mountain" sorts before "Swamp" and
/// the allocator walks the groups in order — stranding the Geistflame.
#[test]
fn x_is_funded_from_what_nothing_else_needs() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ids = lands(&mut state, &reg, &["Mountain", "Mountain", "Swamp"]);
    spell_in_hand(&mut state, &reg, "Geistflame", P0);
    let play = spell_in_hand(&mut state, &reg, "Devil's Play", P0);

    let cast = engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, targets, .. }
            if *object_id == play && targets.iter().any(|t| matches!(t, mtg_engine::actions::Target::Player(p) if *p == P1))))
        .expect("Devil's Play at the opponent is offered");
    let state = engine::submit_action(&state, &cast, &reg);
    let legal = engine::legal_actions(&state, &reg);
    let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, .. }) =
        legal.resolution_prompt.as_ref() else {
        panic!("Devil's Play asks for X funding");
    };
    let (response, shortfall) = mtg_engine::funding::allocate_for_x(options, 1);
    assert_eq!(shortfall, 0);
    assert_eq!(response.taps.keys().cloned().collect::<Vec<_>>(), vec!["Swamp".to_string()],
        "X = 1 taps the Mountain Geistflame needs: {response:?}");
    let state = engine::submit_action(&state,
        &Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) }, &reg);
    assert!(ids.iter().any(|id| state.get_object(*id).unwrap().name == "Mountain"
        && !state.get_object(*id).unwrap().tapped), "a Mountain is left for Geistflame");
}

/// Issue #840, the pool half (from its propagation review): a payment the
/// pool covers taps nothing, and the pool was then spent in the fixed
/// "C, W, U, B, R, G" order — floating {W}{G} paid Mentor's {1} with the
/// {W} the Doomed Traveler in hand needs. It is paid around what else could
/// be paid for, as a cast is (#678).
#[test]
fn a_resolution_payment_from_the_pool_keeps_the_hand_castable() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    spell_in_hand(&mut state, &reg, "Doomed Traveler", P0);
    add_mana(&mut state, P0, &[(ManaType::White, 1), (ManaType::Green, 1)]);
    stock_library(&mut state, &reg, P0, 2);

    let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
    assert!(engine::pay_cost_with_sources(&mut state, P0, &cost, &reg));
    let pool = &state.get_player(P0).mana_pool;
    assert_eq!((pool.get(ManaType::White), pool.get(ManaType::Green)), (1, 0),
        "paying {{1}} from {{W}}{{G}} spent the {{W}} Doomed Traveler needs");
}

/// X out of floating mana spends what nothing else needs first (from the
/// second propagation review). Devil's Play `{X}{R}` with {R}{W}{W}{G}
/// floating and two Doomed Travelers `{W}` in hand: X = 1 spent the
/// largest bucket, a {W}, and left one Traveler unpayable; the {G} pays it.
#[test]
fn x_from_the_pool_spends_what_nothing_else_needs() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    spell_in_hand(&mut state, &reg, "Doomed Traveler", P0);
    spell_in_hand(&mut state, &reg, "Doomed Traveler", P0);
    let play = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 1), (ManaType::White, 2), (ManaType::Green, 1)]);

    let cast = engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, targets, .. }
            if *object_id == play && targets.iter().any(|t| matches!(t, mtg_engine::actions::Target::Player(p) if *p == P1))))
        .expect("Devil's Play at the opponent is offered");
    let state = engine::submit_action(&state, &cast, &reg);
    let legal = engine::legal_actions(&state, &reg);
    let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, .. }) =
        legal.resolution_prompt.as_ref() else {
        panic!("Devil's Play asks for X funding");
    };
    let (response, _) = mtg_engine::funding::allocate_for_x(options, 1);
    let state = engine::submit_action(&state,
        &Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) }, &reg);
    let pool = &state.get_player(P0).mana_pool;
    assert_eq!((pool.get(ManaType::White), pool.get(ManaType::Green)), (2, 0),
        "X = 1 was paid with a {{W}} a Doomed Traveler needs");
}

/// Issue #843: Island x2, Swamp x2, Mountain, Nephalia Drownyard, Stensia
/// Bloodhall; Devil's Play in hand, Think Twice in the graveyard. X = 3
/// tapped Island, Island, Swamp: basics went before the colourless lands,
/// and Bloodhall's `{3}{B}{R}` — unpayable once the Mountain pays Devil's
/// Play's `{R}` — still voted for keeping the Swamps and itself. The
/// flashback `{2}{U}` went with the Islands.
#[test]
fn x_funding_keeps_a_flashback_payable_over_demands_nobody_can_pay() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    lands(&mut state, &reg, &["Island", "Island", "Swamp", "Swamp", "Mountain",
        "Nephalia Drownyard", "Stensia Bloodhall"]);
    let think = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);
    let play = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    stock_library(&mut state, &reg, P0, 5);
    let flashback_offered = |s: &GameState| engine::legal_actions(s, &reg).actions.iter()
        .any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == think));

    let cast = engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, targets, .. }
            if *object_id == play && targets.iter().any(|t| matches!(t, mtg_engine::actions::Target::Player(p) if *p == P1))))
        .expect("Devil's Play at the opponent is offered");
    let state = engine::submit_action(&state, &cast, &reg);
    let legal = engine::legal_actions(&state, &reg);
    let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, .. }) =
        legal.resolution_prompt.as_ref() else {
        panic!("Devil's Play asks for X funding");
    };
    let (response, shortfall) = mtg_engine::funding::allocate_for_x(options, 3);
    assert_eq!(shortfall, 0);
    let state = engine::submit_action(&state,
        &Action::ResolveChoice { choice: ResolvedChoice::XFunding(response.clone()) }, &reg);
    assert!(flashback_offered(&state),
        "X = 3 stranded Think Twice's flashback: {:?}", response.taps);
}

/// #843's review: floating mana is spent on X around the demands the board
/// can pay, as the taps are. Devil's Play `{X}{R}` with {R}{U}{B}{B}
/// floating, Think Twice `{1}{U}` and Army of the Damned `{5}{B}{B}{B}` in
/// hand: the Army cannot be paid, and its three {B} pips must not make the
/// {U} Think Twice needs look like the spare mana.
#[test]
fn x_from_the_pool_ignores_demands_nobody_can_pay() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    spell_in_hand(&mut state, &reg, "Think Twice", P0);
    spell_in_hand(&mut state, &reg, "Army of the Damned", P0);
    let play = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 1), (ManaType::Blue, 1), (ManaType::Black, 2)]);

    let cast = engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, targets, .. }
            if *object_id == play && targets.iter().any(|t| matches!(t, mtg_engine::actions::Target::Player(p) if *p == P1))))
        .expect("Devil's Play at the opponent is offered");
    let state = engine::submit_action(&state, &cast, &reg);
    let legal = engine::legal_actions(&state, &reg);
    let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, .. }) =
        legal.resolution_prompt.as_ref() else {
        panic!("Devil's Play asks for X funding");
    };
    let (response, _) = mtg_engine::funding::allocate_for_x(options, 1);
    let state = engine::submit_action(&state,
        &Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) }, &reg);
    let pool = &state.get_player(P0).mana_pool;
    assert_eq!((pool.get(ManaType::Blue), pool.get(ManaType::Black)), (1, 1),
        "X = 1 was paid with the {{U}} Think Twice needs");
}

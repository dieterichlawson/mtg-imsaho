//! Which board states are worth stopping a player for.
//!
//! CR 117.3d: a player with nothing to do passes. The engine decides that for
//! a seat by asking whether it has a meaningful action — and the answer has to
//! be the same one `legal_actions` would give, or the seat is stopped at a
//! menu that cannot do the thing it was stopped for.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::state::GameState;
use mtg_engine::types::*;

/// Play out `turns` worth of priority with both seats passing, counting how
/// many times each seat was actually asked. Any non-pass prompt (a combat
/// declaration, a mandatory choice) is answered in the least eventful way.
///
/// The count is only worth something if the game lasts: a `game_at_step`
/// board has empty libraries, so the first draw step ends it, and a "stops
/// per turn" claim measured on it has seen one priority window. Callers
/// stock the libraries with [`nothing_to_do`] first.
fn count_prompts(state: &mut GameState, registry: &mtg_engine::cards::CardRegistry, stop_turn: u32) -> usize {
    let mut prompts = 0;
    mtg_engine::engine::run_game_loop(state, registry, |gs, _player, legal| {
        if gs.turn_number >= stop_turn {
            return Action::Concede;
        }
        if legal.combat_prompt.is_some() {
            prompts += 1;
            return Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] };
        }
        if legal.actions.iter().any(|a| matches!(a, Action::PassPriority)) {
            prompts += 1;
            return Action::PassPriority;
        }
        prompts += 1;
        legal.actions[0].clone()
    });
    prompts
}

/// An Equipment on the battlefield with no creature to equip is not a reason
/// to stop the player: equip is sorcery-speed-only and has no legal target, so
/// `legal_actions` does not offer it at any window. The gate used to say
/// otherwise and stopped both seats at every priority window for the rest of
/// the game (issue #269).
#[test]
fn an_unequippable_equipment_is_not_a_meaningful_action() {
    let reg = registry();

    let mut without = game_at_step(Step::Upkeep, P0);
    for _ in 0..2 { named_permanent(&mut without, &reg, "Plains", P0); }
    nothing_to_do(&mut without, &reg);
    let baseline = count_prompts(&mut without, &reg, 4);

    let mut with = game_at_step(Step::Upkeep, P0);
    for _ in 0..2 { named_permanent(&mut with, &reg, "Plains", P0); }
    named_permanent(&mut with, &reg, "Blazing Torch", P0);
    nothing_to_do(&mut with, &reg);
    let with_equipment = count_prompts(&mut with, &reg, 4);

    assert_eq!(with_equipment, baseline,
        "an Equipment with nothing to equip must not add a single stop");
}

/// The same Equipment with a creature to equip *is* worth stopping for, in a
/// main phase — the fix must not go the other way and hide a legal equip.
#[test]
fn an_equippable_equipment_is_offered_in_a_main_phase() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..2 { named_permanent(&mut state, &reg, "Plains", P0); }
    named_permanent(&mut state, &reg, "Blazing Torch", P0);
    named_permanent(&mut state, &reg, "Doomed Traveler", P0);

    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(legal.actions.iter().any(|a| matches!(a, Action::ActivateAbility { .. })),
        "equip is legal here and must be on offer: {:?}", legal.actions);
}

/// CR 605.1a: a mana ability is still an activation the player chooses to
/// make, and Deranged Assistant's "{T}, Mill a card: Add {C}" mills whether or
/// not the mana is ever spent. A seat with one and nothing else to do was
/// never given priority for it (issue #266).
#[test]
fn a_mana_ability_with_a_side_effect_is_a_meaningful_action() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let assistant = named_permanent(&mut state, &reg, "Deranged Assistant", P0);
    state.get_object_mut(assistant).unwrap().summoning_sick = false;
    // The ability mills, so it needs a library to mill from.
    for _ in 0..3 {
        let card = spell_in_hand(&mut state, &reg, "Island", P0);
        state.get_object_mut(card).unwrap().zone = Zone::Library;
        state.get_player_mut(P0).library_order.push(card);
    }

    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let offered = legal.actions.iter().any(|a|
        matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == assistant));
    assert!(offered, "test precondition: the ability is legal");

    let mut asked = false;
    let mut probe = state.clone();
    mtg_engine::engine::run_game_loop(&mut probe, &reg, |gs, player, _legal| {
        if player == P0 && gs.step == Step::PrecombatMain && gs.turn_number == 1 {
            asked = true;
        }
        Action::Concede
    });
    assert!(asked, "the seat must be given priority to use it");
}

/// A plain tap-for-mana still is not: a player holding nothing castable is not
/// stopped just because they control a Plains — not once, at any window of
/// any turn. This used to allow "at most 4 stops" on a board whose libraries
/// were empty, so the game ended at its first draw step having opened one
/// window, and a gate that stopped the seat at every window it ever opened
/// scored 1.
#[test]
fn a_bare_mana_ability_is_still_not_a_meaningful_action() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    for _ in 0..2 { named_permanent(&mut state, &reg, "Plains", P0); }
    nothing_to_do(&mut state, &reg);

    let stops = count_prompts(&mut state, &reg, 3);
    assert_eq!(stops, 0,
        "a board with two lands and nothing to spend them on passes through every window");
    // Never asked, so never told to concede: the game ran on to its own end.
    assert!(state.turn_number > 3, "the count covered a whole game, not turn {}", state.turn_number);
}

/// Give both seats draws that change nothing: a green creature, which the
/// seat with two Plains cannot cast and the seat with no lands cannot either,
/// so the draw step neither ends the game nor adds a legal play.
fn nothing_to_do(state: &mut GameState, registry: &mtg_engine::cards::CardRegistry) {
    for p in [P0, P1] {
        stock_library_with(state, registry, p, "Grizzly Bears", 8);
    }
}

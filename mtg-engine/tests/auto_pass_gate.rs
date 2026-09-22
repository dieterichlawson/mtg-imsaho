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
    let baseline = count_prompts(&mut without, &reg, 4);

    let mut with = game_at_step(Step::Upkeep, P0);
    for _ in 0..2 { named_permanent(&mut with, &reg, "Plains", P0); }
    named_permanent(&mut with, &reg, "Blazing Torch", P0);
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
/// stopped just because they control a Plains.
#[test]
fn a_bare_mana_ability_is_still_not_a_meaningful_action() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    for _ in 0..2 { named_permanent(&mut state, &reg, "Plains", P0); }

    let stops = count_prompts(&mut state, &reg, 3);
    assert!(stops <= 4,
        "an empty board with two lands should pass through its steps, got {stops} stops");
}

/// The safety valve counts passes, not spinning.
///
/// `run_game_loop` breaks an infinite auto-pass loop by advancing or resolving
/// the game itself once it has auto-passed `MAX_AUTO_PASSES` times in a row.
/// The counter was reset only when a seat had a meaningful action, so a game
/// that is making perfectly good progress *through* auto-passes trips it: a
/// stack neither seat can respond to drains at two auto-passes per resolution,
/// and after fifty resolutions the valve fires.
///
/// What it does when it fires is the defect. It resolves the top of the stack
/// with nobody having passed priority (CR 117.4) and without going through the
/// decision callback, so an observer never sees that resolution at all — the
/// transition it does see spans two, and reads the second one's bookkeeping
/// against the first one's stack entry. That is issue #571 verbatim: "ability
/// resolved with sacrifice=Some(ObjectId(93)) but the state says
/// sacrifice=Some(ObjectId(95))", bg-coverage vs ub-coverage, seed
/// 20718200198, where a random seat had stacked thirty Grimgrin activations
/// and then run out of creatures to respond with.
#[test]
fn a_stack_drained_by_auto_passes_resolves_one_object_per_pass_round() {
    use mtg_engine::state::StackEntry;

    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    // Grimgrin with no other creature to sacrifice: its ability is on the
    // card and not on the menu, so neither seat has a meaningful action and
    // every pass from here is an auto-pass.
    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let card_id = state.get_object(grimgrin).unwrap().card_id;

    // Deep enough to pass MAX_AUTO_PASSES at two auto-passes per resolution,
    // with room to spare.
    const ENTRIES: usize = 80;
    for _ in 0..ENTRIES {
        state.stack.push(StackEntry::Ability {
            source_id: grimgrin,
            ability_index: 0,
            behavior_card_id: card_id,
            targets: vec![],
            activator: P0,
            x_value: None,
            target_requirement: None,
            sacrificed: None,
            sacrificed_toughness: None,
            loyalty: false,
        });
    }
    // The fuzz runner's mode: every pass is a decision the observer sees.
    state.observe_every_submit = true;

    let mut depths: Vec<usize> = Vec::new();
    let mut unobserved: Option<(usize, usize)> = None;
    mtg_engine::engine::run_game_loop(&mut state, &reg, |gs, _player, legal| {
        if let Some(&before) = depths.last() {
            if before > gs.stack.len() + 1 {
                unobserved.get_or_insert((before, gs.stack.len()));
            }
        }
        depths.push(gs.stack.len());
        if gs.stack.is_empty() {
            return Action::Concede;
        }
        if legal.actions.iter().any(|a| matches!(a, Action::PassPriority)) {
            return Action::PassPriority;
        }
        legal.actions[0].clone()
    });

    assert_eq!(unobserved, None,
        "the stack went from {:?} to {:?} between two decisions: something \
         resolved without a pass round and without reaching the callback \
         (CR 117.4)",
        unobserved.map(|(a, _)| a), unobserved.map(|(_, b)| b));
    assert!(depths.contains(&0),
        "the stack never drained; deepest-to-last seen: {:?}",
        depths.last());
    assert!(depths.len() >= 2 * ENTRIES,
        "{} entries drained in {} decisions — fewer than the two passes each \
         resolution owes (CR 117.4)", ENTRIES, depths.len());
}

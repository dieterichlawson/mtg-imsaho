//! Activated abilities on creatures: what activating one does, and the
//! restrictions on activating it again (CR 602.2a).
//!
//! Cards covered (6), so this is greppable by name as well as by rule:
//!
//! - Avacynian Priest
//! - Darkthicket Wolf
//! - Feral Ridgewolf
//! - Kessig Wolf
//! - Lantern Spirit
//! - Manor Skeleton

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

// ══════════════════════════════════════════════════════════════════
// What one activation does
// ══════════════════════════════════════════════════════════════════

/// A pump ability changes the creature's characteristics until end of turn.
/// Each row states the cost, the printed size, and what the creature looks
/// like afterwards — a keyword grant is the same shape as a P/T grant.
#[test]
fn a_pump_ability_changes_the_creature_it_is_activated_on() {
    // (card, mana to pay, printed p/t, p/t after one activation, keyword gained)
    const CARDS: &[(&str, &[(ManaType, u32)], (i32, i32), (i32, i32), Option<Keyword>)] = &[
        ("Kessig Wolf", &[(ManaType::Colorless, 1), (ManaType::Red, 1)],
         (3, 1), (3, 1), Some(Keyword::FirstStrike)),
        ("Feral Ridgewolf", &[(ManaType::Colorless, 1), (ManaType::Red, 1)],
         (1, 2), (3, 2), None),
        ("Darkthicket Wolf", &[(ManaType::Colorless, 2), (ManaType::Green, 1)],
         (2, 2), (4, 4), None),
    ];

    for &(name, mana, printed, after, keyword) in CARDS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let creature = named_permanent(&mut state, &reg, name, P0);
        assert_eq!(
            (state.effective_power(creature, &reg), state.effective_toughness(creature, &reg)),
            (Some(printed.0), Some(printed.1)), "{name} starts at its printed size");
        if let Some(kw) = keyword {
            assert!(!state.has_keyword(creature, kw, &reg),
                "{name} does not have {kw:?} before the ability is activated");
        }

        add_mana(&mut state, P0, mana);
        state = activate_only_offered_ability(&state, &reg);

        assert_eq!(
            (state.effective_power(creature, &reg), state.effective_toughness(creature, &reg)),
            (Some(after.0), Some(after.1)), "{name} after one activation");
        if let Some(kw) = keyword {
            assert!(state.has_keyword(creature, kw, &reg), "{name} gained {kw:?}");
        }
    }
}

/// Lantern Spirit's "{U}: Return Lantern Spirit to its owner's hand" — the
/// ability moves the permanent that has it, so it is gone by the time the
/// activation finishes.
#[test]
fn lantern_spirit_returns_itself_to_hand() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let spirit = named_permanent(&mut state, &reg, "Lantern Spirit", P0);
    add_mana(&mut state, P0, &[(ManaType::Blue, 1)]);
    state = activate_only_offered_ability(&state, &reg);

    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Hand);
}

/// Manor Skeleton's "{1}{B}: Regenerate Manor Skeleton" — the shield is placed
/// on activation and spent by the next lethal state-based check (CR 701.15),
/// clearing the damage rather than the creature.
#[test]
fn manor_skeleton_regenerates_out_of_lethal_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let skeleton = named_permanent(&mut state, &reg, "Manor Skeleton", P0);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 1), (ManaType::Black, 1)]);
    state = activate_only_offered_ability(&state, &reg);
    assert_eq!(state.get_object(skeleton).unwrap().regeneration_shields, 1,
        "activating places a regeneration shield");

    state.get_object_mut(skeleton).unwrap().damage_marked = 1; // lethal for a 1/1
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(skeleton).unwrap().zone, Zone::Battlefield,
        "the shield is spent instead of the creature dying");
    assert_eq!(state.get_object(skeleton).unwrap().damage_marked, 0,
        "and regenerating removes all damage from it");
}

/// "**until end of turn**". The grant is a temporary effect the engine's
/// cleanup step removes; a card that made it permanent instead would pass
/// every test above, because they all look at the same turn it was activated.
///
/// The turns are advanced for real rather than by clearing `until_end_of_turn`
/// by hand — a test that cleared it itself would pass with the engine's
/// cleanup deleted.
#[test]
fn a_pump_ability_wears_off_at_end_of_turn() {
    // (card, mana to pay, printed p/t, p/t while pumped, keyword gained)
    const CARDS: &[(&str, &[(ManaType, u32)], (i32, i32), (i32, i32), Option<Keyword>)] = &[
        ("Kessig Wolf", &[(ManaType::Colorless, 1), (ManaType::Red, 1)],
         (3, 1), (3, 1), Some(Keyword::FirstStrike)),
        ("Feral Ridgewolf", &[(ManaType::Colorless, 1), (ManaType::Red, 1)],
         (1, 2), (3, 2), None),
        ("Darkthicket Wolf", &[(ManaType::Colorless, 2), (ManaType::Green, 1)],
         (2, 2), (4, 4), None),
    ];

    for &(name, mana, printed, pumped, keyword) in CARDS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        // Real turns mean real draw steps; without libraries both players deck
        // out before the turn change lands.
        stock_library(&mut state, &reg, P0, 10);
        stock_library(&mut state, &reg, P1, 10);

        let creature = named_permanent(&mut state, &reg, name, P0);
        add_mana(&mut state, P0, mana);
        let mut state = activate_only_offered_ability(&state, &reg);

        assert_eq!(
            (state.effective_power(creature, &reg), state.effective_toughness(creature, &reg)),
            (Some(pumped.0), Some(pumped.1)), "{name}: test precondition, the ability resolved");
        if let Some(kw) = keyword {
            assert!(state.has_keyword(creature, kw, &reg), "{name}: and granted {kw:?}");
        }

        advance_to_next_turn(&mut state, &reg);

        assert_eq!(
            (state.effective_power(creature, &reg), state.effective_toughness(creature, &reg)),
            (Some(printed.0), Some(printed.1)), "{name} is back to its printed size");
        if let Some(kw) = keyword {
            assert!(!state.has_keyword(creature, kw, &reg),
                "{name} no longer has {kw:?} — the grant lasted until end of turn and no longer");
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// When it can be activated again (CR 602.2a)
// ══════════════════════════════════════════════════════════════════

/// Feral Ridgewolf's ability has no restriction, so paying twice pumps twice.
/// The contrast with Darkthicket Wolf below is the point: same shape of
/// ability, and only one of them carries "Activate only once each turn".
#[test]
fn an_unrestricted_pump_ability_stacks_with_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf = named_permanent(&mut state, &reg, "Feral Ridgewolf", P0);
    for expected in [3, 5] {
        add_mana(&mut state, P0, &[(ManaType::Colorless, 1), (ManaType::Red, 1)]);
        state = activate_only_offered_ability(&state, &reg);
        assert_eq!(state.effective_power(wolf, &reg), Some(expected));
    }
}

/// "Activate only once each turn" is a restriction on *this* turn: blocked for
/// the rest of it, offered again on the next one.
///
/// The second half is taken by advancing real turns rather than by clearing
/// `abilities_activated_this_turn` — the engine clears it at the turn change,
/// and a test that cleared it itself passed even with the engine's clearing
/// removed.
#[test]
fn a_once_per_turn_ability_is_blocked_this_turn_and_offered_the_next() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf = named_permanent(&mut state, &reg, "Darkthicket Wolf", P0);
    // Real turns mean real draw steps; without libraries both players deck out
    // and the game ends before the second activation is reached.
    stock_library(&mut state, &reg, P0, 10);
    stock_library(&mut state, &reg, P1, 10);

    let pay = |state: &mut mtg_engine::state::GameState| {
        add_mana(state, P0, &[(ManaType::Colorless, 2), (ManaType::Green, 1)]);
    };

    pay(&mut state);
    assert!(offers_ability_of(&state, &reg, wolf), "offered the first time");
    state = activate_only_offered_ability(&state, &reg);

    pay(&mut state);
    assert!(!offers_ability_of(&state, &reg, wolf),
        "and not again this turn, however much mana is available");

    // Two turn changes puts P0 back in their own precombat main with priority —
    // the same position the first activation happened from.
    advance_to_next_turn(&mut state, &reg);
    advance_to_next_turn(&mut state, &reg);
    advance_to_step(&mut state, &reg, Step::PrecombatMain);
    assert_eq!(state.active_player, P0, "test setup: back on P0's turn");

    pay(&mut state);
    assert!(offers_ability_of(&state, &reg, wolf),
        "the restriction is per turn, so a new turn offers it again");
}

/// The restriction Darkthicket Wolf prints is "only once each turn" and not
/// "only as a sorcery", so the pump is available in combat — after blockers
/// are declared, which is the whole point of holding the mana.
#[test]
fn a_pump_ability_with_no_speed_restriction_is_offered_during_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P1);

    let wolf = named_permanent(&mut state, &reg, "Darkthicket Wolf", P0);
    state.priority_player = Some(P0);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 2), (ManaType::Green, 1)]);

    assert!(offers_ability_of(&state, &reg, wolf),
        "the ability says only \"once each turn\", not \"only as a sorcery\" — \
         and this is not even P0's turn");
    let state = activate_only_offered_ability(&state, &reg);
    assert_eq!(state.effective_power(wolf, &reg), Some(4));
}

/// "{1}, {T}: Tap target non-Human creature." The tap symbol in the cost is its
/// own restriction — a tapped permanent cannot pay it again (CR 602.2a) — and
/// the target restriction is checked when the ability is offered (CR 601.2c).
#[test]
fn avacynian_priest_taps_a_non_human_and_then_cannot_be_paid_again() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    let wolf = named_permanent(&mut state, &reg, "Kessig Wolf", P1);      // not a Human
    let cathar = named_permanent(&mut state, &reg, "Elder Cathar", P1);   // a Human

    add_mana(&mut state, P0, &[(ManaType::Colorless, 1)]);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let targeted: Vec<&Target> = legal.actions.iter()
        .filter_map(|a| match a {
            Action::ActivateAbility { targets, .. } => targets.first(),
            _ => None,
        })
        .collect();
    assert!(targeted.contains(&&Target::Object(wolf)), "the non-Human is a legal target");
    assert!(!targeted.contains(&&Target::Object(cathar)), "the Human is not");

    state = activate_only_offered_ability(&state, &reg);
    assert!(state.get_object(wolf).unwrap().tapped, "the Wolf is tapped by the ability");
    assert!(state.get_object(priest).unwrap().tapped, "and the Priest by its own cost");

    add_mana(&mut state, P0, &[(ManaType::Colorless, 1)]);
    assert!(!offers_ability_of(&state, &reg, priest),
        "a tapped Priest cannot pay {{T}} again");
}

/// CR 608.2b: a target that stops satisfying what the ability *asks* of it is
/// illegal on resolution, not only one that has left or gained hexproof.
///
/// `stack.rs` names this card as the example — "Avacynian Priest's 'target
/// non-Human creature' is not a legal target once it has become a Human" — and
/// nothing tested it. A werewolf transforming back to its Human front face is
/// how a creature becomes a Human in this set: Villagers of Estwald is a Human
/// Werewolf, and its back face, Howlpack of Estwald, is not a Human.
#[test]
fn avacynian_priests_target_that_becomes_a_human_is_no_longer_legal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    let villager = named_permanent(&mut state, &reg, "Villagers of Estwald", P1);
    // On its back face it is a Werewolf and not a Human, so it is targetable.
    mtg_engine::cards::helpers::apply_transform(&mut state, villager, &reg);
    assert!(!state.has_subtype(villager, "Human", &reg),
        "test precondition: Howlpack of Estwald is not a Human");

    add_mana(&mut state, P0, &[(ManaType::Colorless, 1)]);
    let action = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == priest && targets == &[Target::Object(villager)]))
        .expect("the non-Human back face is a legal target");
    let mut state = mtg_engine::engine::submit_action(&state, &action, &reg);

    // In response it transforms back, and is a Human again.
    mtg_engine::cards::helpers::apply_transform(&mut state, villager, &reg);
    assert!(state.has_subtype(villager, "Human", &reg),
        "test precondition: the front face is a Human Werewolf");

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(!state.get_object(villager).unwrap().tapped,
        "'target non-Human creature' stopped being true of it, so the ability \
         is countered by game rules and it is not tapped");
    assert!(state.get_object(priest).unwrap().tapped,
        "the Priest still paid its {{T}} — costs are not refunded when an \
         ability is countered");
}

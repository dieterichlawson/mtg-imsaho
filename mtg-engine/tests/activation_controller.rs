//! CR 602.2a: an activated ability's controller is the player who activated
//! it, fixed when it is put on the stack.

mod common;
use common::*;
use mtg_engine::state::StackEntry;
use mtg_engine::types::*;

/// "Sacrifice this" is paid before the ability goes on the stack, and paying
/// it resets the source's controller to its owner (CR 108.4). The activator
/// used to be read off the source *after* that, so a stolen permanent's own
/// sacrifice ability was filed under its owner.
#[test]
fn a_stolen_permanents_sacrifice_ability_belongs_to_the_player_who_activated_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cathar = named_permanent(&mut state, &reg, "Selfless Cathar", P1);
    state.change_control(cathar, P0);
    add_mana(&mut state, P0, &[(ManaType::White, 1), (ManaType::Colorless, 1)]);

    let state = activate_onto_stack(&state, &reg, cathar, None);

    assert_eq!(state.get_object(cathar).unwrap().zone, Zone::Graveyard, "the cost was paid");
    match state.stack.last() {
        Some(StackEntry::Ability { activator, .. }) => assert_eq!(*activator, P0,
            "the ability's controller is the player who activated it (CR 602.2a), \
             not the owner the sacrificed card reverted to"),
        other => panic!("expected the ability on the stack, got {other:?}"),
    }
}

/// CR 601.2c/608.2b: what an ability asks of its target is fixed when it is
/// activated and re-checked on resolution — so it has to be read before the
/// costs are paid. Skirsdag Cultist paying "sacrifice a creature" with
/// itself is gone from the battlefield by the time the entry is built, and
/// a lookup through the source then found no ability at all: the entry
/// carried targets with no requirement (found by fuzzing).
#[test]
fn an_ability_whose_cost_removed_its_source_keeps_its_target_requirement() {
    use mtg_engine::actions::Action;
    use mtg_engine::cards::TargetRequirement;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 1)]);

    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let action = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, sacrifice: Some(s), .. } if *object_id == cultist && *s == cultist))
        .cloned()
        .unwrap_or_else(|| panic!("the Cultist can sacrifice itself to its own ability: {:?}", legal.actions));
    let state = mtg_engine::engine::submit_action(&state, &action, &reg);

    assert_eq!(state.get_object(cultist).unwrap().zone, Zone::Graveyard, "the cost was paid");
    match state.stack.last() {
        Some(StackEntry::Ability { targets, target_requirement, .. }) => {
            assert!(!targets.is_empty(), "the ability targets");
            assert!(matches!(target_requirement, Some(TargetRequirement::AnyTarget)),
                "the requirement read at activation rides on the entry (CR 601.2c), got {target_requirement:?}");
        }
        other => panic!("expected the ability on the stack, got {other:?}"),
    }
}

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

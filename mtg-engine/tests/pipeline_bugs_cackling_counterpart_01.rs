mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::PlayerId;
use mtg_engine::types::*;

fn make_zombie_token(state: &mut mtg_engine::state::GameState, registry: &CardRegistry, owner: PlayerId) -> mtg_engine::ids::ObjectId {
    let ids = state.create_token_with_subtypes(
        "Zombie",
        owner,
        2,
        2,
        vec![Color::Black],
        vec![CardType::Creature],
        vec![],
        vec!["Zombie".to_string()],
        registry,
    );
    ids.into_iter().next().unwrap()
}

#[test]
fn cackling_counterpart_copy_of_zombie_token_preserves_card_types() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let zombie = make_zombie_token(&mut state, &registry, P0);
    let copy = state.create_token_copy(zombie, P0, &registry);

    let copy_obj = state.get_object(copy).expect("copy token should exist");
    assert_eq!(
        copy_obj.card_types.contains(&CardType::Creature),
        true,
        "token copy of 2/2 black Zombie should have Creature card type, got {:?}",
        copy_obj.card_types,
    );
    assert_eq!(
        copy_obj.subtypes.contains(&"Zombie".to_string()),
        true,
        "token copy of 2/2 black Zombie should have Zombie subtype, got {:?}",
        copy_obj.subtypes,
    );
}

#[test]
fn cackling_counterpart_copy_of_zombie_token_preserves_color() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let zombie = make_zombie_token(&mut state, &registry, P0);
    let copy = state.create_token_copy(zombie, P0, &registry);

    let copy_obj = state.get_object(copy).expect("copy token should exist");
    assert_eq!(
        copy_obj.colors.contains(&Color::Black),
        true,
        "token copy of 2/2 black Zombie should be black, got colors={:?}",
        copy_obj.colors,
    );
}

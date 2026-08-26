mod common;
use common::*;
use mtg_engine::actions::Target;
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

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// CR 707.2: a token copy has the copiable characteristics of the original,
/// colour included. The test above covers copying a *token*, whose colours live
/// on the object; this one copies a real card, whose colours come from its face.
#[test]
fn a_token_copy_of_a_card_takes_that_cards_colors() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place a green creature
    let creature = named_creature(&mut state, &registry, "Grizzly Bears", P0);

    // Cast Cackling Counterpart targeting it
    let cc = castable_spell(&mut state, &registry, "Cackling Counterpart", P0);
    state = cast_and_resolve(&state, &registry, cc, vec![Target::Object(creature)]);

    // Find the token copy
    let token_id = find_token_named(&state, "Grizzly Bears")
        .expect("Token copy should exist");
    let token_colors = &state.get_object(token_id).unwrap().colors;

    assert_eq!(token_colors, &vec![Color::Green],
        "the copy of a green Bear is green — an empty colour list is the bug this \
         catches, and so is the wrong colour");
}

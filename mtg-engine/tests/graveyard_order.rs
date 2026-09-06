//! CR 404.2: "Each graveyard is kept in a single face-up pile. A player can
//! examine the cards in any graveyard at any time but normally can't change
//! their order." The pile has an order, it is arrival order, and it is public.

mod common;

use common::*;
use mtg_engine::types::*;

fn names_in_graveyard(state: &mtg_engine::state::GameState, player: PlayerId) -> Vec<String> {
    state.objects_in_zone(Zone::Graveyard, player)
        .iter().map(|o| o.name.clone()).collect()
}

/// The pile is in the order the cards arrived, not the order the decklist
/// created them in — which is what sorting by object id gave, and which put
/// three Devil's Plays ahead of a Blasphemous Act that arrived second
/// (issue #222).
#[test]
fn the_graveyard_is_in_arrival_order_not_object_id_order() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Created in one order...
    let first_made = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    let second_made = spell_in_hand(&mut state, &reg, "Blasphemous Act", P0);
    let third_made = spell_in_hand(&mut state, &reg, "Geistflame", P0);
    assert!(first_made < second_made && second_made < third_made, "test precondition");

    // ...and arriving in another.
    state.move_object(second_made, Zone::Graveyard, &reg);
    state.move_object(third_made, Zone::Graveyard, &reg);
    state.move_object(first_made, Zone::Graveyard, &reg);

    assert_eq!(names_in_graveyard(&state, P0),
        vec!["Blasphemous Act", "Geistflame", "Devil's Play"],
        "the pile is in arrival order, bottom first");
}

/// A card that leaves the graveyard leaves the pile, and coming back puts it
/// on top — a flashback card exiled and a creature reanimated must not hold
/// their old places.
#[test]
fn leaving_the_graveyard_leaves_the_pile() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let a = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    let b = spell_in_hand(&mut state, &reg, "Geistflame", P0);
    for id in [a, b] {
        state.move_object(id, Zone::Graveyard, &reg);
    }
    assert_eq!(names_in_graveyard(&state, P0), vec!["Devil's Play", "Geistflame"]);

    // The first one is exiled (flashback), then a new card arrives.
    state.move_object(a, Zone::Exile, &reg);
    assert_eq!(names_in_graveyard(&state, P0), vec!["Geistflame"]);
    assert_eq!(state.get_player(P0).graveyard_order.len(), 1,
        "the exiled card is out of the pile as well as out of the zone");

    let c = spell_in_hand(&mut state, &reg, "Blasphemous Act", P0);
    state.move_object(c, Zone::Graveyard, &reg);
    assert_eq!(names_in_graveyard(&state, P0), vec!["Geistflame", "Blasphemous Act"]);
}

/// A mill puts cards in the graveyard from the top of the library, in order,
/// and the pile records that order — which is the question a self-mill deck
/// asks every turn.
#[test]
fn a_mill_lands_in_the_pile_in_the_order_it_milled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    for name in ["Island", "Mulch", "Boneyard Wurm"] {
        let card = spell_in_hand(&mut state, &reg, name, P0);
        state.get_object_mut(card).unwrap().zone = Zone::Library;
        state.get_player_mut(P0).library_order.push(card);
    }

    mtg_engine::engine::mill_cards(&mut state, P0, 3, "test mill", &reg);

    assert_eq!(names_in_graveyard(&state, P0), vec!["Island", "Mulch", "Boneyard Wurm"],
        "milled cards arrive top-of-library first, and the pile says so");
}

/// Each player's pile is their own (CR 404.3).
#[test]
fn the_two_graveyards_are_separate_piles() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    let theirs = spell_in_hand(&mut state, &reg, "Geistflame", P1);
    state.move_object(theirs, Zone::Graveyard, &reg);
    state.move_object(mine, Zone::Graveyard, &reg);

    assert_eq!(names_in_graveyard(&state, P0), vec!["Devil's Play"]);
    assert_eq!(names_in_graveyard(&state, P1), vec!["Geistflame"]);
}

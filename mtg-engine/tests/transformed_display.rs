//! Regression test for Bug B (BUG_REPORT_8SEAT.md): the PermanentView
//! built from a transformed DFC was reading the name and card_types
//! from registry.card_data() which unconditionally returns the front
//! face, so a transformed Villagers of Estwald would render as
//! "Villagers of Estwald" with back-face stats (4/6).

mod common;
use common::*;

use mtg_engine::cards::helpers;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::PlayerId;
use mtg_engine::types::Step;
use mtg_engine::view::GameView;

const P0: PlayerId = PlayerId(0);

#[test]
fn untransformed_dfc_shows_front_face_name() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = named_creature(&mut state, &registry, "Villagers of Estwald", P0);

    let view = GameView::for_player(&state, P0, &registry);
    let perm = view.battlefield.iter().find(|p| p.object_id == id).unwrap();
    assert_eq!(perm.name, "Villagers of Estwald");
    assert_eq!(perm.effective_power, Some(2));
    assert_eq!(perm.effective_toughness, Some(3));
}

#[test]
fn transformed_dfc_shows_back_face_name() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = named_creature(&mut state, &registry, "Villagers of Estwald", P0);
    helpers::apply_transform(&mut state, id, &registry);

    let view = GameView::for_player(&state, P0, &registry);
    let perm = view.battlefield.iter().find(|p| p.object_id == id).unwrap();
    assert_eq!(perm.name, "Howlpack of Estwald",
        "Transformed DFC must show back-face name, not front face");
    assert_eq!(perm.effective_power, Some(4));
    assert_eq!(perm.effective_toughness, Some(6));
}

#[test]
fn transformed_kruin_outlaw_shows_terror_name() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = named_creature(&mut state, &registry, "Kruin Outlaw", P0);
    helpers::apply_transform(&mut state, id, &registry);

    let view = GameView::for_player(&state, P0, &registry);
    let perm = view.battlefield.iter().find(|p| p.object_id == id).unwrap();
    assert_eq!(perm.name, "Terror of Kruin Pass");
}

#[test]
fn transformed_civilized_scholar_shows_homicidal_brute_name() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = named_creature(&mut state, &registry, "Civilized Scholar", P0);
    helpers::apply_transform(&mut state, id, &registry);

    let view = GameView::for_player(&state, P0, &registry);
    let perm = view.battlefield.iter().find(|p| p.object_id == id).unwrap();
    assert_eq!(perm.name, "Homicidal Brute");
}

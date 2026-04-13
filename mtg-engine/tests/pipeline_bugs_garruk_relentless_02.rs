mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use mtg_engine::view::GameView;

/// Bug: Garruk Relentless does not implement back_face_data(), so when
/// transformed into "Garruk, the Veil-Cursed", the view layer calls
/// back_face_data() and gets None. This causes the oracle_text field in
/// PermanentView to be an empty string (via unwrap_or_default()).
///
/// Oracle text for the back face (Garruk, the Veil-Cursed):
///   +1: Create a 1/1 black Wolf creature token with deathtouch.
///   -1: Sacrifice a creature. If you do, search your library for a creature
///       card, reveal it, put it into your hand, then shuffle.
///   -3: Creatures you control gain trample and get +X/+X until end of turn,
///       where X is the number of creature cards in your graveyard.
///
/// The PermanentView.oracle_text should contain the back face oracle text
/// when Garruk is transformed. Instead it is empty because back_face_data()
/// returns None (the trait default).
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn transformed_garruk_view_shows_back_face_oracle_text() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Garruk Relentless on the battlefield for P0 with 2 loyalty
    // (at or below the transform threshold).
    let garruk_card_id = registry.get_id_by_name("Garruk Relentless").unwrap();
    let garruk = state.create_object(garruk_card_id, P0, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(garruk).unwrap();
        obj.name = "Garruk Relentless".into();
        obj.card_types = vec![CardType::Planeswalker];
        obj.is_legendary = true;
    }
    state.add_counters(garruk, CounterType::Loyalty, 2);

    // Transform Garruk via on_state_trigger (the same code path the engine uses).
    // This sets is_transformed = true and name = "Garruk, the Veil-Cursed"
    // but does NOT call apply_transform and does NOT set back_face_data().
    let behavior = registry.get(garruk_card_id).unwrap();
    behavior.on_state_trigger(&mut state, garruk, &registry);

    // Verify Garruk is transformed.
    let obj = state.get_object(garruk).unwrap();
    assert_eq!(obj.is_transformed, true, "Garruk should be transformed");
    assert_eq!(obj.name, "Garruk, the Veil-Cursed");

    // Generate the game view — this is what the AI player sees.
    let view = GameView::for_player(&state, P0, &registry);

    // Find transformed Garruk in the battlefield view.
    let garruk_view = view.battlefield.iter()
        .find(|p| p.object_id == garruk)
        .expect("Transformed Garruk should be on the battlefield in the view");

    // Oracle-correct behavior: the view should show the back face oracle text,
    // which includes the +1 deathtouch wolf, -1 sacrifice/tutor, and -3 pump
    // abilities. Because back_face_data() is not implemented, the view shows
    // an empty string instead.
    assert_ne!(
        garruk_view.oracle_text.as_str(), "",
        "Transformed Garruk's oracle_text should contain back-face abilities, \
         but it is empty because back_face_data() returns None"
    );

    assert_eq!(
        garruk_view.oracle_text.contains("deathtouch"), true,
        "Back face oracle text should mention 'deathtouch' (from the +1 ability), \
         got: '{}'",
        garruk_view.oracle_text
    );
}

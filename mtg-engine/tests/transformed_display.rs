//! Regression test for Bug B (`BUG_REPORT_8SEAT.md`): the `PermanentView` built
//! from a transformed DFC read its name and `card_types` from
//! `registry.card_data()`, which unconditionally returns the FRONT face — so a
//! transformed Villagers of Estwald rendered as "Villagers of Estwald" with
//! back-face stats (4/6).
//!
//! The view is what a player actually sees, so it has to follow the active
//! face. Four tests used to check three named cards; this checks every DFC in
//! the set, on both faces.

mod common;
use common::*;

use mtg_engine::cards::helpers;
use mtg_engine::types::Step;
use mtg_engine::view::GameView;

#[test]
fn the_view_shows_whichever_face_a_dfc_is_currently_on() {
    let reg = registry();

    let dfcs: Vec<String> = reg.all_names().iter()
        .filter(|n| reg.get_id_by_name(n)
            .and_then(|id| reg.get(id))
            .is_some_and(|b| b.back_face_data().is_some()))
        .map(|n| (*n).to_string())
        .collect();
    assert!(dfcs.len() >= 10,
        "only {} DFCs found — this sweep has stopped covering the set", dfcs.len());

    for name in &dfcs {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let id = named_permanent(&mut state, &reg, name, P0);
        let behavior = reg.get(state.get_object(id).unwrap().card_id).unwrap();
        let front = behavior.card_data();
        let back = behavior.back_face_data().unwrap();

        let shown = |state: &mtg_engine::state::GameState| {
            let view = GameView::for_player(state, P0, &reg);
            let perm = view.battlefield.iter()
                .find(|p| p.object_id == id)
                .unwrap_or_else(|| panic!("{name} missing from its controller's view"));
            (perm.name.clone(), perm.effective_power, perm.effective_toughness)
        };

        let (shown_name, p, t) = shown(&state);
        assert_eq!(shown_name, front.name, "{name} should render as its front face");
        // A characteristic-defining ability sets P/T from game state, so only
        // compare against the printed box when the card has one printed.
        if behavior.dynamic_pt(&state, id, &reg).is_none() {
            assert_eq!((p, t), (front.power, front.toughness),
                "{name} should render its front-face size");
        }

        helpers::apply_transform(&mut state, id, &reg);

        let (shown_name, p, t) = shown(&state);
        assert_eq!(shown_name, back.name,
            "a transformed {name} must render as {}, not the front face", back.name);
        if behavior.dynamic_pt(&state, id, &reg).is_none() {
            assert_eq!((p, t), (back.power, back.toughness),
                "a transformed {name} should render its back-face size");
        }
    }
}

//! CR 712.8a: a double-faced card that is not on the battlefield or the stack
//! has only its front-face characteristics. When a transformed DFC changes
//! zones, the engine must put the front face back — name, keywords, subtypes,
//! and the `is_transformed` flag.

mod common;
use common::*;

use mtg_engine::cards::helpers::apply_transform;
use mtg_engine::types::*;

/// Every double-faced card in the set, as (front name, back name).
fn every_dfc() -> Vec<(String, String)> {
    let reg = registry();
    let mut out: Vec<(String, String)> = reg
        .all_names()
        .iter()
        .filter_map(|name| {
            let id = reg.get_id_by_name(name)?;
            let back = reg.get(id)?.back_face_data()?;
            Some(((*name).to_string(), back.name))
        })
        .collect();
    out.sort();
    out
}

/// Transform a named DFC on the battlefield, then move it to `dest`.
fn transform_then_move(
    front_name: &str,
    dest: Zone,
) -> (mtg_engine::state::GameState, mtg_engine::ids::ObjectId, mtg_engine::cards::CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = named_permanent(&mut state, &reg, front_name, P0);
    apply_transform(&mut state, id, &reg);
    state.move_object(id, dest, &reg);
    (state, id, reg)
}

/// The whole family at once, in both zones a permanent can leave to. Naming
/// eight of the twelve DFCs one test each left the other four untested and a
/// thirteenth card uncovered the day it is added.
#[test]
fn every_transformed_dfc_shows_its_front_face_after_leaving_the_battlefield() {
    let dfcs = every_dfc();
    assert!(dfcs.len() >= 10,
        "only {} DFCs found — this sweep has stopped covering the set", dfcs.len());

    for (front, back) in &dfcs {
        for dest in [Zone::Graveyard, Zone::Exile] {
            let (state, id, reg) = transform_then_move(front, dest);
            let obj = state.get_object(id).unwrap();

            assert!(!obj.is_transformed,
                "{front} in {dest:?} is still flagged transformed");
            assert_eq!(obj.name, *front,
                "{front} in {dest:?} kept its back-face name {back:?}");

            // Characteristics come from the active face, so they must be the
            // front face's now — checked through the accessors, since
            // `obj.subtypes` / `obj.keywords` hold only runtime grants.
            let front_data = reg.card_data(obj.card_id).unwrap();
            let back_data = reg.get(obj.card_id).unwrap().back_face_data().unwrap();

            for sub in &front_data.subtypes {
                assert!(state.has_subtype(id, sub, &reg),
                    "{front} in {dest:?} lost its front-face subtype {sub:?}; has {:?}",
                    state.subtypes_of(id, &reg));
            }
            for sub in &back_data.subtypes {
                if !front_data.subtypes.contains(sub) {
                    assert!(!state.has_subtype(id, sub, &reg),
                        "{front} in {dest:?} kept the back face's {sub:?} subtype; has {:?}",
                        state.subtypes_of(id, &reg));
                }
            }
            // Keywords only function on the battlefield, so `has_keyword` is
            // false for everything in a graveyard — asserting on it here would
            // pass no matter which face the object thinks it has. Bring it back
            // instead: a Delver that transformed, died and was reanimated must
            // not still be flying.
            let mut state = state;
            state.move_object(id, Zone::Battlefield, &reg);
            for kw in &back_data.keywords {
                if !front_data.keywords.contains(kw) {
                    assert!(!state.has_keyword(id, *kw, &reg),
                        "{front} came back from {dest:?} still carrying the back face's {kw:?}");
                }
            }
            for kw in &front_data.keywords {
                assert!(state.has_keyword(id, *kw, &reg),
                    "{front} came back from {dest:?} without its front-face {kw:?}");
            }
        }
    }
}

/// The reset has to hold for a flipped state however it was reached, so this
/// one writes `is_transformed` and `name` by hand on purpose — every other
/// test goes through `apply_transform`.
///
/// (The comment here used to justify that by saying Garruk transforms this way
/// himself. He does not: `garruk_relentless.rs` calls `apply_transform` like
/// everything else. The hand-written state is still the point of the test.)
#[test]
fn garruk_name_resets_after_zone_change() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);

    let obj = state.get_object_mut(garruk).unwrap();
    obj.is_transformed = true;
    obj.name = "Garruk, the Veil-Cursed".into();
    assert_eq!(state.get_object(garruk).unwrap().name, "Garruk, the Veil-Cursed",
        "test precondition");

    state.move_object(garruk, Zone::Graveyard, &reg);

    let obj = state.get_object(garruk).unwrap();
    assert!(!obj.is_transformed);
    assert_eq!(obj.name, "Garruk Relentless",
        "CR 712.8a: a DFC in the graveyard has its front-face name");
}

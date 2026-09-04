//! A saved game is a reproducible artifact: the same state always
//! serializes to the same bytes.
//!
//! `GameState.objects` was a `HashMap`, and Rust's default hasher is seeded
//! per process, so two identical seeded runs wrote semantically equal but
//! byte-different saves (issue #199). `cmp`/`sha256sum` could not confirm
//! two runs matched, a triage `diff` showed the whole file as changed, and
//! any test asserting save equality would flake.
//!
//! The fix is ordered maps — the same choice `ManaPool` already made for the
//! same reason — so the guarantee to pin is that keys come out in sorted
//! order. Sorted order is what makes the bytes identical across processes,
//! and unlike a byte comparison it can be checked inside one process, where
//! the per-process hasher would otherwise hide the bug.

mod common;
use common::*;
use mtg_engine::types::*;

/// Build a game with enough objects that an unordered map would be
/// vanishingly unlikely to emit them in ascending order by chance.
fn a_populated_game() -> (mtg_engine::state::GameState, mtg_engine::cards::CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for i in 0..20 {
        let owner = if i % 2 == 0 { P0 } else { P1 };
        let id = named_permanent(&mut state, &reg, "Grizzly Bears", owner);
        state.add_counters(id, CounterType::PlusOnePlusOne, 1 + i % 3);
        named_permanent(&mut state, &reg, "Forest", owner);
        spell_in_hand(&mut state, &reg, "Brimstone Volley", owner);
    }
    (state, reg)
}

#[test]
fn a_saved_game_serializes_its_objects_in_id_order() {
    let (state, _reg) = a_populated_game();
    let json = serde_json::to_string(&state).expect("state serializes");

    let objects = &json[json.find("\"objects\":{").expect("the save carries its objects")..];
    let mut ids = Vec::new();
    let mut rest = objects;
    while let Some(at) = rest.find("\":{\"id\":") {
        let key_start = rest[..at].rfind('"').expect("a quoted key");
        if let Ok(id) = rest[key_start + 1..at].parse::<u64>() {
            ids.push(id);
        }
        rest = &rest[at + 1..];
    }

    assert!(ids.len() >= 60, "the fixture has plenty of objects, got {}", ids.len());
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted,
        "objects serialize in ascending id order, which is what makes two runs \
         of the same game byte-identical; got {:?}...", &ids[..8.min(ids.len())]);
}

/// Serializing the same state twice gives the same bytes, and so does
/// serializing it after a round-trip through the save format — the property
/// an operator relies on when comparing two runs with `sha256sum`.
#[test]
fn the_same_game_always_produces_the_same_bytes() {
    let (state, _reg) = a_populated_game();

    let once = serde_json::to_string(&state).expect("serializes");
    let twice = serde_json::to_string(&state).expect("serializes");
    assert_eq!(once, twice, "one state, one serialization");

    let reloaded: mtg_engine::state::GameState =
        serde_json::from_str(&once).expect("a save reloads");
    let after_round_trip = serde_json::to_string(&reloaded).expect("serializes");
    assert_eq!(once, after_round_trip,
        "a save reloaded and rewritten is byte-identical to the original");
}

//! Failing tests for harness / display bugs documented in
//! `audits/AUDIT_BUGS.md`. These bugs are about the labels and prompts
//! the LLM player sees, not the underlying game state.
//!
//! Bugs covered in this file:
//! - Bug 31-001: `PendingTrigger::display_name` uses front-face card
//!   names for transformed DFCs — the stack shows "Tormented Pariah's
//!   upkeep trigger" even though the battlefield has "Rampaging
//!   Werewolf".

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

/// Bug 31-001 (`audits/AUDIT_BUGS.md)`: `PendingTrigger::display_name`
/// at `triggers.rs` builds its labels with a closure that
/// calls `registry.card_data(card_id)` — which always returns the
/// FRONT face. So a transformed DFC's trigger label includes the
/// front-face name, mismatching the battlefield display (which
/// correctly shows the back-face name post-Bug-B fix).
///
/// Oracle (Tormented Pariah front face): "Human Warrior Werewolf".
/// Oracle (Rampaging Werewolf back face): "Werewolf".
///
/// Failure mode: the `card_name` closure reads `registry.card_data`
/// without consulting `state.get_object(obj_id).is_transformed`. The
/// fix would either thread a `&GameState` into `display_name` or
/// store `is_transformed` on each `PendingTrigger` variant at
/// collection time.
///
/// We construct a `PendingTrigger::UpkeepTrigger` for a transformed
/// Tormented Pariah and check that the display label contains the
/// back-face name ("Rampaging Werewolf") — or at least does NOT
/// contain the front-face name. Today, `display_name` returns the
/// front-face name, so either assertion catches the bug.
///
/// Note: this test calls the `display_name(&CardRegistry)` shape
/// that exists today. The fix will change the signature to take a
/// `&GameState` as well — that signature change itself is the
/// "fix the bug" delta, so this test documents the pre-fix
/// behavior that will need to be replaced rather than asserting
/// the post-fix label shape.
#[test]
fn bug_31_001_pending_trigger_label_uses_back_face_name_for_transformed_dfc() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Upkeep, P0);

    // Tormented Pariah transformed to Rampaging Werewolf.
    let pariah = named_permanent(&mut state, &registry, "Tormented Pariah", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, pariah, &registry);
    let pariah_card_id = state.get_object(pariah).unwrap().card_id;

    let trigger = PendingTrigger {
        source: TriggerSource::new(pariah, pariah_card_id, P0, "transform back if 2+ spells cast"),
        event: TriggerEvent::Upkeep,
    };
    let label = trigger.display_name_with_state(&registry, Some(&state));

    assert!(
        !label.contains("Tormented Pariah"),
        "PendingTrigger::display_name for a transformed DFC should NOT \
         use the front-face name 'Tormented Pariah' — the battlefield \
         shows 'Rampaging Werewolf' post-transform, and the stack \
         label should match. Bug 31-001: the `card_name` closure calls \
         registry.card_data() which always returns the front face. \
         label = {label:?}",
    );
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Boneyard Wurm's power/toughness is dynamic (= creature cards in
/// your graveyard), but the `GameView` shows base P/T (0/0) from obj.power.
/// The view should use `effective_power/effective_toughness`.
#[test]
fn bug_boneyard_wurm_view_shows_base_pt() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 3 creature cards in P0's graveyard
    for _ in 0..3 {
        let card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
        let _id = state.create_object(card_id, P0, Zone::Graveyard, Some(2), Some(2));
    }

    // Place Boneyard Wurm
    let wurm = named_permanent(&mut state, &registry, "Boneyard Wurm", P0);

    // Effective P/T should be 3/3 (dynamic)
    let eff_p = state.effective_power(wurm, &registry).unwrap_or(0);
    assert_eq!(eff_p, 3, "Boneyard Wurm should be 3/3 with 3 creatures in GY");

    // Build GameView and check what it reports
    let view = mtg_engine::view::GameView::for_player(&state, P0, &registry);

    // Find the Wurm in the view
    let wurm_view = view.battlefield.iter()
        .find(|c| c.name == "Boneyard Wurm");

    if let Some(wv) = wurm_view {
        // BUG: View shows base P/T (0/0 or None) instead of effective (3/3)
        assert_eq!(wv.effective_power, Some(3),
            "GameView should show effective power 3, got {:?}", wv.effective_power);
    } else {
        panic!("Boneyard Wurm not found in view");
    }
}

/// Bug 76-001 (`audits/AUDIT_BUGS.md)`: Skirsdag High Priest's
/// `activated_abilities` formats the candidate creature `ObjectIds`
/// with Rust's `{:?}` debug format, so the LLM player sees labels
/// like `... (tap ObjectId(5) & ObjectId(12))`. `ObjectIds` are
/// internal handles — the model has no way to map them to creature
/// names.
///
/// Oracle (Skirsdag High Priest): "{T}, Tap two untapped creatures
/// you control: Create a 5/5 black Demon creature token with flying.
/// Activate this ability only if you control three or more creatures.
/// Morbid — ..."
///
/// Failure mode: `skirsdag_high_priest.rs` calls
/// `format!(... "tap {:?} & {:?} ...", candidates[i], candidates[j])`.
/// Since `candidates[i]` is an `ObjectId`, this renders the literal
/// `ObjectId(N)` substring. Compare with `format_combat_creature_list`
/// in `mtg-player/src/llm.rs`, which uses creature names
/// with `#1`/`#2` suffixes for collisions.
#[test]
fn bug_76_001_skirsdag_high_priest_label_has_no_object_id_debug() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Skirsdag High Priest needs the morbid condition AND at least
    // three creatures total to enable the ability — so put one extra
    // creature on top of the High Priest and the two tap candidates.
    state.creature_died_this_turn = true;
    let _high_priest = named_permanent(&mut state, &registry, "Skirsdag High Priest", P0);
    let _victim_a = ready_creature(&mut state, P0, 2, 2);
    let _victim_b = ready_creature(&mut state, P0, 2, 2);

    let priest_card_id = registry.get_id_by_name("Skirsdag High Priest").unwrap();
    let priest_obj = state
        .objects
        .values()
        .find(|o| o.card_id == priest_card_id)
        .map(|o| o.id)
        .expect("Skirsdag High Priest should be on the battlefield");
    let behavior = registry.get(priest_card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, priest_obj, &registry);

    assert!(
        !abilities.is_empty(),
        "Test setup: Skirsdag High Priest should expose at least one \
         enumerated tap-pair ability with morbid + 2 candidates"
    );
    for ab in &abilities {
        assert!(
            !ab.description.contains("ObjectId("),
            "Skirsdag High Priest's activation label should not contain \
             the Rust debug format 'ObjectId('. Bug 76-001: the format \
             string uses {{:?}} which renders ObjectIds as ObjectId(N). \
             description = {:?}",
            ab.description,
        );
    }
}

/// Bug E1-002 (`audits/AUDIT_BUGS.md)`: The `CardView` projection in
/// `mtg-engine/src/view.rs` reads `obj.power` / `obj.toughness`
/// directly when building hand/graveyard/library views, so CDA
/// creatures like Geist-Honored Monk render as their printed base
/// (0/0) to the LLM.
///
/// Oracle (Geist-Honored Monk): "Power and toughness each equal to
/// the number of creatures you control."
///
/// Per CR 208.2, a characteristic-defining ability "works in all
/// zones." The view projection should consult `state.effective_power`
/// / `state.effective_toughness`, not the raw `obj.power` /
/// `obj.toughness` fields.
///
/// We put Geist-Honored Monk in P0's graveyard with another creature
/// on P0's battlefield (so Monk's CDA value ≥1), then build a
/// `GameView` and check the `CardView` for the Monk's effective P/T.
#[test]
fn bug_e1_002_cardview_uses_effective_pt_for_cda_creatures() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Another creature on P0's battlefield so Geist-Honored Monk's
    // CDA count is non-zero.
    let _bears = named_permanent(&mut state, &registry, "Grizzly Bears", P0);

    // Geist-Honored Monk in P0's graveyard.
    let monk_card_id = registry.get_id_by_name("Geist-Honored Monk").unwrap();
    let monk = state.create_object(monk_card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(monk).unwrap().name = "Geist-Honored Monk".into();

    // Sanity: effective_power says the Monk is ≥1/1.
    let eff_p = state.effective_power(monk, &registry).unwrap_or(0);
    let eff_t = state.effective_toughness(monk, &registry).unwrap_or(0);
    assert!(
        eff_p >= 1 && eff_t >= 1,
        "Test setup: Geist-Honored Monk with 1 creature on bf should \
         have effective P/T ≥ 1/1, got {eff_p}/{eff_t}"
    );

    // Build a GameView from P0's perspective. The Monk's graveyard
    // CardView should reflect the effective P/T, not the base 0/0.
    let view = mtg_engine::view::GameView::for_player(&state, P0, &registry);
    let monk_in_gy = view
        .graveyards
        .iter()
        .find_map(|(pid, cards)| {
            if *pid == P0 {
                cards.iter().find(|c| c.object_id == monk).cloned()
            } else {
                None
            }
        });
    let monk_view = monk_in_gy.expect("Monk should appear in P0's graveyard CardView");

    let visible_power = monk_view.power.unwrap_or(0);
    assert!(
        visible_power >= 1,
        "GameView's graveyard CardView for Geist-Honored Monk should \
         reflect the effective power (≥1 with creatures on the \
         battlefield), not the printed base 0. Bug E1-002: view.rs \
         reads obj.power directly. CardView.power = {:?}",
        monk_view.power,
    );
}

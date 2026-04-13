//! Failing tests for harness / display bugs documented in
//! audits/AUDIT_BUGS.md. These bugs are about the labels and prompts
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
use mtg_engine::triggers::PendingTrigger;
use mtg_engine::types::*;

/// Bug 31-001 (audits/AUDIT_BUGS.md): `PendingTrigger::display_name`
/// at `triggers.rs:193-260` builds its labels with a closure that
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
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 31-001 is fixed.
#[test]
fn bug_31_001_pending_trigger_label_uses_back_face_name_for_transformed_dfc() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Upkeep, P0);

    // Tormented Pariah transformed to Rampaging Werewolf.
    let pariah = named_creature(&mut state, &registry, "Tormented Pariah", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, pariah, &registry);
    let pariah_card_id = state.get_object(pariah).unwrap().card_id;

    let trigger = PendingTrigger::UpkeepTrigger {
        object_id: pariah,
        card_id: pariah_card_id,
        controller: P0,
        description: "transform back if 2+ spells cast".into(),
    };
    let label = trigger.display_name(&registry);

    assert!(
        !label.contains("Tormented Pariah"),
        "PendingTrigger::display_name for a transformed DFC should NOT \
         use the front-face name 'Tormented Pariah' — the battlefield \
         shows 'Rampaging Werewolf' post-transform, and the stack \
         label should match. Bug 31-001: the `card_name` closure calls \
         registry.card_data() which always returns the front face. \
         label = {:?}",
        label,
    );
}

//! Failing tests for bugs documented in audits/AUDIT_BUGS.md.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers the "Token copy — `create_token_*` doesn't
//! preserve all source state" family. The token-copy helpers in
//! `mtg-engine/src/state.rs` lose information when creating tokens
//! that copy a source permanent — dynamic P/T, `is_legendary`, and
//! some peer issues like Evil Twin's broken enter-as-copy lifecycle.
//!
//! Bugs covered in this file:
//! - Bug 0F-002: `create_token_copy` doesn't propagate
//!   `is_legendary`, so a token copy of a legendary creature
//!   evades the legend rule
//! - Bug BJ: Evil Twin enters as a 0/0 and dies to SBA before its
//!   ETB copy trigger resolves

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Bug 0F-002 (audits/AUDIT_BUGS.md): `state.create_token_copy` never
/// reads `card_data.supertypes` and therefore leaves `obj.is_legendary
/// = false` on the token, even when copying a legendary creature.
/// The legend rule (CR 704.5j) keys on `obj.is_legendary && obj.name`,
/// so the original and the token coexist on the same controller's
/// battlefield indefinitely.
///
/// Oracle (Olivia Voldaren): "Legendary Creature — Vampire" (cost
/// {2}{B}{R}, 3/3, Flying).
/// Oracle (Cackling Counterpart): "Create a token that's a copy of
/// target creature you control."
///
/// Failure mode: `state.rs:432-447` (`create_token_copy`) creates the
/// token via `create_token_with_subtypes` (which always sets
/// `is_legendary: false`), then patches `obj.card_id`. It never sets
/// `obj.is_legendary` from `card_data.supertypes`. SBA's legend-rule
/// loop (`sba.rs:248-269`) walks `is_legendary && obj.name` matches
/// — neither the source Olivia nor the token-copy Olivia gets
/// flagged because the token's `is_legendary = false`, so the SBA
/// finds zero pairs and lets both stay.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 0F-002 is fixed.
#[test]
fn bug_0f_002_token_copy_of_legendary_creature_is_legendary() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_creature(&mut state, &registry, "Olivia Voldaren", P0);
    assert!(
        state.get_object(olivia).unwrap().is_legendary,
        "Test setup: cast-from-hand Olivia should be flagged legendary"
    );

    let token = state.create_token_copy(olivia, P0, &registry);
    assert!(token.0 != 0, "Test setup: token should have been created");

    let token_is_legendary = state.get_object(token).unwrap().is_legendary;
    assert!(
        token_is_legendary,
        "A token copy of Olivia Voldaren must be flagged is_legendary so \
         the legend rule (CR 704.5j) can knock one of the two pairs out. \
         Bug 0F-002: create_token_copy patches card_id but never reads \
         card_data.supertypes or sets obj.is_legendary."
    );
}

/// Bug BJ (audits/AUDIT_BUGS.md): Evil Twin's "enter as a copy"
/// effect is implemented as an ETB triggered ability rather than as a
/// CR 614.1d replacement effect. Evil Twin's `card_data` declares
/// `power: Some(0), toughness: Some(0)`, and SBA 704.5f runs before
/// the ETB trigger resolves, so Evil Twin is destroyed in the
/// graveyard before the copy decision ever happens.
///
/// Oracle (Evil Twin): "You may have this creature enter as a copy of
/// any creature on the battlefield, except it has '{U}{B}, {T}: Destroy
/// target creature with the same name as this creature.'" (CR 614.1d
/// "enter as a copy" — replacement, not trigger.)
///
/// Failure mode: `evil_twin.rs:43-61` builds an EntersBattlefield
/// trigger that calls `present_optional_target_choice`. The priority
/// loop at `engine.rs:4085-4096` runs SBA *before* `collect_triggers`
/// pushes the ETB trigger onto the stack, and SBA 704.5f kills any
/// creature with toughness ≤ 0 — including a freshly entered 0/0
/// Evil Twin. By the time the ETB trigger resolves, Evil Twin is
/// already in the graveyard.
///
/// We put Evil Twin onto the battlefield in its native 0/0 form (the
/// state right after entering, before triggers fire) and run SBA. The
/// fix should make Evil Twin survive SBA so its copy effect can apply.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BJ is fixed.
#[test]
fn bug_bj_evil_twin_survives_sba_before_copy_effect_resolves() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Another creature in play so Evil Twin's copy choice has options
    // (otherwise Evil Twin's ETB silently no-ops).
    let _bears = ready_creature(&mut state, P1, 2, 2);

    // Evil Twin enters the battlefield as the printed 0/0 Shapeshifter.
    let evil_twin_card_id = registry.get_id_by_name("Evil Twin").unwrap();
    let twin = state.create_object(
        evil_twin_card_id,
        P0,
        Zone::Battlefield,
        Some(0),
        Some(0),
    );
    state.get_object_mut(twin).unwrap().name = "Evil Twin".into();
    state.get_object_mut(twin).unwrap().card_types = vec![CardType::Creature];

    // Run SBA. Today this kills Evil Twin (0 toughness → SBA 704.5f).
    // Post-fix, Evil Twin should survive long enough for its copy
    // mechanic to apply (whether via dynamic_pt override, replacement
    // effect, or some other mechanism).
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    let still_on_battlefield = state.get_object(twin).map(|o| o.zone) == Some(Zone::Battlefield);
    assert!(
        still_on_battlefield,
        "Evil Twin should not be killed by SBA on entry — its copy \
         effect (CR 614.1d 'enter as a copy') needs the chance to apply \
         first. Bug BJ: evil_twin.rs declares 0/0 base P/T and uses an \
         ETB trigger for the copy, so SBA 704.5f kills it before the \
         trigger resolves. Current zone: {:?}",
        state.get_object(twin).map(|o| o.zone),
    );
}

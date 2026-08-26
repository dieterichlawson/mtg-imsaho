//! Failing tests for bugs documented in `audits/AUDIT_BUGS.md`.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers the "Transform / DFC" family — bugs specific to
//! double-faced cards and transform mechanics.
//!
//! Bugs covered in this file:
//! - Bug 99-003: Daybreak Ranger's `is_valid_target` searches for
//!   *any* Daybreak Ranger the caster controls — two copies with
//!   different transform states cross-contaminate.
//! - Bug BE: Garruk Relentless dies before transforming when damage
//!   takes him from 3+ loyalty straight to 0 — SBA removes him
//!   before the state-trigger runs.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Bug 99-003 (`audits/AUDIT_BUGS.md)`: Daybreak Ranger's
/// `is_valid_target` scans `state.objects.values()` for *any*
/// Daybreak Ranger the caster controls and uses THAT ranger's
/// `is_transformed` flag to decide which branch to apply. With two
/// rangers in different transform states, the filter consults the
/// wrong source.
///
/// Oracle (Daybreak Ranger, front face): "{T}: This creature deals
/// 2 damage to target creature with flying."
/// Oracle (Nightfall Predator, back face): "{R}, {T}: This creature
/// fights target creature."
///
/// The two abilities have DIFFERENT target filters: front face
/// requires flying, back face allows any creature. Because the
/// `CardBehavior::is_valid_target` trait method doesn't receive
/// the activating `source_id`, the card's handler hand-rolls its
/// own source lookup via `.find()` — but `.find()` returns any
/// Ranger, not the ACTIVE one.
///
/// Failure mode: with `ranger_a` (front face) + `ranger_b`
/// (transformed, back face) under the same controller, querying
/// `is_valid_target` for a non-flying creature returns the
/// result for whichever Ranger `.find()` happens to surface first.
/// If it surfaces `ranger_b` (transformed → "any creature"
/// allowed), the non-flying creature is incorrectly offered as a
/// valid target for `ranger_a`'s front-face ability.
///
/// We use `legal_actions` to observe the bug end-to-end:
/// `ranger_a`'s front-face "deal 2 damage to target creature with
/// flying" ability should NOT appear with a non-flying target, but
/// today it does (when `ranger_b` is also in play and transformed).
///
/// Note: because the buggy handler uses `.find()` over a `HashMap`,
/// the failure is non-deterministic per-run. We construct fresh
/// states in a loop and run the check multiple times to defeat the
/// flakiness — a correct implementation returns the same (negative)
/// answer every time, a buggy one flips roughly half the time.
#[test]
fn bug_99_003_daybreak_ranger_no_cross_contamination() {
    let registry = CardRegistry::with_all_cards();

    let mut any_cross_contaminated = false;
    for _ in 0..30 {
        let mut state = game_at_step(Step::PrecombatMain, P0);

        // ranger_a: front face (Daybreak Ranger, "deal 2 to flying").
        let ranger_a = named_creature(&mut state, &registry, "Daybreak Ranger", P0);

        // ranger_b: transformed (Nightfall Predator, "fight any creature").
        let ranger_b = named_creature(&mut state, &registry, "Daybreak Ranger", P0);
        mtg_engine::cards::helpers::apply_transform(&mut state, ranger_b, &registry);

        // Non-flying target on P1's side.
        let non_flying = ready_creature(&mut state, P1, 2, 2);
        state
            .get_object_mut(non_flying)
            .unwrap()
            .card_types = vec![CardType::Creature];

        let legal = engine::legal_actions(&state, &registry);
        let cross_contaminated = legal.actions.iter().any(|a| matches!(
            a,
            Action::ActivateAbility { object_id, ability_index: 0, targets, .. }
                if *object_id == ranger_a
                    && targets.iter().any(|t| matches!(t, Target::Object(id) if *id == non_flying))
        ));
        if cross_contaminated {
            any_cross_contaminated = true;
            break;
        }
    }

    assert!(
        !any_cross_contaminated,
        "Daybreak Ranger's front-face {{T}}: deal 2 to flying creature \
         ability should NOT offer a non-flying target, regardless of \
         whether the caster also controls a transformed Nightfall \
         Predator. Bug 99-003: is_valid_target uses `.find()` to grab \
         any Daybreak Ranger the caster controls and reads ITS \
         transform flag — so a transformed sibling forces \
         self_transformed=true and the front-face filter allows any \
         creature. Observed over 30 random HashMap iterations."
    );
}

/// Bug BE (`audits/AUDIT_BUGS.md)`: Garruk Relentless dies before
/// transforming when damage takes him from 3+ loyalty straight to 0.
/// The SBA loop processes planeswalker zero-loyalty BEFORE the
/// Garruk state trigger, so SBA 704.5i graveyards Garruk before his
/// CR 603.8 "transform when ≤2 loyalty" state trigger fires.
///
/// Oracle (Garruk Relentless): "When Garruk Relentless has two or
/// fewer loyalty counters on him, transform him."
///
/// Per CR 603.8 + 704.5, state-triggered abilities should preempt
/// zero-loyalty destruction: the trigger condition was true the
/// moment loyalty hit ≤2, so the trigger should have been queued
/// before the zero-loyalty SBA ran.
///
/// Failure mode: `sba.rs` moves planeswalkers with 0
/// loyalty to graveyard in the same iteration, BEFORE the state
/// trigger check at `sba.rs+`. If damage takes Garruk from
/// 3 → 0 directly, he dies without transforming.
///
/// We put Garruk on the battlefield with 3 loyalty counters, strip
/// them, and run SBA. The fix should leave Garruk on the
/// battlefield in his transformed (Garruk, the Veil-Cursed) form.
#[test]
fn bug_be_garruk_transforms_before_zero_loyalty_death() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Garruk Relentless with 3 loyalty on P0's battlefield.
    let garruk_card_id = registry.get_id_by_name("Garruk Relentless").unwrap();
    let garruk = state.create_object(garruk_card_id, P0, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(garruk).unwrap();
        obj.name = "Garruk Relentless".into();
        obj.counters.insert(CounterType::Loyalty, 3);
        obj.is_legendary = true;
    }

    // Drop his loyalty directly to 0 (as if a 3-damage burn spell
    // landed on him).
    state
        .get_object_mut(garruk)
        .unwrap()
        .counters
        .insert(CounterType::Loyalty, 0);

    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    // After SBA, Garruk should still be on the battlefield — the
    // state-triggered transform must preempt the zero-loyalty death.
    let zone = state.get_object(garruk).map(|o| o.zone);
    assert_eq!(
        zone,
        Some(Zone::Battlefield),
        "Garruk Relentless dropping from 3 loyalty straight to 0 \
         should trigger his CR 603.8 state-triggered transform (to \
         Garruk, the Veil-Cursed) BEFORE the zero-loyalty SBA \
         graveyards him. Bug BE: SBA processes zero-loyalty before \
         running state triggers, so Garruk dies first. zone = {zone:?}",
    );
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Delver of Secrets suppresses the "you may reveal" choice when the top
/// card is NOT an instant or sorcery. Per ruling: "You may reveal the card even
/// if it's not an instant or sorcery." The player should always get the choice.
#[test]
fn bug_delver_reveal_suppressed_for_non_instant_sorcery() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.active_player = P0;

    // Place Delver of Secrets (front face)
    let delver = named_creature(&mut state, &registry, "Delver of Secrets", P0);

    // Put a creature (not instant/sorcery) on top of library
    let _creature_card = {
        let card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
        let id = state.create_object(card_id, P0, Zone::Library, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Grizzly Bears".into();
        state.get_player_mut(P0).library_order.insert(0, id);
        id
    };

    // Fire upkeep trigger
    let behavior = registry.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &[], &registry);

    // Per the ruling, the player should STILL get a "you may reveal" choice
    // even though the top card is a creature (revealing it won't transform Delver,
    // but the player might want to reveal for information or other game reasons).
    // BUG: No choice is presented because the code only offers the choice when
    // the top card is an instant or sorcery.
    assert!(state.awaiting_action.is_some(),
        "Delver should present 'you may reveal' choice even for non-instant/sorcery top card");
}

/// Bug: After Thraben Sentry transforms to Thraben Militia (back face),
/// it retains Vigilance from the front face because obj.keywords isn't
/// updated during transform.
#[test]
fn bug_thraben_sentry_vigilance_retained_after_transform() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sentry = named_creature(&mut state, &registry, "Thraben Sentry", P0);

    // Set keywords on object to match front face (Vigilance)
    if let Some(obj) = state.get_object_mut(sentry) {
        obj.keywords = vec![Keyword::Vigilance];
    }

    // Transform to back face
    if let Some(obj) = state.get_object_mut(sentry) {
        obj.is_transformed = true;
        obj.name = "Thraben Militia".into();
        // BUG: keywords not updated — Vigilance persists
    }

    // Back face (Thraben Militia) should NOT have Vigilance
    let has_vigilance = state.has_keyword(sentry, Keyword::Vigilance, &registry);

    // BUG: has_keyword returns true because obj.keywords still contains Vigilance
    assert!(!has_vigilance,
        "Thraben Militia (back face) should NOT have Vigilance");
}

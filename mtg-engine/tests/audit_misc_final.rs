//! Failing tests for bugs documented in audits/AUDIT_BUGS.md.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file is the catch-all for the remaining engine bugs that
//! don't fit cleanly into one of the other audit_* family files.
//!
//! Bugs covered in this file:
//! - Bug 76-001: Skirsdag High Priest's activation labels render
//!   ObjectIds with Rust's `{:?}` debug format, leaking ObjectId(N)
//!   into the prompt.
//! - Bug 76-002: Ludevic's Test Subject stores hatchling counters in
//!   `obj.card_state` instead of using `state.add_counters` so
//!   proliferate / counter-removal can never see them.
//! - Bug 99-001: Gutter Grime's `is_token` check reads
//!   `state.get_object(dead_id)`, but the token has already been
//!   cleaned up by SBA 704.5d, so token deaths wrongly add slime
//!   counters and produce extra Ooze tokens.
//! - Bug AC: Unbreathing Horde reanimated from graveyard under-counts
//!   itself.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Bug 76-001 (audits/AUDIT_BUGS.md): Skirsdag High Priest's
/// `activated_abilities` formats the candidate creature ObjectIds
/// with Rust's `{:?}` debug format, so the LLM player sees labels
/// like `... (tap ObjectId(5) & ObjectId(12))`. ObjectIds are
/// internal handles — the model has no way to map them to creature
/// names.
///
/// Oracle (Skirsdag High Priest): "{T}, Tap two untapped creatures
/// you control: Create a 5/5 black Demon creature token with flying.
/// Activate this ability only if you control three or more creatures.
/// Morbid — ..."
///
/// Failure mode: `skirsdag_high_priest.rs:65-68` calls
/// `format!(... "tap {:?} & {:?} ...", candidates[i], candidates[j])`.
/// Since `candidates[i]` is an `ObjectId`, this renders the literal
/// `ObjectId(N)` substring. Compare with `format_combat_creature_list`
/// in `mtg-player/src/llm.rs:2411-2443`, which uses creature names
/// with `#1`/`#2` suffixes for collisions.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 76-001 is fixed.
#[test]
fn bug_76_001_skirsdag_high_priest_label_has_no_object_id_debug() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Skirsdag High Priest needs the morbid condition AND at least
    // three creatures total to enable the ability — so put one extra
    // creature on top of the High Priest and the two tap candidates.
    state.creature_died_this_turn = true;
    let _high_priest = named_creature(&mut state, &registry, "Skirsdag High Priest", P0);
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

/// Bug 76-002 (audits/AUDIT_BUGS.md): Ludevic's Test Subject stores
/// its hatchling counters in `obj.card_state` (abused as an `ObjectId`)
/// instead of using `state.add_counters`. Per CR 122 these are real
/// counters; proliferate (CR 701.24) and counter-removal effects
/// can't see them in the abused-card-state form.
///
/// Oracle (Ludevic's Test Subject): "{1}{U}: Put a hatchling counter
/// on this creature. Then if there are five or more hatchling
/// counters on it, remove all of them and transform it."
///
/// Failure mode: `ludevics_test_subject.rs:90-108` does
/// ```
/// obj.card_state.insert("hatchling_counters".into(), ObjectId(new_count as u64));
/// ```
/// instead of using the real counter pipeline (`state.add_counters` /
/// `state.get_counter_count`). Mikaeus the Lunarch in the same set
/// stores +1/+1 counters correctly via `CounterType::PlusOnePlusOne`
/// — that's the model to follow.
///
/// We assert the bug-fingerprint: after activating the hatchling
/// ability, `obj.card_state` should NOT contain a `hatchling_counters`
/// key — the counter must live in the real counter pipeline so other
/// effects can interact with it.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 76-002 is fixed.
#[test]
fn bug_76_002_ludevic_hatchling_counters_not_in_card_state() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ludevic = named_creature(&mut state, &registry, "Ludevic's Test Subject", P0);

    let ludevic_card_id = state.get_object(ludevic).unwrap().card_id;
    let behavior = registry.get(ludevic_card_id).unwrap();
    behavior.on_activate_ability(&mut state, ludevic, 0, &[], &registry);

    let obj = state.get_object(ludevic).unwrap();
    assert!(
        !obj.card_state.contains_key("hatchling_counters"),
        "Ludevic's Test Subject should not store hatchling counters in \
         obj.card_state — they're real CR 122 counters and need to live \
         in the engine's counter pipeline so proliferate and counter \
         removal can interact with them. Bug 76-002: card_state still \
         contains 'hatchling_counters' after activation. card_state = {:?}",
        obj.card_state,
    );
}

/// Bug 99-001 (audits/AUDIT_BUGS.md): Gutter Grime's `on_any_creature_dies`
/// checks `state.get_object(dead_id).is_token` to enforce the
/// "nontoken" oracle requirement. By the time this handler runs, SBA
/// 704.5d has already removed the dead token from `state.objects`, so
/// `state.get_object(dead_id)` returns None, `was_token` defaults to
/// false, and the handler proceeds to add a slime counter and create
/// an Ooze for *every* creature death — token or not.
///
/// Oracle (Gutter Grime): "Whenever a **nontoken** creature you
/// control dies, put a slime counter on this enchantment, then create
/// a green Ooze creature token..."
///
/// Failure mode: `gutter_grime.rs:43-81`. The dispatcher correctly
/// queues the trigger, the controller filter passes, then
/// `state.get_object(dead_id).map(|o| o.is_token).unwrap_or(false)`
/// returns `false` for an already-cleaned-up token. The fix needs the
/// dispatcher to thread `is_token` (or the dead card_id) into
/// `on_any_creature_dies` so the handler can check it from captured
/// state.
///
/// We simulate the post-cleanup state by passing a dead_id that's not
/// in `state.objects` and observing whether Gutter Grime's slime
/// counter was incremented.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 99-001 is fixed.
#[test]
fn bug_99_001_gutter_grime_does_not_count_token_deaths() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_creature(&mut state, &registry, "Gutter Grime", P0);
    let slime_before = state
        .get_object(grime)
        .unwrap()
        .counters
        .get(&CounterType::Slime)
        .copied()
        .unwrap_or(0);

    // The dead creature was a token that's already been cleaned up by
    // SBA 704.5d. Use an ObjectId that's not in state.objects.
    let dead_token_id = mtg_engine::ids::ObjectId(99999);
    assert!(
        state.get_object(dead_token_id).is_none(),
        "Test setup: dead_token_id should not be in state.objects"
    );

    let grime_card_id = state.get_object(grime).unwrap().card_id;
    let behavior = registry.get(grime_card_id).unwrap();
    behavior.on_any_creature_dies(
        &mut state,
        grime,
        dead_token_id,
        P0, // dead_controller (matches Gutter Grime's owner)
        &[],
        2, // dead_toughness
        &registry,
    );

    let slime_after = state
        .get_object(grime)
        .unwrap()
        .counters
        .get(&CounterType::Slime)
        .copied()
        .unwrap_or(0);

    assert_eq!(
        slime_after, slime_before,
        "Gutter Grime should NOT add a slime counter when a TOKEN \
         creature dies (oracle says 'nontoken'). Bug 99-001: the \
         is_token check reads state.get_object(dead_id), but tokens \
         are already cleaned up by SBA 704.5d at trigger-resolution \
         time, so the handler treats them as nontoken. Slime counters \
         {} -> {}",
        slime_before, slime_after,
    );
}

/// Bug AC (audits/AUDIT_BUGS.md): Unbreathing Horde under-counts when
/// reanimated from a graveyard. Per Scryfall ruling: "If Unbreathing
/// Horde enters from a graveyard, it counts itself for its enter-with-
/// counters ability."
///
/// Oracle (Unbreathing Horde): "This creature enters with a +1/+1
/// counter on it for each other Zombie you control and each Zombie
/// card in your graveyard."
///
/// "Enters with X counters" is a CR 614.1c replacement effect, so the
/// count is computed at entry timing — at which point the Horde is
/// still in the graveyard zone (it hasn't fully entered yet) and the
/// "Zombie cards in your graveyard" count includes the Horde itself.
///
/// Failure mode: `unbreathing_horde.rs:94-103` runs the
/// `add_zombie_counters` helper from the `on_enter_battlefield`
/// handler — i.e. AFTER the move to battlefield. By that point,
/// `count_zombies_in_graveyard` no longer sees the Horde (it's on
/// the battlefield), so the reanimated Horde misses one counter
/// compared to the cast path.
///
/// We put two other Zombies in P0's graveyard alongside the Horde,
/// then move the Horde to the battlefield (mirroring Unburial Rites
/// reanimation), then fire the ETB handler. The fix should give the
/// Horde three +1/+1 counters (2 other Zombies + the Horde itself);
/// the bug gives it only two.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug AC is fixed.
#[test]
fn bug_ac_unbreathing_horde_counts_itself_when_reanimated() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two other Zombie creature cards in P0's graveyard.
    let walking_corpse_id = registry.get_id_by_name("Walking Corpse").unwrap();
    let z1 = state.create_object(walking_corpse_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(z1).unwrap().name = "Walking Corpse (a)".into();
    let z2 = state.create_object(walking_corpse_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(z2).unwrap().name = "Walking Corpse (b)".into();

    // Unbreathing Horde sitting in P0's graveyard, ready to be reanimated.
    let horde_card_id = registry.get_id_by_name("Unbreathing Horde").unwrap();
    let horde = state.create_object(horde_card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(horde).unwrap().name = "Unbreathing Horde".into();

    // Reanimate: move the Horde to the battlefield and fire its ETB
    // handler (this mirrors what Unburial Rites does).
    state.move_object(horde, Zone::Battlefield, &registry);
    let behavior = registry.get(horde_card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, horde, &registry);

    let counters = state
        .get_object(horde)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);

    assert!(
        counters >= 3,
        "Reanimated Unbreathing Horde should enter with at least 3 \
         +1/+1 counters (2 other Zombies in graveyard + the Horde \
         counts itself per the Scryfall ruling). Bug AC: \
         on_enter_battlefield runs after the move, so the helper sees \
         only the 2 other Zombies in the graveyard and adds 2 counters. \
         Got: {}",
        counters,
    );
}

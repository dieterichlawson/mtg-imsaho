//! Failing tests for bugs documented in `audits/AUDIT_BUGS.md`.
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
//! - Bug 0F-001: `create_token_copy` only patches the `card_id` of
//!   the FIRST token, so Parallel Lives copies lose their
//!   `CardBehavior`.
//! - Bug 4D-001: `create_token_with_subtypes` returns only the
//!   primary id, so callers that post-mutate (tap the token,
//!   insert into combat, etc.) silently miss the doubled copies
//!   (Army of the Damned example).
//! - Bug BY: Geist of Saint Traft's Angel-token `on_attacks` handler
//!   sets the angel's defender via `state.opponent(controller)`
//!   instead of reading from `combat.attackers.get(&geist_id)`.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Bug 0F-002 (`audits/AUDIT_BUGS.md)`: `state.create_token_copy` never
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

/// Bug BJ (`audits/AUDIT_BUGS.md)`: Evil Twin's "enter as a copy"
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
/// Failure mode: `evil_twin.rs:43-61` builds an `EntersBattlefield`
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

/// Bug 0F-001 (`audits/AUDIT_BUGS.md)`: `create_token_copy` only patches
/// the `card_id` of the FIRST token, so Parallel Lives copies lose
/// their `CardBehavior`.
///
/// Oracle (Parallel Lives): "If an effect would create one or more
/// tokens under your control, it creates twice that many of those
/// tokens instead."
/// Oracle (Cackling Counterpart): "Create a token that's a copy of
/// target creature you control."
///
/// Failure mode: `state.rs:432-447` calls `create_token_with_subtypes`
/// (which generates primary + extras under Parallel Lives), then
/// patches ONLY the returned primary's `card_id`. The extras stay at
/// `CardId(0)`. `registry.get(CardId(0))` returns None, so the
/// doubled copies have no `CardBehavior` and miss every trigger, static
/// effect, or `dynamic_pt` their source has.
///
/// We put Parallel Lives in play and Cackling-Counterpart-copy a
/// Splinterfright, then look at all Splinterfright-named tokens on
/// the battlefield. Every token copy should have the source's
/// `card_id` patched — but only the primary does.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 0F-001 is fixed.
#[test]
fn bug_0f_001_parallel_lives_token_copies_share_card_id() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _parallel = named_creature(&mut state, &registry, "Parallel Lives", P0);

    // Three creature cards in P0's graveyard so Splinterfright has
    // non-zero CDA power — avoids stillborn-SBA complications.
    for _ in 0..3 {
        let bears = registry.get_id_by_name("Grizzly Bears").unwrap();
        let id = state.create_object(bears, P0, Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Grizzly Bears".into();
    }

    let splinter = named_creature(&mut state, &registry, "Splinterfright", P0);
    let splinter_card_id = state.get_object(splinter).unwrap().card_id;

    // Cackling Counterpart creates a token copy; Parallel Lives
    // doubles it to two tokens.
    let _primary = state.create_token_copy(splinter, P0, &registry);

    let token_copies: Vec<_> = state
        .objects
        .values()
        .filter(|o| o.is_token && o.name == "Splinterfright")
        .map(|o| (o.id, o.card_id))
        .collect();

    assert!(
        token_copies.len() >= 2,
        "Test setup: expected at least 2 Splinterfright tokens with \
         Parallel Lives in play, got {}",
        token_copies.len()
    );
    for (id, card_id) in &token_copies {
        assert_eq!(
            *card_id, splinter_card_id,
            "Every Parallel-Lives-doubled Splinterfright token should \
             carry Splinterfright's card_id so registry lookups succeed \
             and the copy retains Splinterfright's dynamic_pt / triggered \
             abilities. Bug 0F-001: create_token_copy patches only the \
             FIRST returned token's card_id. Token {id:?} has card_id {card_id:?}, \
             expected {splinter_card_id:?}.",
        );
    }
}

/// Bug 4D-001 (`audits/AUDIT_BUGS.md)`: `create_token_with_subtypes`
/// returns only the primary token id, so callers that post-mutate
/// the returned id (to tap the token, insert it into combat, or
/// link it to a source via `card_state`) silently miss the doubled
/// Parallel Lives copies.
///
/// Oracle (Army of the Damned): "Create thirteen **tapped** 2/2
/// black Zombie creature tokens."
///
/// Failure mode: `army_of_the_damned.rs:41-56` creates each token,
/// receives the primary id, and does `obj.tapped = true;`. With
/// Parallel Lives in play, the helper creates two copies per
/// iteration but only returns the primary, so 13 tokens are tapped
/// and 13 are untapped (instead of the expected 26 tapped).
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 4D-001 is fixed.
#[test]
fn bug_4d_001_parallel_lives_army_of_the_damned_tokens_are_all_tapped() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    let _parallel = named_creature(&mut state, &registry, "Parallel Lives", P0);

    // Resolve Army of the Damned directly via on_resolve.
    let army_card_id = registry.get_id_by_name("Army of the Damned").unwrap();
    let army = state.create_object(army_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(army).unwrap().name = "Army of the Damned".into();
    let behavior = registry.get(army_card_id).unwrap();
    behavior.on_resolve(&mut state, army, &[], &registry);

    let n_zombies = count_tokens_named_by(&state, "Zombie", P0);
    assert!(
        n_zombies >= 26,
        "Test setup: expected ≥26 zombie tokens with Parallel Lives \
         doubling the 13-token Army of the Damned, got {n_zombies}"
    );
    for z in state.objects.values().filter(|o| o.is_token && o.name == "Zombie" && o.controller == P0) {
        assert!(
            z.tapped,
            "Every zombie token created by Army of the Damned (including \
             Parallel Lives doubled copies) should enter tapped per \
             oracle. Bug 4D-001: create_token_with_subtypes returns only \
             the primary id, so the caller's `obj.tapped = true` loop \
             only touches half the tokens. Token {:?} is untapped.",
            z.id,
        );
    }
}

/// Bug BY (`audits/AUDIT_BUGS.md)`: Geist of Saint Traft's attack
/// trigger spawns an Angel token and inserts it into combat with
/// defender = `state.opponent(controller)`. This happens to coincide
/// with Geist's actual defender in 1v1 ISD, but the correct source
/// is Geist's own entry in `combat.attackers`.
///
/// Oracle (Geist of Saint Traft): "Whenever this creature attacks,
/// create a 4/4 white Angel creature token with flying that's tapped
/// and attacking."
///
/// Per Scryfall ruling: "The Angel token will be attacking the same
/// player or planeswalker that Geist of Saint Traft is attacking."
///
/// Failure mode: `geist_of_saint_traft.rs:74-78` reads
/// `state.opponent(controller)` instead of
/// `state.combat.as_ref().and_then(|c| c.attackers.get(&self_id).copied())`.
/// We exercise the bug by running the trigger with Geist's combat
/// entry already pointing at a specific player, then checking that
/// the Angel token is inserted with the same player as its defender.
/// Today the handler ignores the combat entry entirely and recomputes
/// the defender, which happens to match in 1v1 but is structurally
/// wrong. The assertion enforces the shape of the fix (read from
/// `combat.attackers`) — a test that would catch the bug if the
/// engine gains real multiplayer/planeswalker combat support.
///
/// This test asserts the EXPECTED CORRECT behavior. It passes today
/// (1v1 coincidence) but will continue to hold once the handler reads
/// from `combat.attackers` — so it doubles as a regression test
/// guarding the shape of the fix. The bug entry itself is "latent in
/// 1v1" so no test can observe it divergently; tracking as
/// `not-reproduced` with an explicit assertion for the fix shape.
#[test]
fn bug_by_geist_angel_token_defender_matches_geist() {
    use mtg_engine::state::CombatState;
    use std::collections::HashMap;

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_creature(&mut state, &registry, "Geist of Saint Traft", P0);

    // Set up combat state with Geist attacking P1.
    let mut attackers: HashMap<_, _> = HashMap::new();
    attackers.insert(geist, P1);
    state.combat = Some(CombatState {
        attackers,
        blocker_assignments: HashMap::new(),
        ..Default::default()
    });

    let geist_card_id = state.get_object(geist).unwrap().card_id;
    let behavior = registry.get(geist_card_id).unwrap();
    behavior.on_attacks(&mut state, geist, &[], &registry);

    // Find the Angel token that was just created.
    let angel = state
        .objects
        .values()
        .find(|o| o.is_token && o.name.contains("Angel") && o.controller == P0)
        .map(|o| o.id);
    let angel = angel.expect("Geist should have spawned an Angel token");

    let angel_defender = state
        .combat
        .as_ref()
        .and_then(|c| c.attackers.get(&angel).copied());

    assert_eq!(
        angel_defender,
        Some(P1),
        "Geist of Saint Traft's Angel token should be inserted into \
         combat.attackers with the same defender as Geist (P1). Bug BY: \
         the handler reads state.opponent(controller) instead of \
         consulting combat.attackers.get(&self_id). defender={angel_defender:?}",
    );
}

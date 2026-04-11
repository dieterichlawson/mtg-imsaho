//! Failing tests for bugs documented in audits/AUDIT_BUGS.md.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers the "Subtype filter family — instance vs registry
//! mismatch" group. The root cause is Bug BD (`setup_game` leaves
//! `obj.subtypes` empty), and the rest of the family is a catalog of
//! call sites that break because of it.
//!
//! Bugs covered in this file:
//! - Bug BD: setup_game doesn't copy card_data.subtypes into obj.subtypes
//! - Bug AX: ISD dual lands always enter tapped (instance-only subtype check)
//! - Bug AT: registry-only subtype filters miss tokens (Slayer of the Wicked)
//! - Bug AY: TargetFilter::HasSubtype is instance-only (Olivia Voldaren's
//!   {3}{B}{B}: Gain control of target Vampire can't see cast-from-hand Vampires)
//! - Bug AU: Moonmist's "instance subtypes non-empty" branch ignores the
//!   registry, so Olivia-bitten Humans can't be transformed by Moonmist

mod common;
use common::*;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::types::*;

/// Bug BD (audits/AUDIT_BUGS.md): `setup_game` doesn't initialize
/// `obj.subtypes` from registry data.
///
/// Oracle (Swamp): "Basic Land — Swamp" — the card's subtype is "Swamp"
/// per the registry (`mtg-engine/src/cards/swamp.rs`:
/// `subtypes: vec!["Swamp".into()]`).
///
/// Failure mode: `setup_game` (`engine.rs:3449-3462`) copies
/// `card_data.colors`, `name`, `keywords`, and `card_types` onto each
/// freshly-created library object, but does NOT copy `card_data.subtypes`.
/// So every normal card object starts the game with `obj.subtypes = []`,
/// which is the root cause of Bug AX (dual lands), Bug AT (token
/// subtype filters), and Bug AY (HasSubtype target filter).
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BD is fixed.
#[test]
fn bug_bd_setup_game_populates_obj_subtypes_from_registry() {
    let registry = CardRegistry::with_all_cards();
    let config = GameConfig {
        player_names: vec!["P0".into(), "P1".into()],
        decklists: vec![
            Decklist { entries: vec![("Swamp".into(), 60)] },
            Decklist { entries: vec![("Swamp".into(), 60)] },
        ],
        starting_life: 20,
        starting_player: Some(P0),
    };

    let state = engine::setup_game(&config, &registry);

    // Every Swamp object in the game — library or opening hand — should
    // have "Swamp" in its instance subtypes, matching the registry.
    let swamp_card_id = registry.get_id_by_name("Swamp").unwrap();
    let swamps: Vec<_> = state
        .objects
        .values()
        .filter(|o| o.card_id == swamp_card_id)
        .collect();
    assert!(
        !swamps.is_empty(),
        "Expected Swamps to exist after setup_game"
    );

    for swamp in &swamps {
        assert!(
            swamp.subtypes.iter().any(|s| s == "Swamp"),
            "Swamp object {:?} (zone {:?}) should have 'Swamp' in obj.subtypes \
             after setup_game, but obj.subtypes = {:?}. Bug BD: setup_game \
             doesn't initialize obj.subtypes from registry data.",
            swamp.id,
            swamp.zone,
            swamp.subtypes,
        );
    }
}

/// Bug AX (audits/AUDIT_BUGS.md): Four ISD dual lands always enter
/// tapped because their "unless you control a <basic>" check reads
/// only instance subtypes.
///
/// Oracle (Woodland Cemetery): "Woodland Cemetery enters tapped unless
/// you control a Swamp or a Forest."
///
/// Failure mode: `woodland_cemetery.rs:21-22` checks
/// `o.subtypes.iter().any(|s| s == "Swamp") || ...` against the instance
/// `obj.subtypes` vector. Because of Bug BD, basic Swamps on the
/// battlefield have `obj.subtypes = []` (their "Swamp" subtype lives in
/// the registry). The check returns false, so Woodland Cemetery enters
/// tapped even when a Swamp is in play. Hinterland Harbor is the
/// counter-example that got this right by also consulting the registry.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug AX is fixed.
#[test]
fn bug_ax_woodland_cemetery_untapped_with_swamp_in_play() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a basic Swamp on the battlefield. This mirrors what happens
    // during a real game — basic lands come in without anyone writing
    // their registry subtypes into obj.subtypes.
    let swamp_card_id = registry.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp).unwrap().card_types = vec![CardType::Land];

    // Cast Woodland Cemetery and resolve it onto the battlefield.
    let wc_card_id = registry.get_id_by_name("Woodland Cemetery").unwrap();
    let wc = state.create_object(wc_card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(wc).unwrap().name = "Woodland Cemetery".into();
    // Play it as a land by moving directly to the battlefield and firing
    // the ETB handler. This bypasses the cast-a-land path but still
    // exercises the card's on_enter_battlefield logic (which is where
    // the "enters tapped unless..." decision is made).
    state.move_object(wc, Zone::Battlefield, &registry);
    if let Some(behavior) = registry.get(wc_card_id) {
        behavior.on_enter_battlefield(&mut state, wc, &registry);
    }

    let wc_obj = state.get_object(wc).unwrap();
    assert!(
        !wc_obj.tapped,
        "Woodland Cemetery should enter UNTAPPED when controller already \
         has a Swamp in play. Bug AX: the card's check reads only \
         obj.subtypes, which is empty for basic lands (Bug BD)."
    );
}

/// Bug AT (audits/AUDIT_BUGS.md): Slayer of the Wicked's ETB destroy
/// filter is registry-only, so it can't see Vampire/Werewolf/Zombie
/// tokens.
///
/// Oracle (Slayer of the Wicked): "When Slayer of the Wicked enters the
/// battlefield, you may destroy target Vampire, Werewolf, or Zombie."
///
/// Failure mode: `slayer_of_the_wicked.rs:42-46` calls
/// `registry.card_data(o.card_id)` and checks the registry's subtypes.
/// Tokens are created with `card_id: CardId(0)` (a sentinel), so the
/// registry lookup returns None and the filter rejects every token.
/// Bloodline Keeper's 2/2 Vampire tokens, Endless Ranks zombies, and
/// Moan of the Unhallowed zombies are all untargetable.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug AT is fixed.
#[test]
fn bug_at_slayer_of_the_wicked_targets_vampire_token() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a 2/2 Vampire token like Bloodline Keeper makes. P1 controls
    // it so the only possible Slayer target is this token.
    let token = state.create_token_with_subtypes(
        "Vampire",
        P1,
        2,
        2,
        vec![Color::Black],
        vec![CardType::Creature],
        vec![],
        vec!["Vampire".into()],
        &registry,
    );
    if let Some(obj) = state.get_object_mut(token) {
        obj.summoning_sick = false;
    }
    // Sanity-check: the token's obj.subtypes carries the Vampire tag.
    assert!(
        state
            .get_object(token)
            .unwrap()
            .subtypes
            .iter()
            .any(|s| s == "Vampire"),
        "Test setup: Vampire token should have 'Vampire' in obj.subtypes"
    );

    // Put Slayer of the Wicked onto the battlefield and fire its ETB
    // handler. The bug is in `on_enter_battlefield` itself, not in
    // `is_valid_target`, so we have to actually run the ETB code path.
    let slayer_card_id = registry.get_id_by_name("Slayer of the Wicked").unwrap();
    let slayer = state.create_object(
        slayer_card_id,
        P0,
        Zone::Battlefield,
        Some(3),
        Some(2),
    );
    state.get_object_mut(slayer).unwrap().name = "Slayer of the Wicked".into();
    let behavior = registry.get(slayer_card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, slayer, &registry);

    // The ETB is "you may destroy target V/W/Z", so when there's at least
    // one valid target the card sets up an awaiting_action::ChooseTarget
    // with the token in the options list. Today the filter rejects the
    // token (registry-only) and the targets vector is empty, so
    // present_optional_target_choice silently early-returns and no
    // awaiting_action is set.
    let token_in_options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. },
            ..
        }) => options
            .iter()
            .any(|t| matches!(t, Target::Object(id) if *id == token)),
        _ => false,
    };

    assert!(
        token_in_options,
        "Slayer of the Wicked's ETB should offer the Vampire TOKEN as a \
         destroy target. Bug AT: the card-side filter only checks \
         registry.card_data(), which returns None for tokens, so the \
         targets list is empty and no choice is presented. \
         awaiting_action = {:?}",
        state.awaiting_action,
    );
}

/// Bug AY (audits/AUDIT_BUGS.md): `TargetFilter::HasSubtype` reads
/// `obj.subtypes` only, missing registry-subtype Vampires like Stromkirk
/// Noble or Bloodcrazed Neonate.
///
/// Oracle (Olivia Voldaren, second ability): "{3}{B}{B}: Gain control of
/// target Vampire."
///
/// Failure mode: `engine.rs:1808-1810` and `engine.rs:1944` both
/// implement `HasSubtype` as `obj.subtypes.contains(subtype)`. Regular
/// creatures cast from hand have `obj.subtypes = []` (Bug BD), so their
/// subtypes live only in the registry. `matches_ability_target_filter`
/// therefore rejects Stromkirk Noble et al as targets for Olivia's
/// gain-control ability — the action doesn't even appear in
/// `legal_actions`.
///
/// This test goes through `engine::legal_actions` so it exercises the
/// real activated-ability target-enumeration pipeline.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug AY is fixed.
#[test]
fn bug_ay_olivia_vampire_steal_can_target_registry_vampire() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Olivia is on P0's side, ready to activate.
    let olivia = named_creature(&mut state, &registry, "Olivia Voldaren", P0);

    // Stromkirk Noble (a registry-subtype Vampire) on P1's side.
    let noble = named_creature(&mut state, &registry, "Stromkirk Noble", P1);
    assert!(
        state.get_object(noble).unwrap().subtypes.is_empty(),
        "Test setup: cast-from-hand Vampires have obj.subtypes == [] (Bug BD)"
    );
    let noble_card_id = state.get_object(noble).unwrap().card_id;
    assert!(
        registry
            .card_data(noble_card_id)
            .unwrap()
            .subtypes
            .iter()
            .any(|s| s == "Vampire"),
        "Test setup: Stromkirk Noble's registry data should include Vampire"
    );

    // Pay {3}{B}{B} for Olivia's second ability up front by adding the
    // mana to the pool — that way `legal_actions` doesn't need to plan
    // an autotap.
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let legal = engine::legal_actions(&state, &registry);
    let can_steal_noble = legal.actions.iter().any(|a| matches!(
        a,
        Action::ActivateAbility { object_id, ability_index, targets, .. }
            if *object_id == olivia
                && *ability_index == 1
                && targets.iter().any(|t| matches!(t, Target::Object(id) if *id == noble))
    ));

    assert!(
        can_steal_noble,
        "Olivia Voldaren's {{3}}{{B}}{{B}} should be able to target Stromkirk \
         Noble (a registry-subtype Vampire). Bug AY: TargetFilter::HasSubtype \
         only reads obj.subtypes, which is [] for cast-from-hand creatures."
    );
}

/// Bug AU (audits/AUDIT_BUGS.md): Moonmist's Human filter takes the
/// "instance subtypes non-empty" branch when a creature's `obj.subtypes`
/// has been mutated (e.g. Olivia bit it) and then ignores the registry
/// completely — so a Gatstaf Shepherd that Olivia turned into a Vampire
/// fails to transform under Moonmist even though it's still a Human.
///
/// Oracle (Moonmist): "Transform all Humans. ..."
/// Oracle (Olivia Voldaren, first ability): "{1}{R}: Olivia Voldaren
/// deals 1 damage to another target creature. That creature becomes a
/// Vampire **in addition to its other types**. ..."
/// Oracle (Gatstaf Shepherd front face): "Human Werewolf".
///
/// Failure mode: `moonmist.rs:43-56` does
/// `if !o.subtypes.is_empty() { o.subtypes.iter().any(|s| s == "Human") }`
/// — when the instance vector is populated (e.g. by Olivia's bite, which
/// only pushes "Vampire") it stops looking at the registry. Olivia's
/// hook (`olivia_voldaren.rs:107-110`) `obj.subtypes.push("Vampire")`
/// onto an empty vector, so a bitten Gatstaf Shepherd has
/// `obj.subtypes = ["Vampire"]`. Moonmist sees no "Human", concludes
/// it isn't a Human, and skips the transform — even though oracle text
/// for both cards says the creature is still a Human in addition to
/// being a Vampire.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug AU is fixed.
#[test]
fn bug_au_moonmist_transforms_olivia_bitten_human_dfc() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Gatstaf Shepherd is Human Werewolf on its front face — exactly
    // the kind of creature Moonmist should transform.
    let shepherd = named_creature(&mut state, &registry, "Gatstaf Shepherd", P1);
    assert!(
        !state.get_object(shepherd).unwrap().is_transformed,
        "Test setup: Gatstaf Shepherd should start on its front (Human) face"
    );

    // Simulate the Olivia bite: push "Vampire" onto the empty
    // obj.subtypes vector. This is exactly what Olivia's
    // `on_activate_ability` does at olivia_voldaren.rs:108-110.
    state
        .get_object_mut(shepherd)
        .unwrap()
        .subtypes
        .push("Vampire".into());

    // Sanity: the Shepherd is still a Human per oracle text — its
    // registry subtypes still contain "Human". Moonmist's filter must
    // see this even though obj.subtypes is now non-empty.
    let shepherd_card_id = state.get_object(shepherd).unwrap().card_id;
    assert!(
        registry
            .card_data(shepherd_card_id)
            .unwrap()
            .subtypes
            .iter()
            .any(|s| s == "Human"),
        "Test setup: Gatstaf Shepherd's registry data should include Human"
    );

    // Resolve Moonmist directly. We don't need to put it on the stack;
    // calling on_resolve mirrors what stack resolution does.
    let moonmist_card_id = registry.get_id_by_name("Moonmist").unwrap();
    let moonmist = state.create_object(moonmist_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(moonmist).unwrap().name = "Moonmist".into();
    let behavior = registry.get(moonmist_card_id).unwrap();
    behavior.on_resolve(&mut state, moonmist, &[], &registry);

    let transformed = state.get_object(shepherd).unwrap().is_transformed;
    assert!(
        transformed,
        "Moonmist should transform an Olivia-bitten Gatstaf Shepherd \
         (still a Human per the 'in addition to its other types' clause). \
         Bug AU: Moonmist's filter takes the 'instance subtypes non-empty' \
         branch and only sees ['Vampire'], so it ignores the registry's \
         Human subtype."
    );
}

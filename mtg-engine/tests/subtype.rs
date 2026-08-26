//! Regressions for bugs documented in `audits/AUDIT_BUGS.md`. Each of these
//! failed when it was written and passes now; they stay to protect against
//! the bug coming back.
//!
//! This file covers the "Subtype filter family — instance vs registry
//! mismatch" group. The rest of the family is a catalog of call sites that
//! each hand-rolled their own answer to "does this object have subtype X?".
//!
//! The audit named Bug BD (`setup_game` leaves `obj.subtypes` empty) as the
//! family's root cause and proposed copying the registry's subtypes onto every
//! object. That was the wrong direction — see the first test below. The actual
//! root cause is that a permanent's characteristics had no single authoritative
//! reader, so every call site improvised; the fix is the characteristics layer
//! in `state.rs`, and Bug BD is resolved by removing the duplication rather
//! than extending it.
//!
//! Bugs covered in this file:
//! - Bug BD: re-decided — printed characteristics stay on the card's face
//! - Bug AX: ISD dual lands always enter tapped (instance-only subtype check)
//! - Bug AT: registry-only subtype filters miss tokens (Slayer of the Wicked)
//! - Bug AY: `TargetFilter::HasSubtype` is instance-only (Olivia Voldaren's
//!   {3}{B}{B}: Gain control of target Vampire can't see cast-from-hand Vampires)
//! - Bug AU: Moonmist's "instance subtypes non-empty" branch ignores the
//!   registry, so Olivia-bitten Humans can't be transformed by Moonmist
//! - Bug 31-003: Urgent Exorcism's "Spirit or enchantment" filter is
//!   registry-only, can't target Spirit tokens
//! - Bug 31-002: Avacynian Priest's "non-Human" filter reads front-face
//!   registry, refuses to target transformed werewolves
//! - Bug 31-004: Elder Cathar's Human-bonus check reads front-face
//!   registry, wrongly grants 2 counters to transformed ex-Humans
//! - Bug 99-002: Delver of Secrets hand-rolls its transform without
//!   `apply_transform`, leaving `obj.subtypes` stale
//! - Bug AO: `combat::get_subtypes` unions instance + front-face
//!   registry subtypes, so a transformed DFC falsely reports dropped
//!   front-face subtypes

mod common;
use common::*;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::types::*;

/// Bug BD, re-decided. The original audit saw that `obj.subtypes` was empty
/// on library objects and prescribed "make `setup_game` populate it from the
/// registry, the way it already populates `card_types`, `colors` and
/// `keywords`". That cure was worse than the disease: copying printed
/// characteristics onto every object gave a card TWO sources of truth that had
/// to be kept in sync by hand, and since `create_object` (tests, tokens,
/// reanimation) never did the copying, the same card had populated fields in a
/// real game and empty ones under test. Card code that read the raw fields
/// therefore worked in play and silently did nothing in its own tests — which
/// is how this one bug got found and re-reported roughly fifteen times.
///
/// The invariant is the opposite: printed characteristics live on the card's
/// active face and are read through the characteristics accessors; the
/// object-level vectors carry only what an effect granted at runtime. So
/// `setup_game` must NOT populate them — and the thing the original test
/// actually wanted, "a Swamp is discoverably a Swamp", has to hold anyway.
#[test]
fn setup_game_leaves_printed_characteristics_on_the_card_not_the_object() {
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

    let swamp_card_id = registry.get_id_by_name("Swamp").unwrap();
    let swamps: Vec<_> = state.objects.values()
        .filter(|o| o.card_id == swamp_card_id)
        .collect();
    assert!(!swamps.is_empty(), "expected Swamps to exist after setup_game");

    for swamp in &swamps {
        // The accessor answers correctly — that is the guarantee that matters.
        assert!(state.has_subtype(swamp.id, "Swamp", &registry),
            "a Swamp must be discoverably a Swamp through the characteristics \
             layer (object {:?}, zone {:?})", swamp.id, swamp.zone);
        assert!(state.has_card_type(swamp.id, CardType::Land, &registry),
            "a Swamp must be discoverably a Land (object {:?})", swamp.id);

        // And the raw vectors stay empty, because nothing granted anything.
        assert!(swamp.subtypes.is_empty() && swamp.card_types.is_empty()
                && swamp.colors.is_empty(),
            "printed characteristics must not be duplicated onto the object; \
             got subtypes={:?} card_types={:?} colors={:?}",
            swamp.subtypes, swamp.card_types, swamp.colors);
    }
}

/// The same object built by `create_object` must be indistinguishable from one
/// built by `setup_game` as far as the characteristics layer is concerned.
/// This is the property whose absence made the whole bug class invisible to
/// tests, so it gets pinned directly.
#[test]
fn test_built_and_game_built_objects_agree_on_characteristics() {
    let reg = CardRegistry::with_all_cards();

    let config = GameConfig {
        player_names: vec!["P0".into(), "P1".into()],
        decklists: vec![
            Decklist { entries: vec![("Avacyn's Pilgrim".into(), 60)] },
            Decklist { entries: vec![("Avacyn's Pilgrim".into(), 60)] },
        ],
        starting_life: 20,
        starting_player: Some(P0),
    };
    let game_state = engine::setup_game(&config, &reg);
    let game_obj = game_state.objects.values()
        .find(|o| o.card_id == reg.get_id_by_name("Avacyn's Pilgrim").unwrap())
        .expect("pilgrim should exist");

    let mut test_state = game_at_step(Step::PrecombatMain, P0);
    let test_obj = named_permanent(&mut test_state, &reg, "Avacyn's Pilgrim", P0);

    assert_eq!(
        game_state.subtypes_of(game_obj.id, &reg),
        test_state.subtypes_of(test_obj, &reg),
        "subtypes must not depend on which code path created the object");
    assert_eq!(
        game_state.card_types_of(game_obj.id, &reg),
        test_state.card_types_of(test_obj, &reg),
        "card types must not depend on which code path created the object");
    assert_eq!(
        game_state.colors_of(game_obj.id, &reg),
        test_state.colors_of(test_obj, &reg),
        "colors must not depend on which code path created the object");
}

/// Bug AX (`audits/AUDIT_BUGS.md)`: Four ISD dual lands always enter
/// tapped because their "unless you control a <basic>" check reads
/// only instance subtypes.
///
/// Oracle (Woodland Cemetery): "Woodland Cemetery enters tapped unless
/// you control a Swamp or a Forest."
///
/// Failure mode: `woodland_cemetery.rs` checks
/// `o.subtypes.iter().any(|s| s == "Swamp") || ...` against the instance
/// `obj.subtypes` vector. Because of Bug BD, basic Swamps on the
/// battlefield have `obj.subtypes = []` (their "Swamp" subtype lives in
/// the registry). The check returns false, so Woodland Cemetery enters
/// tapped even when a Swamp is in play. Hinterland Harbor is the
/// counter-example that got this right by also consulting the registry.
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
        behavior.on_enter_battlefield(&mut state, wc, &[], &registry);
    }

    let wc_obj = state.get_object(wc).unwrap();
    assert!(
        !wc_obj.tapped,
        "Woodland Cemetery should enter UNTAPPED when controller already \
         has a Swamp in play. Bug AX: the card's check reads only \
         obj.subtypes, which is empty for basic lands (Bug BD)."
    );
}

/// Bug AT (`audits/AUDIT_BUGS.md)`: Slayer of the Wicked's ETB destroy
/// filter is registry-only, so it can't see Vampire/Werewolf/Zombie
/// tokens.
///
/// Oracle (Slayer of the Wicked): "When Slayer of the Wicked enters the
/// battlefield, you may destroy target Vampire, Werewolf, or Zombie."
///
/// Failure mode: `slayer_of_the_wicked.rs` calls
/// `registry.card_data(o.card_id)` and checks the registry's subtypes.
/// Tokens are created with `card_id: CardId(0)` (a sentinel), so the
/// registry lookup returns None and the filter rejects every token.
/// Bloodline Keeper's 2/2 Vampire tokens, Endless Ranks zombies, and
/// Moan of the Unhallowed zombies are all untargetable.
/// Bug AT (`audits/AUDIT_BUGS.md`): Slayer of the Wicked's ETB targets
/// "Vampire, Werewolf, or Zombie", and a Vampire TOKEN qualifies. The card-side
/// filter asked `registry.card_data()`, which returns nothing for a token, so
/// tokens were invisible to it.
///
/// Checked through trigger collection: the target is chosen as the trigger goes
/// on the stack (CR 603.3d), so the ETB hook never enumerates anything.
#[test]
fn bug_at_slayer_of_the_wicked_targets_vampire_token() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // A Vampire token for the opponent — no registry face at all.
    let token = state.create_token_with_subtypes(
        "Vampire", P1, 2, 2,
        vec![Color::Black], vec![CardType::Creature], vec![],
        vec!["Vampire".into()],
        &registry,
    )[0];
    assert!(state.has_subtype(token, "Vampire", &registry), "test precondition");

    let slayer = named_permanent(&mut state, &registry, "Slayer of the Wicked", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: slayer,
        controller: P0,
    });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    // Sole legal target, so the engine locks it in rather than prompting.
    let locked = state.stack.iter().any(|e| matches!(e,
        mtg_engine::state::StackEntry::Trigger(t)
            if t.source.chosen_targets.contains(&Target::Object(token))));
    assert!(locked,
        "a Vampire token should be a legal target — the filter must ask the \
         accessor, which unions the token's own subtypes with any registry face");
}

/// Bug AY (`audits/AUDIT_BUGS.md)`: `TargetFilter::HasSubtype` reads
/// `obj.subtypes` only, missing registry-subtype Vampires like Stromkirk
/// Noble or Bloodcrazed Neonate.
///
/// Oracle (Olivia Voldaren, second ability): "{3}{B}{B}: Gain control of
/// target Vampire."
///
/// Failure mode: `engine.rs` and `engine.rs` both
/// implement `HasSubtype` as `obj.subtypes.contains(subtype)`. Regular
/// creatures cast from hand have `obj.subtypes = []` (Bug BD), so their
/// subtypes live only in the registry. `matches_ability_target_filter`
/// therefore rejects Stromkirk Noble et al as targets for Olivia's
/// gain-control ability — the action doesn't even appear in
/// `legal_actions`.
///
/// This test goes through `engine::legal_actions` so it exercises the
/// real activated-ability target-enumeration pipeline.
#[test]
fn bug_ay_olivia_vampire_steal_can_target_registry_vampire() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Olivia is on P0's side, ready to activate.
    let olivia = named_permanent(&mut state, &registry, "Olivia Voldaren", P0);

    // Stromkirk Noble (a registry-subtype Vampire) on P1's side.
    let noble = named_permanent(&mut state, &registry, "Stromkirk Noble", P1);
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

/// Bug AT — Vampiric Fury aspect (`audits/AUDIT_BUGS.md)`: Vampiric
/// Fury's "Vampire creatures you control get +2/+0" filter uses the
/// same registry-only pattern as Slayer of the Wicked. A Vampire
/// TOKEN (Bloodline Keeper's 2/2 Vampire) should receive the buff
/// but doesn't because `registry.card_data(CardId(0))` returns None.
///
/// Oracle (Vampiric Fury): "Vampire creatures you control get +2/+0
/// and gain first strike until end of turn."
#[test]
fn bug_at_vampiric_fury_buffs_vampire_token() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let token = state.create_token_with_subtypes(
        "Vampire", P0, 2, 2,
        vec![Color::Black],
        vec![CardType::Creature],
        vec![],
        vec!["Vampire".into()],
        &registry,
    )[0];
    if let Some(obj) = state.get_object_mut(token) {
        obj.summoning_sick = false;
    }

    let fury_card_id = registry.get_id_by_name("Vampiric Fury").unwrap();
    let fury = state.create_object(fury_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(fury).unwrap().name = "Vampiric Fury".into();
    let behavior = registry.get(fury_card_id).unwrap();
    behavior.on_resolve(&mut state, fury, &[], &registry);

    let eff_p = state.effective_power(token, &registry).unwrap_or(0);
    assert_eq!(
        eff_p, 4,
        "Vampiric Fury should give a Vampire TOKEN +2/+0 (2 base + 2 \
         buff = 4). Bug AT: the registry-only subtype filter calls \
         registry.card_data(CardId(0)) for tokens → returns None → \
         token is excluded from the Vampire filter. Got effective_power \
         = {eff_p}",
    );
}

/// Bug AU (`audits/AUDIT_BUGS.md)`: Moonmist's Human filter takes the
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
/// Failure mode: `moonmist.rs` does
/// `if !o.subtypes.is_empty() { o.subtypes.iter().any(|s| s == "Human") }`
/// — when the instance vector is populated (e.g. by Olivia's bite, which
/// only pushes "Vampire") it stops looking at the registry. Olivia's
/// hook (`olivia_voldaren.rs`) `obj.subtypes.push("Vampire")`
/// onto an empty vector, so a bitten Gatstaf Shepherd has
/// `obj.subtypes = ["Vampire"]`. Moonmist sees no "Human", concludes
/// it isn't a Human, and skips the transform — even though oracle text
/// for both cards says the creature is still a Human in addition to
/// being a Vampire.
#[test]
fn bug_au_moonmist_transforms_olivia_bitten_human_dfc() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Gatstaf Shepherd is Human Werewolf on its front face — exactly
    // the kind of creature Moonmist should transform.
    let shepherd = named_permanent(&mut state, &registry, "Gatstaf Shepherd", P1);
    assert!(
        !state.get_object(shepherd).unwrap().is_transformed,
        "Test setup: Gatstaf Shepherd should start on its front (Human) face"
    );

    // Simulate the Olivia bite: push "Vampire" onto the empty
    // obj.subtypes vector. This is exactly what Olivia's
    // `on_activate_ability` does at olivia_voldaren.rs.
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

/// Bug 31-003 (`audits/AUDIT_BUGS.md)`: Urgent Exorcism's
/// `is_valid_target` is registry-only, so Spirit tokens (Midnight
/// Haunting, Doomed Traveler, Mausoleum Guard, Geist-Honored Monk) are
/// untargetable.
///
/// Oracle (Urgent Exorcism): "Destroy target Spirit or enchantment."
///
/// Failure mode: `urgent_exorcism.rs` calls
/// `registry.card_data(obj.card_id)` and asks the registry whether the
/// object is a Spirit or enchantment. Tokens have `card_id: CardId(0)`,
/// so the registry lookup returns None, and the filter rejects every
/// token. The fix is the Bug AT pattern: also consult the instance
/// `obj.subtypes` and `obj.card_types`.
#[test]
fn bug_31_003_urgent_exorcism_targets_spirit_token() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a 1/1 Spirit token like Midnight Haunting makes.
    let token = state.create_token_with_subtypes(
        "Spirit",
        P1,
        1,
        1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![],
        vec!["Spirit".into()],
        &registry,
    )[0];
    if let Some(obj) = state.get_object_mut(token) {
        obj.summoning_sick = false;
    }
    assert!(
        state
            .get_object(token)
            .unwrap()
            .subtypes
            .iter()
            .any(|s| s == "Spirit"),
        "Test setup: Spirit token should have 'Spirit' in obj.subtypes"
    );

    let exorcism_card_id = registry.get_id_by_name("Urgent Exorcism").unwrap();
    let behavior = registry.get(exorcism_card_id).unwrap();
    let is_valid = behavior.is_valid_target(
        &state,
        P0,
        &Target::Object(token),
        &registry,
    );

    assert!(
        is_valid,
        "Urgent Exorcism should be able to target a Spirit TOKEN. \
         Bug 31-003: the card-side filter only checks \
         registry.card_data(), which returns None for tokens, so the \
         token is wrongly rejected."
    );
}

/// Bug 31-002 (`audits/AUDIT_BUGS.md)`: Avacynian Priest's `is_valid_target`
/// reads front-face registry subtypes, so it refuses to tap a transformed
/// werewolf (which is no longer a Human on its live face).
///
/// Oracle (Avacynian Priest): "{1}, {T}: Tap target non-Human creature."
/// Oracle (Tormented Pariah front face): "Human Warrior Werewolf".
/// Oracle (Rampaging Werewolf back face): "Werewolf" (no Human subtype).
///
/// Failure mode: `avacynian_priest.rs` checks
/// `registry.card_data(o.card_id).subtypes.contains("Human") || o.subtypes.contains("Human")`.
/// For a transformed Tormented Pariah, `obj.subtypes = ["Werewolf"]`
/// (back face — set by `apply_transform`) but `registry.card_data(...)`
/// returns the front face, which still says Human. So `is_human = true`
/// and the Pariah is rejected — even though its live face is non-Human.
#[test]
fn bug_31_002_avacynian_priest_can_tap_transformed_werewolf() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Tormented Pariah is Human Warrior Werewolf on its front face.
    // Transforming it gives the Rampaging Werewolf back face, which is
    // a plain Werewolf (no Human).
    let pariah = named_permanent(&mut state, &registry, "Tormented Pariah", P1);
    mtg_engine::cards::helpers::apply_transform(&mut state, pariah, &registry);
    assert!(
        state.get_object(pariah).unwrap().is_transformed,
        "Test setup: Tormented Pariah should be on its back (Rampaging Werewolf) face"
    );
    assert!(
        !state
            .get_object(pariah)
            .unwrap()
            .subtypes
            .iter()
            .any(|s| s == "Human"),
        "Test setup: Rampaging Werewolf back face should NOT have Human in obj.subtypes"
    );

    let priest_card_id = registry.get_id_by_name("Avacynian Priest").unwrap();
    let behavior = registry.get(priest_card_id).unwrap();
    let is_valid = behavior.is_valid_target(
        &state,
        P0,
        &Target::Object(pariah),
        &registry,
    );

    assert!(
        is_valid,
        "Avacynian Priest should be able to tap a transformed werewolf \
         (Rampaging Werewolf is non-Human on its live face). Bug 31-002: \
         the filter consults the front-face registry data and sees Human, \
         so it wrongly rejects every transformed ex-Human."
    );
}

/// Bug 31-004 (`audits/AUDIT_BUGS.md)`: Elder Cathar's "if Human, +2
/// counters instead" check reads front-face registry subtypes, so a
/// transformed werewolf (whose live face is non-Human) wrongly gets the
/// Human bonus.
///
/// Oracle (Elder Cathar): "When this creature dies, put a +1/+1 counter
/// on target creature you control. If that creature is a Human, put two
/// +1/+1 counters on it instead."
/// Oracle (Tormented Pariah / Rampaging Werewolf): see Bug 31-002 above.
///
/// Failure mode: `elder_cathar.rs` checks
/// `o.subtypes.iter().any(|s| s == "Human") || registry.card_data(o.card_id).subtypes.iter().any(|s| s == "Human")`.
/// For a transformed Tormented Pariah, `obj.subtypes = ["Werewolf"]`
/// (no Human) but the registry returns the front face which DOES have
/// Human, so `is_human = true` and the bonus fires. The transformed
/// werewolf gets two counters when oracle text says it should only get
/// one (because its live face is not a Human).
#[test]
fn bug_31_004_elder_cathar_no_bonus_on_transformed_werewolf() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has Elder Cathar (about to die) and a single transformed
    // Tormented Pariah (their only other creature, so Elder Cathar's
    // single-target auto-resolve path is the one that fires).
    let cathar = named_permanent(&mut state, &registry, "Elder Cathar", P0);
    let pariah = named_permanent(&mut state, &registry, "Tormented Pariah", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, pariah, &registry);
    assert!(
        state.get_object(pariah).unwrap().is_transformed,
        "Test setup: Tormented Pariah should be on its back (Rampaging Werewolf) face"
    );

    // Sanity-check: nothing else of P0's is on the battlefield to make
    // single-target the only path. (Cathar itself is excluded from the
    // target list because of the `o.id != object_id` filter at line 41.)
    let p0_creature_count = state
        .objects
        .values()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.controller == P0
                && o.power.is_some()
                && o.id != cathar
        })
        .count();
    assert_eq!(
        p0_creature_count, 1,
        "Test setup: only the transformed Pariah should be eligible for Cathar's counter"
    );

    let counters_before = state
        .get_object(pariah)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);

    // Fire Elder Cathar's death trigger directly. CR 603.3b: the target is
    // chosen when the trigger goes on the stack and handed to the resolution
    // handler, so pass it the way the engine does. (This used to be `&[]`,
    // because the card selected its own target at resolution — which is the
    // bug elder_cathar-01 reported.)
    let cathar_card_id = registry.get_id_by_name("Elder Cathar").unwrap();
    let behavior = registry.get(cathar_card_id).unwrap();
    behavior.on_dies(&mut state, cathar, &[Target::Object(pariah)], &registry);

    let counters_after = state
        .get_object(pariah)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);
    let added = counters_after - counters_before;

    assert_eq!(
        added, 1,
        "Elder Cathar's death should put exactly ONE +1/+1 counter on a \
         transformed werewolf (Rampaging Werewolf is non-Human on its live \
         face). Bug 31-004: the Human-bonus check reads the front-face \
         registry subtypes and sees Human, so it wrongly grants 2 counters."
    );
}

/// Bug 99-002 (`audits/AUDIT_BUGS.md)`: Delver of Secrets hand-rolls its
/// transform without going through `crate::cards::helpers::apply_transform`,
/// so `obj.subtypes` is stale after the transform fires.
///
/// Oracle (Delver of Secrets, front face): "Creature — Human Wizard. ...
/// At the beginning of your upkeep, look at the top card of your library.
/// You may reveal that card. If an instant or sorcery card is revealed
/// this way, transform Delver of Secrets."
/// Oracle (Insectile Aberration, back face): "Creature — Insect" (the
/// front-face Human and Wizard subtypes are gone).
///
/// Failure mode: `delver_of_secrets.rs` does
/// ```
/// obj.is_transformed = true;
/// obj.name = "Insectile Aberration".into();
/// ```
/// and stops there. The Bug-D fix migrated every werewolf to
/// `apply_transform` (which copies subtypes/keywords/name from the back
/// face onto the instance), but Delver was missed because it isn't a
/// werewolf. Once Bug BD lands and `obj.subtypes` actually carries the
/// front-face Human/Wizard tags, the post-transform instance keeps
/// reporting "Human Wizard" instead of "Insect" — Hamlet Captain's
/// Human anthem and Village Cannibals' "Human dies" trigger then both
/// fire on a creature whose live face is an Insect.
///
/// We simulate the post-Bug-BD state by writing the front-face subtypes
/// onto `obj.subtypes` directly, then driving Delver's "yes, reveal"
/// path with an instant on top of the library so the transform fires.
#[test]
fn bug_99_002_delver_transform_updates_obj_subtypes() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Upkeep, P0);

    let delver = named_permanent(&mut state, &registry, "Delver of Secrets", P0);

    // Put an instant on top of P0's library so the reveal triggers a transform.
    let bolt_card_id = registry.get_id_by_name("Lightning Bolt").unwrap();
    let bolt = state.create_object(bolt_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(bolt).unwrap().name = "Lightning Bolt".into();
    state.get_player_mut(P0).library_order.insert(0, bolt);

    // Drive Delver's "yes, reveal" path. It hand-rolls its own transform
    // rather than calling `apply_transform` — which used to matter, because
    // `apply_transform` also copied the new face's subtypes onto the object
    // and a hand-rolled flip left them stale. Nothing is copied any more, so
    // there is nothing to leave stale: the subtypes follow `is_transformed`.
    let delver_card_id = registry.get_id_by_name("Delver of Secrets").unwrap();
    let behavior = registry.get(delver_card_id).unwrap();
    behavior.on_yes_no_choice(&mut state, delver, true, &registry);

    assert!(state.get_object(delver).unwrap().is_transformed,
        "test setup: Delver should have transformed (instant on top of library)");

    // The guarantee, asserted through the characteristics layer rather than
    // through whichever field happens to back it.
    let back = behavior.back_face_data()
        .expect("Delver of Secrets should expose back_face_data()");
    for sub in &back.subtypes {
        assert!(state.has_subtype(delver, sub, &registry),
            "after transforming to Insectile Aberration the creature must have \
             back-face subtype {sub:?}; subtypes_of = {:?}",
            state.subtypes_of(delver, &registry));
    }
    assert!(!state.has_subtype(delver, "Wizard", &registry),
        "'Wizard' is on the front face only and must not survive the flip; \
         subtypes_of = {:?}", state.subtypes_of(delver, &registry));
}

/// Bug AO (`audits/AUDIT_BUGS.md)`: `combat::get_subtypes` is not
/// face-aware for transformed DFCs. It unions the instance
/// `obj.subtypes` (set by `apply_transform` to the back-face
/// subtypes) with `registry.card_data(obj.card_id).subtypes` (always
/// the front face). For a DFC that DROPS a subtype on its back face,
/// the dropped subtype falsely persists in the union.
///
/// Oracle (Cloistered Youth front face): "Human" subtype.
/// Oracle (Unholy Fiend back face): "Horror" subtype (Human dropped).
///
/// Failure mode: `combat.rs` calls `registry.card_data()`
/// which always returns front-face data. For a transformed Cloistered
/// Youth, `obj.subtypes = ["Horror"]` (back face) and
/// `registry.card_data().subtypes = ["Human"]` (front face). The
/// union is `["Horror", "Human"]` — but the live face is Horror only.
///
/// We transform Cloistered Youth via `apply_transform`, then call
/// `combat::get_subtypes` and assert "Human" is NOT in the result.
#[test]
fn bug_ao_get_subtypes_excludes_dropped_front_face_subtype() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cloistered Youth → transform → Unholy Fiend (Horror, no Human).
    let youth = named_permanent(&mut state, &registry, "Cloistered Youth", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, youth, &registry);
    assert!(
        state.get_object(youth).unwrap().is_transformed,
        "Test setup: Cloistered Youth should be transformed to Unholy Fiend"
    );
    // Sanity: the live face is Horror. (Read through the accessor — the back
    // face's subtypes are no longer mirrored onto `obj.subtypes`.)
    assert!(state.has_subtype(youth, "Horror", &registry),
        "Test setup: Unholy Fiend should have the Horror subtype");

    let subtypes = mtg_engine::combat::get_subtypes(&state, youth, &registry);
    assert!(
        !subtypes.iter().any(|s| s == "Human"),
        "combat::get_subtypes for a transformed Unholy Fiend should NOT \
         include 'Human' (the front-face-only subtype). Bug AO: \
         get_subtypes unions instance subtypes with the front-face \
         registry data, so 'Human' persists even though the live back \
         face is Horror only. subtypes = {subtypes:?}",
    );
}

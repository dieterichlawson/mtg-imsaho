//! Failing tests for bugs documented in `audits/AUDIT_BUGS.md`.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers the "Auto-pick — engine makes choices that should
//! belong to the player" family. The pattern is: an oracle effect
//! should ask the player a question (which creature to exile, which
//! basic land to tutor, which legend to keep) but the implementation
//! takes a deterministic shortcut.
//!
//! Bugs covered in this file:
//! - Bug D: Moorland Haunt's activation cost auto-picks the first
//!   creature in the controller's graveyard to exile
//! - Bug P: Caravan Vigil auto-picks the first basic land in library
//!   order, so a splash deck can't tutor the splash colour
//! - Bug 76-003: Traveler's Amulet auto-picks the first basic land
//!   in library order (Bug P sibling)
//! - Bug E: Nevermore reads the opponent's hand and auto-picks a
//!   name (should ask the controller for a string choice)
//! - Bug F: `AdditionalCost::ExileCreaturesFromGraveyard` auto-picks
//!   the highest-power creature instead of asking the player
//! - Bug U: Kessig Wolf Run's `{X}{R}{G}, {T}` activated ability
//!   has no in-engine X enumeration — a single generic entry is
//!   offered with X determined at apply time from the mana pool
//! - Bug W: The legend rule SBA auto-picks which legend to keep
//!   (CR 704.5j says the player chooses)

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Bug D (`audits/AUDIT_BUGS.md)`: Moorland Haunt's `{W}{U}, {T}, Exile
/// a creature from your graveyard` cost auto-picks the first matching
/// creature card in the controller's graveyard. The player should be
/// the one choosing which creature to exile.
///
/// Oracle (Moorland Haunt): "{W}{U}, {T}, Exile a creature card from
/// your graveyard: Create a 1/1 white Spirit creature token with
/// flying."
///
/// Failure mode: `moorland_haunt.rs:85-90` does
/// `state.objects_in_zone(Graveyard, controller).iter().filter(...).map(o.id).next()`
/// — it picks the first matching creature deterministically and
/// exiles it without ever asking the player. With multiple creatures
/// in the graveyard the player has no way to preserve a graveyard
/// creature they care about (e.g., a Boneyard Wurm fueling
/// Splinterfright's CDA).
///
/// We put two distinct creatures into P0's graveyard, fire Moorland
/// Haunt's activation directly, and assert that NO creature has been
/// exiled yet — the fix should set up an awaiting choice instead.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug D is fixed.
#[test]
fn bug_d_moorland_haunt_does_not_auto_pick_creature_to_exile() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two distinct creature cards in P0's graveyard.
    let bears_a = {
        let card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
        let id = state.create_object(card_id, P0, Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Grizzly Bears (a)".into();
        id
    };
    let bears_b = {
        let card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
        let id = state.create_object(card_id, P0, Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Grizzly Bears (b)".into();
        id
    };

    // Moorland Haunt on P0's side.
    let haunt = named_creature(&mut state, &registry, "Moorland Haunt", P0);

    // Fire Moorland Haunt's activation directly.
    let haunt_card_id = state.get_object(haunt).unwrap().card_id;
    let behavior = registry.get(haunt_card_id).unwrap();
    behavior.on_activate_ability(&mut state, haunt, 1, &[], &registry);

    // After firing, neither creature should have been moved to exile
    // yet — the fix should pause for a player choice.
    let a_zone = state.get_object(bears_a).map(|o| o.zone);
    let b_zone = state.get_object(bears_b).map(|o| o.zone);
    let either_exiled = a_zone == Some(Zone::Exile) || b_zone == Some(Zone::Exile);

    assert!(
        !either_exiled,
        "Moorland Haunt's activation cost should NOT auto-pick a \
         graveyard creature to exile when multiple are eligible — the \
         player chooses. Bug D: the handler picks the first matching \
         creature with `iter().filter(...).next()`. zones: a={a_zone:?}, b={b_zone:?}",
    );
}

/// Bug P (`audits/AUDIT_BUGS.md)`: Caravan Vigil's "search your library
/// for a basic land card" auto-picks the first basic land in
/// `library_order`, so a splash deck cannot specifically tutor the
/// splash colour.
///
/// Oracle (Caravan Vigil): "Search your library for a basic land card,
/// reveal it, put it into your hand, then shuffle. ..."
///
/// Failure mode: `caravan_vigil.rs:39-50` calls
/// `library_order.iter().find(|&id| <is basic land>)`. The first
/// matching basic in library order is the one that lands in hand,
/// regardless of which colour the player wants. A B/R deck splashing
/// one green card cannot specifically tutor a Forest with this
/// implementation.
///
/// We put a Forest and a Swamp in P0's library (in that order) and
/// resolve Caravan Vigil. The bug auto-picks the Forest. The fix
/// should pause for a player choice instead, so neither basic land
/// has moved to hand yet when `on_resolve` returns.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug P is fixed.
#[test]
fn bug_p_caravan_vigil_does_not_auto_pick_basic_land() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Forest then Swamp in P0's library, in that order.
    let forest_card_id = registry.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    state.get_player_mut(P0).library_order.push(forest);

    let swamp_card_id = registry.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_player_mut(P0).library_order.push(swamp);

    // Resolve Caravan Vigil directly. (No creatures died — morbid path
    // is irrelevant; we just want to test the auto-pick.)
    let vigil_card_id = registry.get_id_by_name("Caravan Vigil").unwrap();
    let vigil = state.create_object(vigil_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(vigil).unwrap().name = "Caravan Vigil".into();
    let behavior = registry.get(vigil_card_id).unwrap();
    behavior.on_resolve(&mut state, vigil, &[], &registry);

    // Neither basic should have ended up in hand without a player
    // choice. The fix should set an awaiting_action of "choose a basic
    // land type to tutor".
    let forest_zone = state.get_object(forest).map(|o| o.zone);
    let swamp_zone = state.get_object(swamp).map(|o| o.zone);
    let either_in_hand = forest_zone == Some(Zone::Hand) || swamp_zone == Some(Zone::Hand);

    assert!(
        !either_in_hand,
        "Caravan Vigil should not auto-tutor a basic land — the player \
         chooses which basic to fetch (this matters for splash decks). \
         Bug P: the implementation walks library_order and picks the \
         first matching basic. zones: forest={forest_zone:?}, swamp={swamp_zone:?}",
    );
}

/// Bug W (`audits/AUDIT_BUGS.md)`: The legend-rule SBA in `sba.rs:248-269`
/// auto-picks which copy to keep when a player controls two legendary
/// permanents with the same name. CR 704.5j explicitly says the player
/// chooses.
///
/// Oracle (CR 704.5j): "If a player controls two or more legendary
/// permanents with the same name, that player chooses one of them, and
/// the rest are put into their owners' graveyards."
///
/// Failure mode: `sba.rs:251-269` builds a `legend_groups` `HashMap` and
/// for each group of size > 1 keeps `ids[0]` and moves the rest to
/// graveyard. There's no `awaiting_action` prompt and no player input
/// — the kept permanent is whichever `HashMap` iteration surfaced
/// first (which is also nondeterministic across runs).
///
/// We put two Olivia Voldarens on P0's battlefield (e.g. by
/// reanimating one onto the existing one) and run SBA. The bug
/// silently drops one of them; the fix should pause for a player
/// choice with both Olivias still on the battlefield.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug W is fixed.
#[test]
fn bug_w_legend_rule_pauses_for_player_choice() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia_a = named_creature(&mut state, &registry, "Olivia Voldaren", P0);
    let olivia_b = named_creature(&mut state, &registry, "Olivia Voldaren", P0);
    assert!(
        state.get_object(olivia_a).unwrap().is_legendary
            && state.get_object(olivia_b).unwrap().is_legendary,
        "Test setup: both Olivias should be flagged is_legendary"
    );

    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    let a_zone = state.get_object(olivia_a).map(|o| o.zone);
    let b_zone = state.get_object(olivia_b).map(|o| o.zone);
    let both_on_battlefield = a_zone == Some(Zone::Battlefield) && b_zone == Some(Zone::Battlefield);

    assert!(
        both_on_battlefield,
        "The legend rule should pause for a player choice (CR 704.5j: \
         'that player chooses one of them'). Both Olivia Voldarens \
         should still be on the battlefield until the player picks one \
         to keep. Bug W: SBA auto-picks ids[0] and silently moves the \
         other to graveyard. zones: a={a_zone:?}, b={b_zone:?}",
    );
}

/// Bug 76-003 (`audits/AUDIT_BUGS.md)`: Traveler's Amulet's
/// `{1}, Sacrifice: search your library for a basic land card` auto-
/// picks the first matching basic in `library_order`, with exactly
/// the same shape as Bug P (Caravan Vigil).
///
/// Oracle (Traveler's Amulet): "{1}, Sacrifice this artifact: Search
/// your library for a basic land card, reveal it, put it into your
/// hand, then shuffle."
///
/// Failure mode: `travelers_amulet.rs:57-65` does
/// `library_order.iter().find(|&&id| <is basic land>)` and auto-
/// picks the first match. A B/R deck splashing green cannot
/// specifically tutor a Forest if Mountain or Swamp comes first in
/// library order.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 76-003 is fixed.
#[test]
fn bug_76_003_travelers_amulet_does_not_auto_pick_basic_land() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Forest then Swamp in P0's library (in that order).
    let forest_card_id = registry.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    state.get_player_mut(P0).library_order.push(forest);

    let swamp_card_id = registry.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_player_mut(P0).library_order.push(swamp);

    // Traveler's Amulet on P0's battlefield.
    let amulet = named_creature(&mut state, &registry, "Traveler's Amulet", P0);
    let amulet_card_id = state.get_object(amulet).unwrap().card_id;

    // Fire the activation handler directly — the artifact is normally
    // already sacrificed by the engine before on_activate_ability.
    let behavior = registry.get(amulet_card_id).unwrap();
    behavior.on_activate_ability(&mut state, amulet, 0, &[], &registry);

    let forest_zone = state.get_object(forest).map(|o| o.zone);
    let swamp_zone = state.get_object(swamp).map(|o| o.zone);
    let either_in_hand = forest_zone == Some(Zone::Hand) || swamp_zone == Some(Zone::Hand);

    assert!(
        !either_in_hand,
        "Traveler's Amulet should not auto-tutor a basic land — the \
         player chooses (Bug P sibling). Bug 76-003: the implementation \
         walks library_order and picks the first matching basic. zones: \
         forest={forest_zone:?}, swamp={swamp_zone:?}",
    );
}

/// Bug E (`audits/AUDIT_BUGS.md)`: Nevermore auto-picks a name by
/// reading the opponent's hand. Doubly wrong: (1) Nevermore's name
/// choice is independent of any player's hand, and (2) the
/// implementation leaks opponent hand information into the decision.
///
/// Oracle (Nevermore): "As this enchantment enters, choose a nonland
/// card name. Spells with the chosen name can't be cast."
///
/// Failure mode: `nevermore.rs:42-70` iterates
/// `state.objects.values()` filtering for `Zone::Hand` + opponent
/// owner, extracts the first nonland name, and stores it as the
/// blocked name. No player choice is presented.
///
/// We put a specific card in the opponent's hand and check that
/// Nevermore's `PreventCastingNamed` effect matches that card's name
/// — the bug's fingerprint. After the fix the handler should pause
/// for a `ChooseCardName`-style choice (which doesn't exist yet, so
/// we instead assert that the name wasn't leaked from opp's hand).
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug E is fixed.
#[test]
fn bug_e_nevermore_does_not_read_opponent_hand() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a very specific card in P1's hand.
    let bolt_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    let leaked = state.create_object(bolt_card_id, P1, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(leaked).unwrap().name = "Grizzly Bears".into();

    // Nevermore enters the battlefield for P0 and fires its ETB.
    let nevermore_card_id = registry.get_id_by_name("Nevermore").unwrap();
    let nevermore = state.create_object(nevermore_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(nevermore).unwrap().name = "Nevermore".into();
    let behavior = registry.get(nevermore_card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, nevermore, &[], &registry);

    // If the bug fires, Nevermore's instance continuous effects
    // include a `PreventCastingNamed { name: "Grizzly Bears" }` entry
    // — the name was leaked from P1's hand. The fix should either
    // leave it unset (pending a choice) or not equal the hand card.
    let leaked_name_chosen = state
        .get_object(nevermore)
        .and_then(|o| o.instance_continuous_effects.clone())
        .is_some_and(|effects| {
            effects.iter().any(|e| matches!(
                e,
                ContinuousEffect::PreventCastingNamed { name } if name == "Grizzly Bears"
            ))
        });

    assert!(
        !leaked_name_chosen,
        "Nevermore must not auto-pick a name by reading the opponent's \
         hand. Bug E: the handler iterates state.objects for \
         Zone::Hand + opponent and picks the first nonland — both an \
         information leak and an auto-pick."
    );
}

/// Bug F (`audits/AUDIT_BUGS.md)`: `AdditionalCost::ExileCreaturesFromGraveyard`
/// auto-picked the highest-power creature at apply time, so the
/// player never chose between exile candidates (Stitched Drake /
/// Makeshift Mauler / Skaab Goliath / Skaab Ruinator).
///
/// Oracle (Stitched Drake): "As an additional cost to cast this
/// spell, exile a creature card from your graveyard."
///
/// Fix: `legal_actions` emits a single `CastSpell` per target with
/// `exile_ids = vec![]`; on submission the engine sets up a
/// `ChooseExileFromGraveyard` resolution prompt listing every eligible
/// creature in the caster's graveyard. The player picks via
/// `ResolvedChoice::ChosenExileSet`. Subset enumeration in
/// `legal_actions` would scale `C(graveyard_size, n)` per target, so
/// the structured prompt replaces it.
///
/// We put two distinct-power creatures in P0's graveyard, cast
/// Stitched Drake, and assert the engine sets up the prompt with
/// BOTH creatures as options.
#[test]
fn bug_f_stitched_drake_enumerates_exile_choices() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two distinct creature cards in P0's graveyard: a high-power
    // option (Grizzly Bears, power 2) and a low-power option (Doomed
    // Traveler, power 1).
    let bears = registry.get_id_by_name("Grizzly Bears").unwrap();
    let bears_obj = state.create_object(bears, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(bears_obj).unwrap().name = "Grizzly Bears".into();

    let traveler = registry.get_id_by_name("Doomed Traveler").unwrap();
    let traveler_obj = state.create_object(traveler, P0, Zone::Graveyard, Some(1), Some(1));
    state.get_object_mut(traveler_obj).unwrap().name = "Doomed Traveler".into();

    // Stitched Drake in hand with mana paid.
    let drake = castable_spell(&mut state, &registry, "Stitched Drake", P0);

    // legal_actions should emit exactly one CastSpell for the Drake —
    // the subset enumeration is gone.
    let legal = engine::legal_actions(&state, &registry);
    let drake_casts: Vec<_> = legal.actions.iter()
        .filter(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == drake))
        .collect();
    assert_eq!(drake_casts.len(), 1,
        "Stitched Drake should emit exactly one CastSpell entry; the \
         exile choice now goes through ChooseExileFromGraveyard. Got {} entries.",
        drake_casts.len());

    // Submit the cast — the engine should set up a prompt offering
    // BOTH creatures, not auto-pick one.
    let cast = drake_casts[0].clone();
    let post = engine::submit_action(&state, &cast, &registry);

    match post.awaiting_action.as_ref() {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseExileFromGraveyard { options, min, max, .. },
            ..
        }) => {
            assert!(
                options.contains(&bears_obj) && options.contains(&traveler_obj),
                "Both Bears and Traveler should appear as exile options — \
                 Bug F was the engine auto-picking max-power. options={options:?}"
            );
            assert_eq!((*min, *max), (1, 1),
                "Stitched Drake exiles exactly 1 creature; got min={min} max={max}");
        }
        other => panic!(
            "Submitting Cast Stitched Drake should set up a \
             ChooseExileFromGraveyard prompt, got {other:?}"
        ),
    }
}

/// Bug O (`audits/AUDIT_BUGS.md)`: Memory's Journey's `GraveyardCard`
/// enumerator returns cards from ALL graveyards — even when a
/// specific player was chosen as the first target. Per oracle the
/// graveyard cards must come from THAT player's graveyard.
///
/// Oracle (Memory's Journey): "Target player shuffles up to three
/// target cards from **their** graveyard into their library."
///
/// Failure mode: `engine.rs::valid_targets_for_req`'s `GraveyardCard`
/// arm iterates `state.objects.values().filter(zone == Graveyard)`
/// without constraining to the chosen player. The `TwoTargets`
/// Cartesian product then emits combos where the player target is
/// P1 but the graveyard card belongs to P0, which is wrong.
///
/// We put one card in each player's graveyard and cast Memory's
/// Journey as P0 targeting P1. The `legal_actions` list should contain
/// no `CastSpell` where the player target is P1 and a graveyard card
/// target is owned by P0.
///
/// We exercise the bug at resolution time: given `targets = [Player(P1),
/// Object(p0_card)]` (which the buggy enumeration would allow), the
/// fix should either reject the cast at legal-action time OR discard
/// the mismatched graveyard card when resolving. The safest assertion
/// is that after resolving Memory's Journey with the wrong-player
/// graveyard target, the P0-owned card is NOT moved to any library —
/// the oracle restriction must prevent it.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug O is fixed.
#[test]
fn bug_o_memorys_journey_only_shuffles_targeted_players_graveyard() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // A creature card in each player's graveyard.
    let bears = registry.get_id_by_name("Grizzly Bears").unwrap();
    let p0_card = state.create_object(bears, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(p0_card).unwrap().name = "Grizzly Bears (P0)".into();
    let p1_card = state.create_object(bears, P1, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(p1_card).unwrap().name = "Grizzly Bears (P1)".into();

    // Resolve Memory's Journey directly with the bad target pair.
    let journey_card_id = registry.get_id_by_name("Memory's Journey").unwrap();
    let journey = state.create_object(journey_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(journey).unwrap().name = "Memory's Journey".into();
    let behavior = registry.get(journey_card_id).unwrap();
    behavior.on_resolve(
        &mut state,
        journey,
        &[Target::Player(P1), Target::Object(p0_card)],
        &registry,
    );

    let p0_zone = state.get_object(p0_card).map(|o| o.zone);
    assert_eq!(
        p0_zone,
        Some(Zone::Graveyard),
        "Memory's Journey targeting P1 should NOT be allowed to pull a \
         card from P0's graveyard. Bug O: the on_resolve loop shuffles \
         every Target::Object in the target list regardless of which \
         player owns the card. P0's Grizzly Bears ended up in zone {p0_zone:?}.",
    );
}

/// Bug U (`audits/AUDIT_BUGS.md)`: Kessig Wolf Run's
/// `{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample
/// until end of turn` is offered as a single legal-actions entry.
/// The effective X is determined at apply time by reading whatever
/// happens to be in the mana pool — there's no enumeration of
/// possible X values and no separate X prompt.
///
/// Oracle (Kessig Wolf Run): "{X}{R}{G}, {T}: Target creature gets
/// +X/+0 and gains trample until end of turn."
///
/// Failure mode: `engine.rs:588-599` (`legal_actions` for activated
/// abilities) and `engine.rs:2288-2305` (apply path) treat the X-cost
/// ability as a single entry. There's no `X` dimension in
/// `Action::ActivateAbility`, and the only way to set X is to
/// pre-tap lands into the pool. The player can't express "I want
/// X=2" through the action list.
///
/// With multiple attainable X values (enough mana for X=0, 1, 2),
/// there should be multiple distinct `ActivateAbility` entries (one
/// per X) — or equivalently a follow-up X-selection prompt. Today
/// there's only one entry.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug U is fixed.
#[test]
fn bug_u_kessig_wolf_run_enumerates_x_choices() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Kessig Wolf Run into play along with a target creature and
    // enough mana to support at least two different X values.
    let run_card_id = registry.get_id_by_name("Kessig Wolf Run").unwrap();
    let run = state.create_object(run_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(run).unwrap().name = "Kessig Wolf Run".into();
    state.get_object_mut(run).unwrap().card_types = vec![CardType::Land];

    let _target = ready_creature(&mut state, P0, 2, 2);

    // Pre-fill the pool with {2}{R}{G} — enough to pump with X=0, 1,
    // or 2.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    // The ability should appear as a single ActivateAbility entry.
    let legal = engine::legal_actions(&state, &registry);
    let kessig_action = legal
        .actions
        .iter()
        .find(|a| matches!(
            a,
            Action::ActivateAbility { object_id, ability_index, .. }
                if *object_id == run && *ability_index == 1
        ))
        .expect("Kessig Wolf Run ability should be available");

    // Activate it — the engine should present a followup ChooseXFunding prompt.
    let new_state = engine::submit_action(&state, kessig_action, &registry);

    // After activation, a ChooseXFunding prompt should be pending with max_x >= 2.
    let has_x_prompt = match &new_state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice { choice, .. }) => {
            matches!(choice, mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, .. } if options.max_x >= 2)
        }
        _ => false,
    };

    assert!(
        has_x_prompt,
        "Kessig Wolf Run's X-cost activated ability should present a \
         followup ChooseXFunding prompt so the player can pick X=0, X=1, \
         or X=2. Bug U: X was auto-determined from the mana pool with \
         no player input. awaiting_action: {:?}",
        new_state.awaiting_action,
    );
}

/// Bug BF (`audits/AUDIT_BUGS.md)`: Traveler's Amulet's
/// `on_activate_ability` searches the library for a basic land but
/// does NOT shuffle the library afterwards. Per oracle: "{1},
/// Sacrifice this artifact: Search your library for a basic land card,
/// reveal it, put it into your hand, **then shuffle**."
///
/// Other tutors (Caravan Vigil, Ghost Quarter, Bitterheart Witch,
/// Garruk) DO call `library_order.shuffle(&mut rng)`. Traveler's
/// Amulet was missed — the comment at line 83 says "no-op".
///
/// We fill the library with 20 non-land cards plus one Forest,
/// activate the Amulet, and check that the remaining 20 cards are
/// not in the original insertion order. With 20 cards the probability
/// of a random shuffle reproducing the same order is 1/20! ≈ 4e-19.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BF is fixed.
#[test]
fn bug_bf_travelers_amulet_shuffles_library_after_search() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Build a library of 20 Grizzly Bears + 1 Forest (at the end).
    let bears_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..20 {
        let id = state.create_object(bears_card_id, P0, Zone::Library, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Grizzly Bears".into();
        state.get_player_mut(P0).library_order.push(id);
    }
    let forest_card_id = registry.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    state.get_player_mut(P0).library_order.push(forest);

    // Snapshot the order AFTER inserting (the Forest will be removed
    // by the search, so we only snapshot the 20 Bears).
    let order_before: Vec<_> = state
        .get_player(P0)
        .library_order
        .iter()
        .filter(|&&id| id != forest)
        .copied()
        .collect();

    // Traveler's Amulet on the battlefield. Fire activation directly.
    let amulet = named_creature(&mut state, &registry, "Traveler's Amulet", P0);
    let amulet_card_id = state.get_object(amulet).unwrap().card_id;
    let behavior = registry.get(amulet_card_id).unwrap();
    behavior.on_activate_ability(&mut state, amulet, 0, &[], &registry);

    // Forest should have moved to hand.
    assert_eq!(
        state.get_object(forest).map(|o| o.zone),
        Some(Zone::Hand),
        "Test setup: Forest should have been tutored into hand"
    );

    // The remaining library should be shuffled (different order from
    // the original insertion order).
    let order_after: Vec<_> = state
        .get_player(P0)
        .library_order
        .clone();

    assert_ne!(
        order_before, order_after,
        "Traveler's Amulet should shuffle the library after searching. \
         Bug BF: the comment at line 83 says 'no-op' and no shuffle \
         call is made. With 20 cards, the probability of a random \
         shuffle matching the original order is ~4e-19."
    );
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Falkenrath Noble auto-targets the opponent for life drain.
/// Oracle: "target player loses 1 life and you gain 1 life"
/// "Target player" means the controller chooses which player to target,
/// including potentially themselves. The code does state.opponent(controller)
/// without presenting a choice.
#[test]
fn bug_falkenrath_noble_auto_targets_opponent() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Falkenrath Noble for P0
    let _noble = named_creature(&mut state, &registry, "Falkenrath Noble", P0);

    // Place a creature for P1 and kill it to trigger Noble
    let victim = ready_creature(&mut state, P1, 1, 1);
    mtg_engine::destruction::sacrifice(&mut state, victim, &registry);
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    // Process the death trigger
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // The Noble's trigger should present a choice of which player to target.
    // If it auto-targeted the opponent, P1's life will already be 19 and
    // there won't be an AwaitingAction for player choice.
    //
    // BUG: Noble auto-selects opponent — no choice is presented
    let p1_life = state.get_player(P1).life;
    let awaiting = state.awaiting_action.is_some();

    // Either there should be an awaiting action (choice pending)
    // OR if it already resolved, it should have targeted correctly.
    // The bug is that it resolves WITHOUT presenting a choice.
    assert!(awaiting || p1_life == 20,
        "Noble should either present target choice (awaiting_action) or not have auto-drained yet. P1 life: {p1_life}, awaiting: {awaiting}");
}

/// Bug: Ghost Quarter auto-searches instead of presenting "may" choice.
/// Oracle: "Its controller may search their library for a basic land card"
/// The "may" means the land's controller can decline to search.
/// The code auto-finds the first basic land without presenting a choice.
#[test]
fn bug_ghost_quarter_may_search_is_mandatory() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Ghost Quarter for P0
    let gq = named_creature(&mut state, &registry, "Ghost Quarter", P0);

    // Place a target land for P1
    let target_land = {
        let card_id = registry.get_id_by_name("Forest").unwrap();
        let id = state.create_object(card_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        id
    };

    // Put a Plains in P1's library
    let _plains_id = {
        let card_id = registry.get_id_by_name("Plains").unwrap();
        let id = state.create_object(card_id, P1, Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_player_mut(P1).library_order.push(id);
        id
    };

    let bf_count_before = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P1 && o.name == "Plains")
        .count();

    // Activate Ghost Quarter
    let behavior = registry.get(state.get_object(gq).unwrap().card_id).unwrap();
    state.move_object(gq, Zone::Graveyard, &registry);
    behavior.on_activate_ability(&mut state, gq, 1, &[Target::Object(target_land)], &registry);

    let bf_count_after = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P1 && o.name == "Plains")
        .count();

    // BUG: The Plains was auto-placed on the battlefield without P1 getting
    // a choice to decline the search. In a real game, P1 might want to
    // decline (e.g., to avoid a shuffle, or in specific strategic scenarios).
    // After the ability resolves, there should be an AwaitingAction for P1's
    // "may search" choice, OR the search should not have happened yet.
    let awaiting = state.awaiting_action.is_some();

    // The bug is that the search happened automatically
    assert!(awaiting || bf_count_after == bf_count_before,
        "Ghost Quarter should present 'may search' choice, not auto-search. Plains placed: {}",
        bf_count_after - bf_count_before);
}

/// Bug: Thraben Sentry auto-transforms when a creature you control dies,
/// without presenting the "you may" choice from the oracle text.
#[test]
fn bug_thraben_sentry_auto_transforms_without_choice() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Thraben Sentry
    let sentry = named_creature(&mut state, &registry, "Thraben Sentry", P0);
    assert!(!state.get_object(sentry).unwrap().is_transformed);

    // Place and kill another creature
    let victim = ready_creature(&mut state, P0, 1, 1);
    mtg_engine::destruction::sacrifice(&mut state, victim, &registry);
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    // Process triggers
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // The "you may transform" should present a choice, not auto-transform
    let is_transformed = state.get_object(sentry).unwrap().is_transformed;
    let has_choice = state.awaiting_action.is_some();

    // BUG: Auto-transforms without presenting "you may" choice
    assert!(!is_transformed || has_choice,
        "Sentry should either present 'you may' choice or not auto-transform. Transformed: {is_transformed}, Choice pending: {has_choice}");
}

/// When casting Harvest Pyre, the engine must let the player choose
/// WHICH cards to exile — not just a count. The original auto-pick
/// behavior was replaced by a structured `ChooseExileFromGraveyard`
/// prompt that lists every eligible graveyard card and lets the player
/// pick any subset (0 ≤ k ≤ `graveyard_size`).
///
/// Previous incarnation of this test asserted the engine enumerated
/// `C(gy,k)` expanded `CastSpell` actions. That was a transitional
/// fix; the final fix is the structured prompt, which avoids the
/// combinatorial explosion entirely (important for the LLM player:
/// 2^N actions flood the action list for an N-card graveyard).
#[test]
fn bug_harvest_pyre_auto_selects_exile() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put several different cards in P0's graveyard
    let mut gy_ids = Vec::new();
    for name in ["Grizzly Bears", "Lightning Bolt", "Giant Growth"] {
        let card_id = registry.get_id_by_name(name).unwrap();
        let id = state.create_object(card_id, P0, Zone::Graveyard, None, None);
        state.get_object_mut(id).unwrap().name = name.into();
        gy_ids.push(id);
    }

    let target = ready_creature(&mut state, P1, 5, 5);

    add_mana_for(&mut state, &registry, "Harvest Pyre", P0);
    let pyre = spell_in_hand(&mut state, &registry, "Harvest Pyre", P0);

    // Engine should emit exactly ONE CastSpell action per target —
    // no subset enumeration.
    let legal = engine::legal_actions(&state, &registry);
    let pyre_actions: Vec<_> = legal.actions.iter().filter(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == pyre)
    }).collect();
    assert_eq!(pyre_actions.len(), 1,
        "Harvest Pyre should emit exactly one CastSpell; exile choice goes through \
         ChooseExileFromGraveyard prompt. Got {} entries.", pyre_actions.len());

    // Submitting the cast should set up a ChooseExileFromGraveyard
    // prompt offering all three graveyard cards, with min=0 max=3.
    let cast = Action::CastSpell {
        object_id: pyre,
        targets: vec![Target::Object(target)],
        sacrifice: None, exile_count: None, exile_ids: vec![],
        alternative_cost: None, tap_plan: vec![],
    };
    let post = engine::submit_action(&state, &cast, &registry);

    match post.awaiting_action.as_ref() {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseExileFromGraveyard { options, min, max, .. },
            ..
        }) => {
            assert_eq!(*min, 0, "Harvest Pyre allows X=0");
            assert_eq!(*max, 3, "Harvest Pyre max X = graveyard size (3)");
            for id in &gy_ids {
                assert!(options.contains(id),
                    "all P0 graveyard cards should appear as options, missing {id:?}");
            }
        }
        other => panic!(
            "Casting Harvest Pyre should set up ChooseExileFromGraveyard, got {other:?}"
        ),
    }

    // Harvest Pyre should still be in hand while the prompt is pending.
    assert_eq!(post.get_object(pyre).map(|o| o.zone), Some(Zone::Hand));
    assert!(post.stack.is_empty());
}

/// Bug: Mentor of the Meek says "you may pay {1}" to draw a card when
/// a creature with power 2 or less enters. The code auto-pays without
/// presenting a choice.
#[test]
fn bug_mentor_of_the_meek_auto_pays() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Mentor of the Meek
    let mentor = named_creature(&mut state, &registry, "Mentor of the Meek", P0);

    // Add {1} mana so the pay choice is available
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Give P0 some library cards to draw from
    for _ in 0..3 {
        let card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
        let id = state.create_object(card_id, P0, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P0).library_order.push(id);
    }

    let hand_before = state.objects_in_zone(Zone::Hand, P0).len();

    // Place a small creature and directly call the trigger handler
    let small = ready_creature(&mut state, P0, 1, 1);
    let behavior = registry.get(state.get_object(mentor).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, mentor, small, P0, &registry);

    let hand_after = state.objects_in_zone(Zone::Hand, P0).len();
    let has_choice = state.awaiting_action.is_some();

    // "you may pay {1}" should present a choice, not auto-draw
    // BUG: Auto-draws without presenting the pay choice
    assert!(has_choice || hand_after == hand_before,
        "Mentor should present 'you may pay' choice. Drew {} cards without asking.",
        hand_after.saturating_sub(hand_before));
}

/// Bug: Skirsdag High Priest's ability costs "tap two untapped creatures
/// you control" but the engine auto-selects which creatures to tap.
#[test]
fn bug_skirsdag_high_priest_auto_selects_tap_targets() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Skirsdag High Priest and 3 other creatures
    let priest = named_creature(&mut state, &registry, "Skirsdag High Priest", P0);
    let _c1 = ready_creature(&mut state, P0, 1, 1);
    let _c2 = ready_creature(&mut state, P0, 2, 2);
    let _c3 = ready_creature(&mut state, P0, 3, 3);

    // Morbid must be active
    state.creature_died_this_turn = true;

    // Get legal actions
    let legal = engine::legal_actions(&state, &registry);
    let priest_abilities: Vec<_> = legal.actions.iter().filter(|a| {
        matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == priest)
    }).collect();

    // With 3 untapped creatures (besides the priest who taps itself),
    // there should be C(3,2) = 3 different tap combinations.
    // If there's only 1, the engine auto-selected.
    // BUG: Only 1 action (auto-selected tap targets)
    assert!(priest_abilities.len() >= 3,
        "Should have 3+ tap combinations for 3 creatures, got {}",
        priest_abilities.len());
}

/// Bug: Brain Weevil says "Target player discards two cards" but only
/// forces 1 discard when the player has 3+ cards (missing `on_discard_choice` chain).
#[test]
fn bug_brain_weevil_incomplete_discard() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P1 three cards in hand
    for name in ["Grizzly Bears", "Lightning Bolt", "Giant Growth"] {
        spell_in_hand(&mut state, &registry, name, P1);
    }
    let hand_before = state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_before, 3);

    // Place Brain Weevil and activate its sacrifice ability targeting P1
    let weevil = named_creature(&mut state, &registry, "Brain Weevil", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let behavior = registry.get(state.get_object(weevil).unwrap().card_id).unwrap();
    mtg_engine::destruction::sacrifice(&mut state, weevil, &registry);
    behavior.on_activate_ability(&mut state, weevil, 0, &[Target::Player(P1)], &registry);

    // Resolve any pending choices (first discard)
    while state.awaiting_action.is_some() {
        if let Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseCardFromHand { cards, .. }, ..
        }) = &state.awaiting_action {
            if let Some(&first) = cards.first() {
                let action = Action::ResolveChoice {
                    choice: mtg_engine::actions::ResolvedChoice::ChosenCard(first),
                };
                state = engine::submit_action(&state, &action, &registry);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let hand_after = state.objects_in_zone(Zone::Hand, P1).len();
    // BUG: Only 1 card discarded instead of 2
    assert_eq!(hand_after, 1,
        "Brain Weevil should force 2 discards. Hand: {hand_before} -> {hand_after} (expected 3 -> 1)");
}

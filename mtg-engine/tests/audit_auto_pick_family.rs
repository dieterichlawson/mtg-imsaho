//! Failing tests for bugs documented in audits/AUDIT_BUGS.md.
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

/// Bug D (audits/AUDIT_BUGS.md): Moorland Haunt's `{W}{U}, {T}, Exile
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
         creature with `iter().filter(...).next()`. zones: a={:?}, b={:?}",
        a_zone, b_zone,
    );
}

/// Bug P (audits/AUDIT_BUGS.md): Caravan Vigil's "search your library
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
/// has moved to hand yet when on_resolve returns.
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
         first matching basic. zones: forest={:?}, swamp={:?}",
        forest_zone, swamp_zone,
    );
}

/// Bug W (audits/AUDIT_BUGS.md): The legend-rule SBA in `sba.rs:248-269`
/// auto-picks which copy to keep when a player controls two legendary
/// permanents with the same name. CR 704.5j explicitly says the player
/// chooses.
///
/// Oracle (CR 704.5j): "If a player controls two or more legendary
/// permanents with the same name, that player chooses one of them, and
/// the rest are put into their owners' graveyards."
///
/// Failure mode: `sba.rs:251-269` builds a `legend_groups` HashMap and
/// for each group of size > 1 keeps `ids[0]` and moves the rest to
/// graveyard. There's no `awaiting_action` prompt and no player input
/// — the kept permanent is whichever HashMap iteration surfaced
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
         other to graveyard. zones: a={:?}, b={:?}",
        a_zone, b_zone,
    );
}

/// Bug 76-003 (audits/AUDIT_BUGS.md): Traveler's Amulet's
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
         forest={:?}, swamp={:?}",
        forest_zone, swamp_zone,
    );
}

/// Bug E (audits/AUDIT_BUGS.md): Nevermore auto-picks a name by
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
    behavior.on_enter_battlefield(&mut state, nevermore, &registry);

    // If the bug fires, Nevermore's instance continuous effects
    // include a `PreventCastingNamed { name: "Grizzly Bears" }` entry
    // — the name was leaked from P1's hand. The fix should either
    // leave it unset (pending a choice) or not equal the hand card.
    let leaked_name_chosen = state
        .get_object(nevermore)
        .and_then(|o| o.instance_continuous_effects.as_ref().cloned())
        .map(|effects| {
            effects.iter().any(|e| matches!(
                e,
                ContinuousEffect::PreventCastingNamed { name } if name == "Grizzly Bears"
            ))
        })
        .unwrap_or(false);

    assert!(
        !leaked_name_chosen,
        "Nevermore must not auto-pick a name by reading the opponent's \
         hand. Bug E: the handler iterates state.objects for \
         Zone::Hand + opponent and picks the first nonland — both an \
         information leak and an auto-pick."
    );
}

/// Bug F (audits/AUDIT_BUGS.md): `AdditionalCost::ExileCreaturesFromGraveyard`
/// auto-picks the highest-power creature at apply time. For Corpse
/// Lunge that's accidentally fine, but for Stitched Drake / Makeshift
/// Mauler / Skaab Goliath / Skaab Ruinator the player should choose.
///
/// Oracle (Stitched Drake): "As an additional cost to cast this
/// spell, exile a creature card from your graveyard."
///
/// Failure mode: the additional-cost handler in
/// `engine.rs:2119-2152` sorts candidates by base `power` and
/// `.take(n)`s the top entries. No player choice, no enumeration of
/// alternative exile subsets in `legal_actions`.
///
/// We put two distinct-power creatures in P0's graveyard and cast
/// Stitched Drake. `legal_actions` should enumerate one CastSpell
/// entry per exile choice (mirroring how SacrificeCost is handled
/// post-Bug-C fix). Today there's only one entry and the exile is
/// determined at apply time.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug F is fixed.
#[test]
fn bug_f_stitched_drake_enumerates_exile_choices() {
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

    let legal = engine::legal_actions(&state, &registry);
    let distinct_drake_casts = legal
        .actions
        .iter()
        .filter(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == drake))
        .count();

    assert!(
        distinct_drake_casts >= 2,
        "Stitched Drake should expose one CastSpell entry per distinct \
         exile choice (exile Bears, exile Traveler) — the player \
         chooses, not the engine. Bug F: the additional-cost handler \
         auto-picks the highest-power candidate at apply time and emits \
         a single CastSpell entry. Got {} entries.",
        distinct_drake_casts,
    );
}

/// Bug O (audits/AUDIT_BUGS.md): Memory's Journey's `GraveyardCard`
/// enumerator returns cards from ALL graveyards — even when a
/// specific player was chosen as the first target. Per oracle the
/// graveyard cards must come from THAT player's graveyard.
///
/// Oracle (Memory's Journey): "Target player shuffles up to three
/// target cards from **their** graveyard into their library."
///
/// Failure mode: `engine.rs::valid_targets_for_req`'s `GraveyardCard`
/// arm iterates `state.objects.values().filter(zone == Graveyard)`
/// without constraining to the chosen player. The TwoTargets
/// Cartesian product then emits combos where the player target is
/// P1 but the graveyard card belongs to P0, which is wrong.
///
/// We put one card in each player's graveyard and cast Memory's
/// Journey as P0 targeting P1. The legal_actions list should contain
/// no CastSpell where the player target is P1 and a graveyard card
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
         player owns the card. P0's Grizzly Bears ended up in zone {:?}.",
        p0_zone,
    );
}

/// Bug U (audits/AUDIT_BUGS.md): Kessig Wolf Run's
/// `{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample
/// until end of turn` is offered as a single legal-actions entry.
/// The effective X is determined at apply time by reading whatever
/// happens to be in the mana pool — there's no enumeration of
/// possible X values and no separate X prompt.
///
/// Oracle (Kessig Wolf Run): "{X}{R}{G}, {T}: Target creature gets
/// +X/+0 and gains trample until end of turn."
///
/// Failure mode: `engine.rs:588-599` (legal_actions for activated
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

    // Activate it — the engine should present a followup ChooseXValue prompt.
    let mut new_state = engine::submit_action(&state, kessig_action, &registry);

    // After activation, a ChooseXValue prompt should be pending with max_x >= 2.
    let has_x_prompt = match &new_state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice { choice, .. }) => {
            matches!(choice, mtg_engine::state::ResolutionChoiceKind::ChooseXValue { max_x, .. } if *max_x >= 2)
        }
        _ => false,
    };

    assert!(
        has_x_prompt,
        "Kessig Wolf Run's X-cost activated ability should present a \
         followup ChooseXValue prompt so the player can pick X=0, X=1, \
         or X=2. Bug U: X was auto-determined from the mana pool with \
         no player input. awaiting_action: {:?}",
        new_state.awaiting_action,
    );
}

/// Bug BF (audits/AUDIT_BUGS.md): Traveler's Amulet's
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

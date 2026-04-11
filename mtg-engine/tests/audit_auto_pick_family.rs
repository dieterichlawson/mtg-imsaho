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
//! - Bug W: The legend rule SBA auto-picks which legend to keep
//!   (CR 704.5j says the player chooses)

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
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

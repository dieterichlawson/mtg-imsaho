//! CR 608.2b: as a spell resolves, its targets are checked again. A spell whose
//! targets have *all* become illegal is countered by game rules — it never
//! resolves at all, which is not the same as resolving and finding nothing to
//! do. A spell that keeps at least one legal target resolves and does as much
//! as it can.
//!
//! The difference is only observable in what the resolution emits, so these
//! tests watch for `GameEvent::SpellResolved` rather than for the effect. A
//! spell that "resolved but did nothing because the target was gone" emits it;
//! a fizzled one must not.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::events::GameEvent;
use mtg_engine::types::*;

/// Did the last resolution report the spell as resolved?
fn resolved(state: &mtg_engine::state::GameState, spell: ObjectId) -> bool {
    state.events.iter().any(|e| matches!(e, GameEvent::SpellResolved { object } if *object == spell))
}

/// Cast `spell` at `target`, move the target to `moved_to`, then resolve.
/// Events are cleared before resolution so only what the resolution emitted is
/// visible.
fn cast_then_move_target(
    state: &mut mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    spell: ObjectId,
    target: ObjectId,
    moved_to: Zone,
) {
    *state = cast_onto_stack(state, reg, spell, vec![Target::Object(target)]);
    state.move_object(target, moved_to, reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(state, reg);
}

// ---------------------------------------------------------------------------
// A spell whose only target is gone is countered by game rules
// ---------------------------------------------------------------------------

/// Every kind of single-target spell in the set, each losing its one target
/// before resolution. What varies is the card and where the target went; what
/// must not vary is that the spell is countered rather than resolved.
///
/// One card per effect shape on purpose: a damage spell, an exile spell with a
/// rider, a destroy spell, a pump spell, and an Aura — an Aura in particular
/// has somewhere else it could wrongly end up (CR 704.5m).
#[test]
fn a_spell_whose_only_target_became_illegal_is_countered_by_game_rules() {
    // (spell, target's power/toughness, where the target goes)
    const CASES: &[(&str, i32, i32, Zone)] = &[
        ("Lightning Bolt", 3, 3, Zone::Graveyard),
        ("Swords to Plowshares", 5, 5, Zone::Exile),
        ("Doom Blade", 5, 5, Zone::Exile),
        ("Giant Growth", 2, 2, Zone::Graveyard),
        ("Pacifism", 2, 2, Zone::Graveyard),
    ];

    for &(spell_name, power, toughness, moved_to) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let creature = ready_creature(&mut state, P1, power, toughness);
        let spell = castable_spell(&mut state, &reg, spell_name, P0);
        cast_then_move_target(&mut state, &reg, spell, creature, moved_to);

        assert!(!resolved(&state, spell),
            "{spell_name} lost its only target, so it is countered by game rules, \
             not resolved (CR 608.2b)");
        assert_eq!(state.get_object(spell).unwrap().zone, Zone::Graveyard,
            "{spell_name} still goes to its owner's graveyard — including an Aura, \
             which must not reach the battlefield with nothing to enchant");
        assert_eq!(state.get_object(creature).unwrap().zone, moved_to,
            "{spell_name} did nothing to the creature it could no longer see");
        assert_eq!(state.get_player(P1).life, 20,
            "{spell_name} did not touch the target's controller either — no \
             redirected damage, and no Swords life gain");
    }
}

/// Scryfall ruling (2011-09-22) for Maw of the Mire ("Destroy target land. You
/// gain 4 life."): "If the targeted land is an illegal target by the time Maw
/// of the Mire resolves, it won't resolve and none of its effects will occur.
/// You won't gain 4 life."
///
/// The table above is all single-effect spells. This is the shape where CR
/// 608.2b bites hardest: a *second* effect that names no target at all, and
/// still does not happen, because the spell never resolves.
/// The same rule where the target lives in a graveyard rather than on the
/// battlefield: "Return target creature card from your graveyard to the
/// battlefield" (Unburial Rites), with the card exiled in response.
///
/// The battlefield cases above all move the target *to* a graveyard, so a
/// re-check that only asked "is this in a graveyard" would pass every one of
/// them for the wrong reason. This one moves it the other way.
#[test]
fn a_graveyard_target_that_leaves_the_graveyard_counters_the_spell() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let corpse = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let rites = castable_spell(&mut state, &reg, "Unburial Rites", P0);

    cast_then_move_target(&mut state, &reg, rites, corpse, Zone::Exile);

    assert!(!resolved(&state, rites), "the spell is countered, not resolved");
    assert_eq!(state.get_object(corpse).unwrap().zone, Zone::Exile,
        "and nothing was reanimated");
}

#[test]
fn a_countered_spells_untargeted_rider_does_not_happen_either() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();

    let maw = castable_spell(&mut state, &reg, "Maw of the Mire", P0);
    let life_before = state.get_player(P0).life;
    cast_then_move_target(&mut state, &reg, maw, forest, Zone::Graveyard);

    assert!(!resolved(&state, maw),
        "the only target is gone, so the spell is countered by game rules");
    assert_eq!(state.get_player(P0).life, life_before,
        "and \"you gain 4 life\" does not happen — it is part of a spell that \
         never resolved, not an independent effect");
    assert_eq!(state.get_object(maw).unwrap().zone, Zone::Graveyard,
        "the spell still goes to the graveyard");
}

/// The control for the test above: with the land still there the spell
/// resolves, destroys it, and the 4 life does arrive. Without this, an engine
/// that never gained the life would pass.
#[test]
fn maw_of_the_mire_gains_the_life_when_it_does_resolve() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();

    let maw = castable_spell(&mut state, &reg, "Maw of the Mire", P0);
    let life_before = state.get_player(P0).life;
    state = cast_onto_stack(&state, &reg, maw, vec![Target::Object(forest)]);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(resolved(&state, maw), "the target was still legal");
    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Graveyard, "the land is destroyed");
    assert_eq!(state.get_player(P0).life, life_before + 4, "and the 4 life arrives");
}

/// "Destroy" is not "exile" or "sacrifice": an indestructible land survives
/// (CR 701.7b). The spell still resolved, so the 4 life happens anyway — the
/// two sentences are sequential, not conditional on each other.
#[test]
fn maw_of_the_mire_gains_the_life_even_when_the_land_survives() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: forest,
        keyword: Keyword::Indestructible,
    });

    let maw = castable_spell(&mut state, &reg, "Maw of the Mire", P0);
    let life_before = state.get_player(P0).life;
    let state = cast_and_resolve(&state, &reg, maw, vec![Target::Object(forest)]);

    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Battlefield,
        "\"destroy\" does not move an indestructible permanent");
    assert_eq!(state.get_player(P0).life, life_before + 4,
        "but the spell resolved, so the life is gained regardless");
}

/// A spell resolves when its target is still there. The control for the table
/// above: without it, an engine that countered every spell would pass.
#[test]
fn a_spell_that_keeps_its_target_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = cast_onto_stack(&state, &reg, bolt, vec![Target::Object(creature)]);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(resolved(&state, bolt), "the target never became illegal");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 3, "and the Bolt hit it");
}

/// Two things that cannot become illegal targets, so the spells naming them
/// cannot fizzle: no target at all, and a player (nobody leaves a two-player
/// game mid-spell).
#[test]
fn a_spell_with_nothing_that_can_become_illegal_always_resolves() {
    let reg = registry();

    // No targets: Divination.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    stock_library(&mut state, &reg, P0, 5);
    let div = castable_spell(&mut state, &reg, "Divination", P0);
    state = cast_onto_stack(&state, &reg, div, vec![]);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(resolved(&state, div), "a spell with no targets has none to lose");
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2, "and it drew its two cards");

    // A player target: Lightning Bolt to the face.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = cast_onto_stack(&state, &reg, bolt, vec![Target::Player(P1)]);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(resolved(&state, bolt), "a player does not stop being a legal target");
    assert_eq!(state.get_player(P1).life, 17, "and took the 3 damage");
}

// ---------------------------------------------------------------------------
// More than one target (CR 608.2b)
// ---------------------------------------------------------------------------

/// "Up to two target creatures" — one target going illegal is not enough to
/// counter the spell; the rest of it still happens. All of them going illegal
/// is.
#[test]
fn a_multi_target_spell_is_countered_only_when_every_target_is_illegal() {
    let reg = registry();

    // One of two dies: the survivor is still tapped.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let a = ready_creature(&mut state, P1, 3, 3);
    let b = ready_creature(&mut state, P1, 2, 2);
    let dread = castable_spell(&mut state, &reg, "Feeling of Dread", P0);
    state = cast_onto_stack(&state, &reg, dread, vec![Target::Object(a), Target::Object(b)]);
    state.move_object(a, Zone::Graveyard, &reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(resolved(&state, dread), "one legal target left, so the spell resolves");
    assert!(state.get_object(b).unwrap().tapped,
        "and does as much as it can: the surviving creature is tapped");

    // Both die: countered by game rules.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let a = ready_creature(&mut state, P1, 3, 3);
    let b = ready_creature(&mut state, P1, 2, 2);
    let dread = castable_spell(&mut state, &reg, "Feeling of Dread", P0);
    state = cast_onto_stack(&state, &reg, dread, vec![Target::Object(a), Target::Object(b)]);
    state.move_object(a, Zone::Graveyard, &reg);
    state.move_object(b, Zone::Graveyard, &reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(!resolved(&state, dread), "no legal target left, so it is countered");
    assert_eq!(state.get_object(dread).unwrap().zone, Zone::Graveyard);
}

/// Prey Upon ("Target creature you control fights target creature you don't
/// control") keeps a legal target when the opponent's creature leaves, so the
/// spell is *not* countered — it resolves and the fight simply does not happen,
/// because a fight needs both creatures (CR 701.12b).
///
/// Resolving-and-doing-nothing and being-countered look the same from the
/// battlefield, which is why this asserts on both the spell's zone and the
/// surviving creature's damage.
#[test]
fn prey_upon_resolves_without_fighting_when_one_of_its_two_targets_is_gone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 3, 3);
    let theirs = ready_creature(&mut state, P1, 2, 2);

    let prey = castable_spell(&mut state, &reg, "Prey Upon", P0);
    state = cast_onto_stack(&state, &reg, prey, vec![Target::Object(mine), Target::Object(theirs)]);
    state.move_object(theirs, Zone::Graveyard, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(prey).unwrap().zone, Zone::Graveyard,
        "one target is still legal, so the spell resolves");
    assert_eq!(state.get_object(mine).unwrap().damage_marked, 0,
        "but a fight needs two creatures, so mine takes nothing");
}

/// Lost in the Mist ("Counter target spell. Return target permanent to its
/// owner's hand") is the one card in the set whose two targets are of
/// *different kinds*, so it is the only one that exercises `TwoTargets`'
/// per-slot legality re-check rather than one requirement applied twice.
///
/// Scryfall ruling (2011-09-22): "If one of Lost in the Mist's targets is
/// illegal by the time it resolves, Lost in the Mist will still affect the
/// remaining legal target. If both targets are illegal at this time, Lost in
/// the Mist won't resolve."
///
/// Both halves of the ruling, and both directions of the first half, because
/// the card does nothing observable beyond what it does to its targets: a
/// version that never fizzled would leave the same battlefield behind. Hence
/// `resolved()`.
#[test]
fn lost_in_the_mist_counters_or_bounces_whichever_target_survives() {
    let reg = registry();

    /// The Bears on the stack, a creature on the battlefield, and Lost in the
    /// Mist on top targeting both.
    fn setup(reg: &mtg_engine::cards::CardRegistry)
        -> (mtg_engine::state::GameState, ObjectId, ObjectId, ObjectId)
    {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let creature = ready_creature(&mut state, P1, 3, 3);
        let bears = castable_spell(&mut state, reg, "Grizzly Bears", P1);
        state.priority_player = Some(P1);
        let mut state = cast_onto_stack(&state, reg, bears, vec![]);
        let litm = castable_spell(&mut state, reg, "Lost in the Mist", P0);
        state.priority_player = Some(P0);
        let state = cast_onto_stack(&state, reg, litm,
            vec![Target::Object(bears), Target::Object(creature)]);
        (state, litm, bears, creature)
    }

    // The permanent leaves: the counter half still happens.
    let (mut state, litm, bears, creature) = setup(&reg);
    state.move_object(creature, Zone::Graveyard, &reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(resolved(&state, litm), "the spell half is still legal");
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "so the Bears are still countered");

    // The spell leaves: the bounce half still happens.
    let (mut state, litm, bears, creature) = setup(&reg);
    state.stack.retain(|e| e.as_spell() != Some(bears));
    state.move_object(bears, Zone::Graveyard, &reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(resolved(&state, litm), "the permanent half is still legal");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Hand,
        "so the creature is still bounced");

    // Both leave: countered by game rules. Nothing on the battlefield tells
    // this apart from the spell resolving and finding neither target, which is
    // what `resolved` is for.
    let (mut state, litm, bears, creature) = setup(&reg);
    state.stack.retain(|e| e.as_spell() != Some(bears));
    state.move_object(bears, Zone::Graveyard, &reg);
    state.move_object(creature, Zone::Graveyard, &reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(!resolved(&state, litm), "neither target is legal, so it is countered");
    assert_eq!(state.get_object(litm).unwrap().zone, Zone::Graveyard);
}

// ---------------------------------------------------------------------------
// Fizzling does not skip the flashback replacement (CR 702.33a)
// ---------------------------------------------------------------------------

/// "Then exile it" applies to a flashback spell that was countered by game
/// rules just as much as to one that resolved: the card left the stack either
/// way.
#[test]
fn a_fizzled_flashback_spell_is_still_exiled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let geistflame = named_card_in_graveyard(&mut state, &reg, "Geistflame", P0);
    // Flashback {3}{R}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let creature = ready_creature(&mut state, P1, 2, 2);
    state = cast_onto_stack(&state, &reg, geistflame, vec![Target::Object(creature)]);
    assert!(state.get_object(geistflame).unwrap().cast_with_flashback,
        "test setup: this is the flashback cast, not a cast from hand");

    state.move_object(creature, Zone::Graveyard, &reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(!resolved(&state, geistflame), "the target is gone, so it is countered");
    assert_eq!(state.get_object(geistflame).unwrap().zone, Zone::Exile,
        "and the flashback replacement still exiles it rather than letting it \
         return to the graveyard to be cast again");
    assert_eq!(state.get_player(P1).life, 20, "no damage was dealt");
}

// ---------------------------------------------------------------------------
// A spell is a legal target too
// ---------------------------------------------------------------------------

/// Counterspell whose target has already left the stack: the same rule, with a
/// spell rather than a permanent as the target.
#[test]
fn counterspell_is_countered_when_its_target_has_left_the_stack() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = cast_onto_stack(&state, &reg, bolt, vec![Target::Player(P1)]);

    state.priority_player = Some(P1);
    let counter = castable_spell(&mut state, &reg, "Counterspell", P1);
    state = cast_onto_stack(&state, &reg, counter, vec![Target::Object(bolt)]);

    // The Bolt leaves the stack before the Counterspell resolves.
    state.stack.retain(|e| e.as_spell() != Some(bolt));
    state.move_object(bolt, Zone::Graveyard, &reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(!resolved(&state, counter),
        "the spell it named is no longer on the stack, so it is countered by \
         game rules (CR 608.2b)");
}

/// Dissipate whose target has left the stack: countered by game rules, and it
/// is Dissipate that goes to the graveyard while nothing at all is exiled.
///
/// This is *adjacent* to the 2004-10-04 ruling "If the spell is not countered
/// ..., then it does not get exiled", but it is not a test of it, and the
/// distinction is worth writing down. The ruling is about Dissipate resolving
/// and failing to counter an uncounterable spell — no card in this set can't
/// be countered, and with a single target the only way the counter does not
/// happen is the spell never resolving at all. So `counter_spell_exiling`'s
/// own "is it still on the stack?" guard is unreachable from here: mutating it
/// to exile regardless changes nothing, because `on_resolve` is never called.
///
/// What this does hold is the other half, which a rider implemented carelessly
/// could get wrong: "exile it instead" is about the *countered* spell, so a
/// fizzling Dissipate must land in its owner's graveyard like any other spell.
#[test]
fn a_fizzling_dissipate_goes_to_the_graveyard_and_exiles_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);
    let mut state = cast_onto_stack(&state, &reg, bears, vec![]);
    let dissipate = castable_spell(&mut state, &reg, "Dissipate", P1);
    state.priority_player = Some(P1);
    let mut state = cast_onto_stack(&state, &reg, dissipate, vec![Target::Object(bears)]);

    // Something else deals with the Bears first — it is off the stack and in
    // the graveyard by the time Dissipate resolves.
    state.stack.retain(|e| e.as_spell() != Some(bears));
    state.move_object(bears, Zone::Graveyard, &reg);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(!resolved(&state, dissipate),
        "its only target is gone, so Dissipate is countered by game rules");
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "the Bears are where they already were; nothing was exiled");
    assert_eq!(state.get_object(dissipate).unwrap().zone, Zone::Graveyard,
        "and Dissipate itself goes to its owner's graveyard — it is the \
         countered spell that gets exiled, never Dissipate");
}

/// CR 608.2b: on resolution, a spell checks each of its targets. One that is no
/// longer legal is not affected; the spell still resolves and still affects the
/// targets that are.
///
/// The engine used to compute only whether *any* target was still legal, and
/// then hand the whole original list to the card. A target that had become
/// illegal without leaving the battlefield — the ordinary case, a creature
/// given hexproof in response — was still affected.
///
/// Into the Maw of Hell is the clean statement of it, because its two halves
/// hit different permanents and its Scryfall ruling says so outright: "If one
/// of Into the Maw of Hell's targets is illegal by the time it resolves, Into
/// the Maw of Hell will still affect the remaining legal target."
#[test]
fn a_target_that_gained_hexproof_in_response_is_skipped_and_the_rest_resolve() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = named_permanent(&mut state, &reg, "Forest", P1);
    let creature = ready_creature(&mut state, P1, 5, 5);
    let spell = castable_spell(&mut state, &reg, "Into the Maw of Hell", P0);
    let mut state = cast_onto_stack(&state, &reg, spell,
        vec![Target::Object(land), Target::Object(creature)]);

    // In response, the creature gains hexproof (Ranger's Guile is in this set).
    // It never leaves the battlefield — only its targetability changes.
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: creature,
        keyword: Keyword::Hexproof,
    });

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
        "the creature is no longer a legal target for an opponent's spell \
         (CR 702.11b), so it takes none of the 13 damage");
    assert_ne!(state.get_object(land).unwrap().zone, Zone::Battlefield,
        "the land was still a legal target, so that half of the spell happened \
         — the spell is not countered while one target remains (CR 608.2b)");

    // A partial fizzle used to log byte-identically to a full resolution
    // (issue #135): the log now says which target dropped out.
    assert!(state.game_log.iter().any(|e|
        e.message.contains("is illegal, resolving with the rest")),
        "the partial fizzle is said out loud; log: {:?}",
        state.game_log.iter().map(|e| &e.message).collect::<Vec<_>>());
}

/// CR 608.2b, the half of `CreatureWithFilter` the re-check used to skip: a
/// "target creature" that has stopped being a creature is no longer a legal
/// target, whatever the filter says about it.
///
/// `is_target_legal` re-ran the filter — "you control", "power 4 or greater" —
/// but never creature-ness, so seven cards each restated it in their own
/// `is_valid_target`. Nothing in this set turns a creature into a
/// non-creature, so no card can stage this; the state is built directly, which
/// is the only way to hold an engine rule that the card pool never reaches.
///
/// Both shapes of "target creature" ask the same question and both have to
/// re-ask it. `CreatureWithFilter` got the re-check when the seven duplicates
/// were collapsed; bare `Creature` — "target creature" with no further
/// restriction, which is most of them — was left taking it on trust.
#[test]
fn a_target_creature_that_stopped_being_a_creature_is_no_longer_legal() {
    // (card, the requirement it declares)
    const SPELLS: &[(&str, &str)] = &[
        ("Ranger's Guile", "CreatureWithFilter(YouControl)"),
        ("Traitorous Blood", "Creature"),
    ];

    for &(card, requirement) in SPELLS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let creature = ready_creature(&mut state, P0, 2, 2);
        let spell = castable_spell(&mut state, &reg, card, P0);
        let mut state = cast_onto_stack(&state, &reg, spell, vec![Target::Object(creature)]);

        // An anonymous object is a creature by virtue of carrying a P/T
        // (CR 205.1b, and `card_types_of` says so); taking that away is the
        // shortest honest way to make it stop being one.
        state.get_object_mut(creature).unwrap().power = None;
        state.get_object_mut(creature).unwrap().toughness = None;
        assert!(!state.is_creature(creature, &reg), "test precondition");

        state.events.clear();
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

        assert!(!resolved(&state, spell),
            "{card} declares {requirement}; its only target is no longer a \
             creature, so the spell is countered by game rules");
    }
}

/// Ranger's Guile is what the test above simulates by hand, and the reason the
/// card exists: cast in response to removal, it makes its creature an illegal
/// target and the removal is countered by game rules.
///
/// Asserted through `resolved()` rather than "the creature survived", because
/// a single-target spell that resolved and found nothing to do leaves exactly
/// the same board.
#[test]
fn rangers_guile_counters_removal_by_granting_hexproof() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let removal = castable_spell(&mut state, &reg, "Doom Blade", P1);
    state.priority_player = Some(P1);
    let mut state = cast_onto_stack(&state, &reg, removal, vec![Target::Object(creature)]);

    // In response, its controller casts Ranger's Guile on it.
    state.priority_player = Some(P0);
    let guile = castable_spell(&mut state, &reg, "Ranger's Guile", P0);
    let mut state = cast_and_resolve(&state, &reg, guile, vec![Target::Object(creature)]);
    assert!(state.has_keyword(creature, Keyword::Hexproof, &reg),
        "test precondition: the Guile resolved and granted hexproof");

    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(!resolved(&state, removal),
        "its only target is no longer legal for an opponent's spell, so the \
         removal is countered by game rules (CR 608.2b / 702.11b)");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "and the creature is still there");
    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "with the +1/+1 as well — both halves of one spell");
}

/// The mirror of the case above — the *land* is the one that becomes illegal —
/// and then the case the same ruling ends on: "If both targets are illegal at
/// this time, Into the Maw of Hell won't resolve."
///
/// The existing test covers only the creature half, which a card that had
/// simply forgotten to check its first target would also pass.
#[test]
fn into_the_maw_of_hell_keeps_the_half_whose_target_is_still_legal() {
    let reg = registry();

    // Only the land becomes illegal: the creature still burns.
    {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let land = named_permanent(&mut state, &reg, "Forest", P1);
        let creature = ready_creature(&mut state, P1, 5, 5);
        let spell = castable_spell(&mut state, &reg, "Into the Maw of Hell", P0);
        let mut state = cast_onto_stack(&state, &reg, spell,
            vec![Target::Object(land), Target::Object(creature)]);

        state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
            target: land,
            keyword: Keyword::Hexproof,
        });
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

        assert_eq!(state.get_object(land).unwrap().zone, Zone::Battlefield,
            "the land is no longer targetable, so it is not destroyed");
        assert_eq!(state.get_object(creature).unwrap().damage_marked, 13,
            "but the creature was still legal and takes its 13");
    }

    // Both become illegal: the spell is countered by game rules and does
    // nothing at all.
    {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let land = named_permanent(&mut state, &reg, "Forest", P1);
        let creature = ready_creature(&mut state, P1, 5, 5);
        let spell = castable_spell(&mut state, &reg, "Into the Maw of Hell", P0);
        let mut state = cast_onto_stack(&state, &reg, spell,
            vec![Target::Object(land), Target::Object(creature)]);

        for target in [land, creature] {
            state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
                target,
                keyword: Keyword::Hexproof,
            });
        }
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

        assert_eq!(state.get_object(land).unwrap().zone, Zone::Battlefield,
            "no legal target remains, so the spell does not resolve");
        assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
            "neither half happens");
        assert_eq!(state.get_object(spell).unwrap().zone, Zone::Graveyard,
            "a countered spell still goes to its owner's graveyard (CR 608.2b)");
    }
}

/// CR 608.2b applies to activated abilities too, and `stack.rs`'s
/// `StackEntry::Ability` arm had no legality check at all — an ability
/// resolved against whatever it had targeted however the board had changed.
///
/// Ghost Quarter's ruling is the plain statement of it: "If the targeted land
/// is an illegal target by the time Ghost Quarter's ability resolves, it won't
/// resolve and none of its effects will happen. The land's controller won't get
/// to search for a basic land card."
///
/// This needs a window between activation and resolution, which the engine did
/// not have: it resolved an activated ability immediately after pushing it, so
/// nothing could ever be done in response to one.
#[test]
fn an_activated_abilitys_targets_are_rechecked_when_it_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let quarter = named_permanent(&mut state, &reg, "Ghost Quarter", P0);
    let victim = named_permanent(&mut state, &reg, "Forest", P1);
    // Give P1 a basic to find, so "no search happened" cannot be for want of one.
    let library_basic = state.create_object(
        reg.get_id_by_name("Island").unwrap(), P1, Zone::Library, None, None);
    state.get_player_mut(P1).library_order.push(library_basic);

    let state = activate_onto_stack(&state, &reg, quarter, Some(Target::Object(victim)));
    let mut state = state;

    // Between activation and resolution the land becomes untargetable.
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: victim,
        keyword: Keyword::Hexproof,
    });
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
        "the land is no longer a legal target, so it is not destroyed");
    assert_eq!(state.get_object(library_basic).unwrap().zone, Zone::Library,
        "and none of the ability's other effects happen either — the search \
         does not occur (CR 608.2b)");
}

/// Ruling: "If the targeted permanent or player is an illegal target by the
/// time the ability resolves, the entire ability won't resolve. No cards will
/// be put into your graveyard, and no damage will be dealt."
///
/// Both halves matter. The mill is the part a card that checked its target
/// *after* milling would get wrong, and the library is the only place that
/// shows it.
#[test]
fn heretics_punishment_mills_nothing_when_its_target_is_gone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let punishment = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);
    stock_library(&mut state, &reg, P0, 6);
    let victim = ready_creature(&mut state, P1, 5, 5);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 4);

    let action = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == punishment && targets == &[Target::Object(victim)]))
        .expect("the ability, targeting the creature");
    let mut state = mtg_engine::engine::submit_action(&state, &action, &reg);

    let library_before = state.get_player(P0).library_order.len();
    // In response, the creature leaves the battlefield.
    state.move_object(victim, Zone::Graveyard, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P0).library_order.len(), library_before,
        "the entire ability does not resolve, so no cards are milled");
    assert_eq!(state.get_object(victim).unwrap().damage_marked, 0,
        "and no damage is dealt");
}


/// The same for an ability whose card does *not* restate its own targeting
/// restriction — which is most of them, and was the hole.
///
/// Silverchase Fox's "{1}{W}, Sacrifice this creature: Exile target
/// enchantment" used to guard the target inside its own resolution: the
/// ability resolved, found the enchantment gone and did nothing. The board
/// ended up right by the wrong route, and any ability without such a guard did
/// not even manage that. With the requirement riding on the stack entry, the
/// engine counters the ability by game rules — and the difference is visible,
/// because a resolution here would exile the enchantment out of the graveyard.
#[test]
fn an_abilitys_declared_requirement_is_rechecked_when_it_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fox = named_permanent(&mut state, &reg, "Silverchase Fox", P0);
    let enchantment = named_permanent(&mut state, &reg, "Glorious Anthem", P1);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 1), (ManaType::White, 1)]);

    let action = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == fox && targets == &[Target::Object(enchantment)]))
        .expect("the ability, targeting the enchantment");
    let mut state = mtg_engine::engine::submit_action(&state, &action, &reg);
    assert_eq!(state.get_object(fox).unwrap().zone, Zone::Graveyard,
        "test premise: the Fox is sacrificed as a cost, so its ability list is \
         gone before the ability resolves");

    // In response, the enchantment is destroyed.
    state.move_object(enchantment, Zone::Graveyard, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(enchantment).unwrap().zone, Zone::Graveyard,
        "the enchantment is no longer a legal target for \"target enchantment\", \
         so the ability is countered by game rules and exiles nothing");
}

/// CR 608.2b applies to a triggered ability like any other, and this file had
/// no case for one — spells, and one activated ability, but nothing that
/// triggered.
///
/// Pitchburn Devils is the shape that shows it: "When this creature dies, it
/// deals 3 damage to any target", so the target is chosen as the trigger goes
/// on the stack (CR 603.3d) and there is a priority window before it resolves.
///
/// The target gains hexproof in that window rather than dying in it. Killing
/// it instead proves nothing: damage aimed at a creature in a graveyard lands
/// nowhere whether the trigger was countered or not, so the test passes with
/// the re-check deleted. It has to stop being *legal* while staying somewhere
/// the damage could still have gone.
#[test]
fn a_triggered_abilitys_target_is_rechecked_when_it_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils = named_permanent(&mut state, &reg, "Pitchburn Devils", P0);
    // 4/4, so it survives the three damage and the marked damage is what the
    // assertion reads.
    let victim = ready_creature(&mut state, P1, 4, 4);

    kill_by_damage(&mut state, &reg, devils);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    let mut state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(victim))),
        },
        &reg,
    );
    assert!(!state.stack.is_empty(), "test setup: the trigger is on the stack, targeted");

    // In response, it becomes untargetable by its opponent's abilities.
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: victim,
        keyword: Keyword::Hexproof,
    });
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
        "test premise: it is still there to be damaged");
    assert_eq!(state.get_object(victim).unwrap().damage_marked, 0,
        "it is no longer a legal target, so the trigger is countered by game \
         rules and deals no damage");
    assert_eq!(state.get_player(P1).life, 20,
        "and the damage does not go somewhere else for want of its target");
}

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
use mtg_engine::actions::Target;
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
/// because a fight needs both creatures (CR 701.15).
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
}

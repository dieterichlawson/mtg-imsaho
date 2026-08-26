//! Tests for instants, sorceries, and targeting.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

/// Lightning Bolt deals 3 damage to a creature, killing a 3-toughness creature.
#[test]
fn lightning_bolt_kills_creature() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has a Lightning Bolt in hand and {R} in pool.
    let bolt = castable_spell(&mut state, &registry, "Lightning Bolt", P0);

    // P1 has a 3/3 creature.
    let creature = ready_creature(&mut state, P1, 3, 3);

    // Cast Lightning Bolt targeting the creature.
    state = cast_onto_stack(&state, &registry, bolt, vec![Target::Object(creature)]);
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Stack);

    // Resolve it.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Creature should have 3 damage.
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 3);
    // Bolt should be in graveyard.
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Graveyard);

    // SBA should kill the creature.
    check_state_based_actions(&mut state, &registry);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Direct-damage spells that hit a player drain that player's life by the
/// spell's stated amount. Lightning Bolt's creature-target behavior is
/// covered separately above; per-spell edge cases (flashback on Bump in
/// the Night, morbid on Brimstone Volley, creature-only on Geistflame)
/// have their own tests.
#[test]
fn direct_damage_spells_drain_player_life() {
    let reg = registry();
    for (name, damage) in [
        ("Lightning Bolt",    3u32),
        ("Lava Axe",          5),
        ("Bump in the Night", 3),
        ("Brimstone Volley",  3),
    ] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let spell = castable_spell(&mut state, &reg, name, P0);
        state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);
        assert_eq!(state.get_player(P1).life, 20 - damage as i32,
            "{name} should deal {damage} damage to the targeted player");
    }
}

/// CR 307.1: a sorcery may be cast only during its controller's main phase,
/// with an empty stack. An instant has no such restriction.
///
/// Every row asks the same question of `legal_actions`, scoped to the spell in
/// hand — asking whether *any* `CastSpell` is offered, as these used to, is a
/// different question once the hand holds more than one card.
#[test]
fn sorcery_timing_restricts_sorceries_and_leaves_instants_alone() {
    /// The board the spell is being cast into.
    enum When {
        /// The caster's own main phase, empty stack — always legal.
        OwnMainPhase,
        /// The opponent's main phase.
        OpponentsTurn,
        /// Mid-combat, on the caster's own turn.
        DuringCombat,
        /// The caster's main phase, but something is already on the stack.
        StackNotEmpty,
    }

    // (spell, when, castable?)
    let cases = [
        ("Lightning Bolt", When::OwnMainPhase, true),
        ("Lightning Bolt", When::OpponentsTurn, true),
        ("Lightning Bolt", When::DuringCombat, true),
        ("Lightning Bolt", When::StackNotEmpty, true),
        ("Divination", When::OwnMainPhase, true),
        ("Divination", When::OpponentsTurn, false),
        ("Divination", When::DuringCombat, false),
        ("Divination", When::StackNotEmpty, false),
    ];

    for (name, when, castable) in cases {
        let reg = registry();
        let (step, active) = match when {
            When::OpponentsTurn => (Step::PrecombatMain, P1),
            When::DuringCombat => (Step::DeclareBlockers, P0),
            _ => (Step::PrecombatMain, P0),
        };
        let mut state = game_at_step(step, active);
        state.priority_player = Some(P0);

        // Something for the Bolt to point at, so it is never held back for
        // want of a target.
        ready_creature(&mut state, P1, 2, 2);
        if matches!(when, When::StackNotEmpty) {
            let dummy = state.create_object(CardId(99), P0, Zone::Stack, None, None);
            state.stack.push(mtg_engine::state::StackEntry::Spell(dummy));
        }

        let spell = spell_in_hand(&mut state, &reg, name, P0);
        add_mana_for(&mut state, &reg, name, P0);

        let situation = match when {
            When::OwnMainPhase => "own main phase, empty stack",
            When::OpponentsTurn => "the opponent's turn",
            When::DuringCombat => "during combat",
            When::StackNotEmpty => "with a spell already on the stack",
        };
        assert_eq!(can_cast(&state, &reg, spell), castable, "{name} in {situation}");
    }
}

/// Divination draws two cards.
#[test]
fn divination_draws_two() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    stock_library(&mut state, &registry, P0, 2);

    let div = castable_spell(&mut state, &registry, "Divination", P0);

    // Cast and resolve.
    state = cast_and_resolve(&state, &registry, div, vec![]);

    // Should have drawn 2 cards.
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2);
    assert_eq!(state.get_object(div).unwrap().zone, Zone::Graveyard);
}

/// Swords to Plowshares exiles and gains life.
#[test]
fn swords_exiles_and_gains_life() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let swords = castable_spell(&mut state, &registry, "Swords to Plowshares", P0);

    // P1 has a 5/5 creature.
    let creature = ready_creature(&mut state, P1, 5, 5);

    state = cast_and_resolve(&state, &registry, swords, vec![Target::Object(creature)]);

    // Creature should be exiled.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Exile);
    // P1 gains 5 life.
    assert_eq!(state.get_player(P1).life, 25);
    // Swords in graveyard.
    assert_eq!(state.get_object(swords).unwrap().zone, Zone::Graveyard);
}

/// Doom Blade destroys a creature.
#[test]
fn doom_blade_destroys() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let doom = castable_spell(&mut state, &registry, "Doom Blade", P0);

    let creature = ready_creature(&mut state, P1, 10, 10);

    state = cast_and_resolve(&state, &registry, doom, vec![Target::Object(creature)]);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Targeted spells need valid targets — can't cast Lightning Bolt with no creatures or players.
#[test]
fn targeted_spell_needs_valid_target() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Doom Blade targets creatures — with none on the battlefield there is no
    // legal target, so the cast is not offered (CR 601.2c).
    let doom = spell_in_hand(&mut state, &registry, "Doom Blade", P0);
    add_mana_for(&mut state, &registry, "Doom Blade", P0);

    assert!(!can_cast(&state, &registry, doom),
        "Doom Blade needs a creature to point at");
}

/// Counterspell counters a spell, preventing it from resolving.
#[test]
fn counterspell_counters_spell() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts Lightning Bolt targeting P1.
    let bolt = castable_spell(&mut state, &registry, "Lightning Bolt", P0);

    state = cast_onto_stack(&state, &registry, bolt, vec![Target::Player(P1)]);
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Stack);

    // P1 responds with Counterspell targeting the Bolt.
    state.priority_player = Some(P1);
    let counter = castable_spell(&mut state, &registry, "Counterspell", P1);

    state = cast_and_resolve(&state, &registry, counter, vec![Target::Object(bolt)]);

    // Bolt should be in graveyard without resolving — P1 life unchanged.
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_player(P1).life, 20);
    // Counterspell itself in graveyard.
    assert_eq!(state.get_object(counter).unwrap().zone, Zone::Graveyard);
}

/// "Counter target spell" needs a spell on the stack to point at, so the cast
/// is offered exactly when there is one (CR 601.2c). Both arms: with only the
/// negative, an engine that never offered Counterspell would pass.
#[test]
fn counterspell_is_offered_exactly_when_a_spell_is_on_the_stack() {
    let reg = registry();

    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P1);
    let counter = spell_in_hand(&mut state, &reg, "Counterspell", P1);
    add_mana_for(&mut state, &reg, "Counterspell", P1);
    assert!(!can_cast(&state, &reg, counter), "nothing on the stack to counter");

    let bolt_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let bolt = state.create_object(bolt_id, P0, Zone::Stack, None, None);
    state.stack.push(mtg_engine::state::StackEntry::Spell(bolt));

    let offered = offered_targets(&state, &reg, counter);
    assert!(offered.contains(&Target::Object(bolt)),
        "with a spell on the stack, Counterspell is offered pointing at it; \
         offered {offered:?}");
}

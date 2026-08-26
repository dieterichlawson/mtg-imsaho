//! Creatures with evasion keywords, and creatures whose power and toughness
//! are a function of the graveyard (CR 208.2).
//!
//! Cards covered (6), so this is greppable by name as well as by rule:
//!
//! - Battleground Geist
//! - Gallows Warden
//! - Geist-Honored Monk
//! - Orchard Spirit
//! - Spider Spawning
//! - Wreath of Geists
//!
//! Festerhide Boar's morbid "enters with counters" is in `intervening_if.rs`,
//! with the rest of CR 603.4.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::combat;
use mtg_engine::ids::CardId;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ── Spirit lords ───────────────────────────────────────────────────

/// "Other Spirit creatures you control get +N/+M." Four claims in one line —
/// other, Spirit, you control, and the size — so each lord is checked against
/// a board holding one of each.
#[test]
fn a_spirit_lord_buffs_other_spirits_you_control_and_nothing_else() {
    // (lord, its printed size, the power/toughness it grants)
    const LORDS: &[(&str, (i32, i32), (i32, i32))] = &[
        ("Battleground Geist", (3, 3), (1, 0)),
        ("Gallows Warden", (3, 3), (0, 1)),
    ];

    for &(name, printed, (dp, dt)) in LORDS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let lord = named_permanent(&mut state, &reg, name, P0);
        let spirit = named_permanent(&mut state, &reg, "Chapel Geist", P0);   // 2/3 Spirit
        let non_spirit = ready_creature(&mut state, P0, 2, 2);
        let their_spirit = named_permanent(&mut state, &reg, "Chapel Geist", P1);

        let pt = |s: &mtg_engine::state::GameState, id| {
            (s.effective_power(id, &reg).unwrap(), s.effective_toughness(id, &reg).unwrap())
        };

        assert_eq!(pt(&state, lord), printed, "{name}: 'other' excludes itself");
        assert_eq!(pt(&state, spirit), (2 + dp, 3 + dt), "{name}: your own Spirit is buffed");
        assert_eq!(pt(&state, non_spirit), (2, 2), "{name}: a non-Spirit is not");
        assert_eq!(pt(&state, their_spirit), (2, 3), "{name}: an opponent's Spirit is not");
    }
}

// ── Dynamic P/T ────────────────────────────────────────────────────

/// Geist-Honored Monk P/T equals creatures you control, ETB creates 2 tokens.
#[test]
fn geist_honored_monk_dynamic_pt_and_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monk = castable_spell(&mut state, &reg, "Geist-Honored Monk", P0);
    stock_library(&mut state, &reg, P0, 5);

    state = cast_and_resolve(&state, &reg, monk, vec![]);
    triggers::process_triggers(&mut state, &reg);

    let creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P0 && o.power.is_some())
        .count();
    assert_eq!(creatures, 3, "Monk + 2 Spirit tokens");
    assert_eq!(state.effective_power(monk, &reg), Some(3),
        "and its P/T counts them, itself included");
    assert_eq!(state.effective_toughness(monk, &reg), Some(3));
}

/// "Enchanted creature gets +X/+X, where X is the number of creature cards in
/// your graveyard." A characteristic-defining count, so it is recomputed as the
/// graveyard changes rather than fixed when the Aura resolved.
#[test]
fn wreath_of_geists_tracks_the_graveyard_as_it_changes() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let wreath = castable_spell(&mut state, &reg, "Wreath of Geists", P0);
    state = cast_and_resolve(&state, &reg, wreath, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "an empty graveyard is X = 0");

    for expected in 3..=5 {
        state.create_object(CardId(9999), P0, Zone::Graveyard, Some(1), Some(1));
        assert_eq!(state.effective_power(creature, &reg), Some(expected),
            "each creature card added to the graveyard raises X");
        assert_eq!(state.effective_toughness(creature, &reg), Some(expected));
    }
}

// ── Block restriction ──────────────────────────────────────────────

/// "Orchard Spirit can't be blocked except by creatures with flying or reach."
/// Three rows, because "the ground creature can't block it" alone is also true
/// of an Orchard Spirit that nothing at all could block.
#[test]
fn orchard_spirit_is_blocked_only_by_flying_or_reach() {
    let reg = registry();
    // (blocker, may it block)
    const BLOCKERS: &[(&str, bool, &str)] = &[
        ("Walking Corpse", false, "a ground creature"),
        ("Chapel Geist", true, "flying"),
        ("Somberwald Spider", true, "reach"),
    ];

    for &(name, may_block, why) in BLOCKERS {
        let mut state = game_at_step(Step::DeclareBlockers, P0);
        let spirit = named_permanent(&mut state, &reg, "Orchard Spirit", P0);
        let blocker = named_permanent(&mut state, &reg, name, P1);

        assert_eq!(combat::can_block_attacker(&state, blocker, spirit, &reg), may_block,
            "{name} ({why})");
    }
}

// ── Token creation from graveyard ──────────────────────────────────

/// Spider Spawning creates tokens equal to creatures in graveyard.
#[test]
fn spider_spawning_creates_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    for i in 0..4 {
        let c = state.create_object(CardId(9999), P0, Zone::Graveyard, Some(1), Some(1));
        state.get_object_mut(c).unwrap().name = format!("Dead {i}");
    }

    let ss = castable_spell(&mut state, &reg, "Spider Spawning", P0);
    state = cast_and_resolve(&state, &reg, ss, vec![]);

    assert_eq!(count_tokens_named(&state, "Spider"), 4, "one Spider per creature card");
    for spider in state.objects.values().filter(|o| o.is_token && o.name == "Spider") {
        assert_eq!((spider.power, spider.toughness), (Some(1), Some(2)));
    }
}

//! Bug test: Liliana of the Veil +1 discards sequentially instead of simultaneously.
//!
//! Per the 2022-09-09 ruling: "first the player whose turn it is chooses a card
//! in hand without revealing it, then each other player in turn order does the
//! same. Then all the chosen cards are discarded at the same time."
//!
//! The engine discards P0's card immediately (auto-discard for 1 card in hand)
//! before P1 is even prompted to choose. This leaks information and causes
//! discard triggers to fire non-simultaneously.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::types::*;

/// Liliana +1 should not move any card to the graveyard until all players
/// have chosen. With P0 holding 1 card (auto-choice) and P1 holding 2,
/// no Discarded events should exist while P1's choice is still pending.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn bug_liliana_plus1_discards_before_all_players_choose() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has exactly 1 card — the engine will auto-discard this.
    spell_in_hand(&mut state, &registry, "Grizzly Bears", P0);

    // P1 has 2 cards — the engine should prompt P1 to choose.
    spell_in_hand(&mut state, &registry, "Giant Growth", P1);
    spell_in_hand(&mut state, &registry, "Doom Blade", P1);

    // Place Liliana on the battlefield under P0's control with loyalty counters.
    let liliana = named_creature(&mut state, &registry, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);

    // Clear any pre-existing events from setup.
    state.events.clear();

    // Activate the +1 ability (ability_index 0, no targets).
    let behavior = registry.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &registry);

    // Per the ruling, all discards happen simultaneously AFTER all players choose.
    // Since P1 has 2 cards and hasn't chosen yet, zero Discarded events should
    // have fired. P0's card should still be in hand (choice recorded, not executed).
    let discard_events: Vec<_> = state.events.iter().filter(|e| {
        matches!(e, GameEvent::Discarded { .. })
    }).collect();

    let p0_graveyard_count = state.objects_in_zone(Zone::Graveyard, P0).len();

    // Oracle ruling says all chosen cards are discarded at the same time.
    // No card should be in the graveyard while P1's choice is still pending.
    assert_eq!(
        p0_graveyard_count, 0,
        "Liliana +1 sequential discard bug: P0's card was moved to graveyard \
         before P1 chose. Ruling says all discards happen simultaneously. \
         P0 graveyard has {} cards, expected 0 while P1 choice is pending.",
        p0_graveyard_count
    );

    assert_eq!(
        discard_events.len(), 0,
        "Liliana +1 sequential discard bug: {} Discarded events fired before \
         all players chose. Ruling says discards are simultaneous — no events \
         should fire until every player has selected a card.",
        discard_events.len()
    );
}

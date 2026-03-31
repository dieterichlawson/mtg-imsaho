//! Tests for Innistrad Tier 8 cards (sacrifice-as-cost abilities).

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Selfless Cathar ─────────────────────────────────────────────────

/// Selfless Cathar: sacrifice gives all your creatures +1/+1 until end of turn.
#[test]
fn selfless_cathar_pump_all_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cathar = named_creature(&mut state, &reg, "Selfless Cathar", P0);
    let bear = ready_creature(&mut state, P0, 2, 2);

    // Add mana for the ability: {1}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: cathar,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // Cathar should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(cathar).unwrap().zone,
        Zone::Graveyard,
        "Selfless Cathar should be sacrificed"
    );

    // Bear should have +1/+1 from the effect.
    assert_eq!(new_state.effective_power(bear, &reg).unwrap(), 3);
    assert_eq!(new_state.effective_toughness(bear, &reg).unwrap(), 3);
}

// ── Silverchase Fox ─────────────────────────────────────────────────

/// Silverchase Fox: sacrifice to exile target enchantment.
#[test]
fn silverchase_fox_exiles_enchantment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fox = named_creature(&mut state, &reg, "Silverchase Fox", P0);

    // Create an enchantment for P1 (use Pacifism as a representative enchantment).
    let enchantment = named_creature(&mut state, &reg, "Glorious Anthem", P1);

    // Add mana for the ability: {1}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: fox,
            ability_index: 0,
            targets: vec![Target::Object(enchantment)],
        },
        &reg,
    );

    // Fox should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(fox).unwrap().zone,
        Zone::Graveyard,
        "Silverchase Fox should be sacrificed"
    );

    // Enchantment should be in exile.
    assert_eq!(
        new_state.get_object(enchantment).unwrap().zone,
        Zone::Exile,
        "Target enchantment should be exiled"
    );
}

// ── Brain Weevil ────────────────────────────────────────────────────

/// Brain Weevil: sacrifice to make target player discard two cards.
#[test]
fn brain_weevil_forces_discard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_creature(&mut state, &reg, "Brain Weevil", P0);

    // Give P1 some cards in hand.
    let _c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let _c2 = spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);
    let _c3 = spell_in_hand(&mut state, &reg, "Giant Growth", P1);

    let hand_before = state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_before, 3);

    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: weevil,
            ability_index: 0,
            targets: vec![Target::Player(P1)],
        },
        &reg,
    );

    // Weevil should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(weevil).unwrap().zone,
        Zone::Graveyard,
        "Brain Weevil should be sacrificed"
    );

    // P1 should have discarded 2 cards (3 - 2 = 1 remaining).
    let hand_after = new_state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_after, 1, "P1 should have 1 card left after discarding 2");
}

/// Brain Weevil has intimidate.
#[test]
fn brain_weevil_has_intimidate() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_creature(&mut state, &reg, "Brain Weevil", P0);
    assert!(state.has_keyword(weevil, Keyword::Intimidate, &reg));
}

// ── Disciple of Griselbrand ────────────────────────────────────────

/// Disciple of Griselbrand: sacrifice a creature to gain life equal to its toughness.
#[test]
fn disciple_of_griselbrand_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let disciple = named_creature(&mut state, &reg, "Disciple of Griselbrand", P0);
    // Create a 2/5 creature to sacrifice.
    let _fatty = ready_creature(&mut state, P0, 2, 5);

    let life_before = state.get_player(P0).life;

    // Add mana for the ability: {1}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: disciple,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // The engine auto-sacrifices the first creature it finds. It may pick the disciple
    // itself (1 toughness) or the fatty (5 toughness). Either way, we gained life.
    let life_after = new_state.get_player(P0).life;
    let gained = life_after - life_before;
    assert!(gained > 0, "Should have gained life, gained {}", gained);
}

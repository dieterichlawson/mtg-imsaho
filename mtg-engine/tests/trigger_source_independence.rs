//! CR 113.7a: a triggered ability on the stack exists independently of its
//! source. Destroying the source in response does not counter the ability.
//!
//! The engine's trigger dispatch used to gate half its arms on the source
//! still being on the battlefield and leave the other half ungated — the
//! split ran straight through matched pairs, so a creature's own
//! combat-damage trigger resolved after the creature died while a watcher's
//! did not. The gate is gone from the engine; these are the cards that had
//! re-implemented it themselves.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::state::StackEntry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Push a trigger for `source` directly onto the stack, the way the collector
/// would have, then remove the source and resolve.
fn resolve_after_source_dies(
    state: &mut mtg_engine::state::GameState,
    reg: &CardRegistry,
    source: mtg_engine::ids::ObjectId,
    event: TriggerEvent,
) {
    let card_id = state.get_object(source).unwrap().card_id;
    let controller = state.get_object(source).unwrap().controller;
    state.stack.push(StackEntry::Trigger(PendingTrigger::new(
        TriggerSource::new(source, card_id, controller, ""),
        event,
    )));
    state.move_object(source, Zone::Graveyard, reg);
    mtg_engine::triggers::resolve_next_trigger(state, reg);
}

// Rakish Heir: "Whenever a Vampire you control deals combat damage to a
// player, put a +1/+1 counter on that Vampire." Trading with a blocker in the
// same combat damage step must not cost the other Vampire its counter.
#[test]
fn rakish_heir_gives_its_counter_after_trading_in_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let heir = named_creature(&mut state, &reg, "Rakish Heir", P0);
    let other = named_creature(&mut state, &reg, "Stromkirk Noble", P0);

    resolve_after_source_dies(&mut state, &reg, heir,
        TriggerEvent::AnyCombatDamageToPlayer { dealer: other, damaged_player: P1, amount: 1 });

    assert_eq!(counters_of(&state, other, CounterType::PlusOnePlusOne), 1,
        "CR 113.7a: the Heir dying in the same combat damage step does not \
         counter its trigger, so the other Vampire still gets its counter");
}

// Balefire Dragon: "Whenever Balefire Dragon deals combat damage to a player,
// it deals that much damage to each creature that player controls."
#[test]
fn balefire_dragon_wipes_the_board_after_being_killed_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let dragon = named_creature(&mut state, &reg, "Balefire Dragon", P0);
    let victim = ready_creature(&mut state, P1, 2, 2);

    resolve_after_source_dies(&mut state, &reg, dragon,
        TriggerEvent::CombatDamageToPlayer { damaged_player: P1, amount: 6 });

    assert!(state.get_object(victim).is_none_or(|o| o.damage_marked >= 6),
        "CR 113.7a: killing the Dragon with its trigger on the stack must not \
         save the defending player's creatures");
}

// Curiosity: "Whenever enchanted creature deals damage to an opponent, you may
// draw a card." Destroying the Aura in response still offers the draw.
#[test]
fn curiosity_offers_its_draw_after_the_aura_is_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let bearer = ready_creature(&mut state, P0, 2, 2);
    let aura = named_creature(&mut state, &reg, "Curiosity", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(bearer);

    resolve_after_source_dies(&mut state, &reg, aura,
        TriggerEvent::AnyDamageToPlayer { dealer: bearer, damaged_player: P1, amount: 2 });

    assert!(state.awaiting_action.is_some(),
        "CR 113.7a: destroying Curiosity in response to its own trigger must \
         still present the 'you may draw a card' choice");
}

// Burning Vengeance: "Whenever you cast an instant or sorcery spell from your
// graveyard, Burning Vengeance deals 2 damage to any target."
#[test]
fn burning_vengeance_deals_its_damage_after_being_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let vengeance = named_creature(&mut state, &reg, "Burning Vengeance", P0);
    let spell = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);
    state.get_object_mut(spell).unwrap().cast_with_flashback = true;

    let card_id = state.get_object(vengeance).unwrap().card_id;
    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource {
            chosen_targets: vec![mtg_engine::actions::Target::Player(P1)],
            ..TriggerSource::new(vengeance, card_id, P0, "")
        },
        event: TriggerEvent::SpellCast { caster: P0, spell_id: spell },
    }));
    state.move_object(vengeance, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 18,
        "CR 113.7a: destroying Burning Vengeance in response still deals the 2 damage");
}

// Curse of the Bloody Tome: "At the beginning of enchanted player's upkeep,
// that player mills two cards." Destroying the Curse in response still mills —
// and the trigger still knows whom it cursed (CR 608.2, last known information).
#[test]
fn curse_of_the_bloody_tome_mills_after_the_curse_is_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Bloody Tome", P0, P1);
    for _ in 0..5 {
        let id = state.create_object(mtg_engine::ids::CardId(9999), P1, Zone::Library, None, None);
        state.get_player_mut(P1).library_order.push(id);
    }
    let before = state.objects_in_zone(Zone::Graveyard, P1).len();

    resolve_after_source_dies(&mut state, &reg, curse, TriggerEvent::Upkeep);

    assert_eq!(state.objects_in_zone(Zone::Graveyard, P1).len(), before + 2,
        "CR 113.7a/608.2: the Curse's mill still happens, and the trigger still \
         knows which player was cursed");
}

// Curse of Stalked Prey: "Whenever a creature deals combat damage to enchanted
// player, put a +1/+1 counter on that creature."
#[test]
fn curse_of_stalked_prey_gives_its_counter_after_the_curse_is_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of Stalked Prey", P0, P1);
    let attacker = ready_creature(&mut state, P0, 2, 2);

    resolve_after_source_dies(&mut state, &reg, curse,
        TriggerEvent::AnyCombatDamageToPlayer { dealer: attacker, damaged_player: P1, amount: 2 });

    assert_eq!(counters_of(&state, attacker, CounterType::PlusOnePlusOne), 1,
        "CR 113.7a/608.2: the counter is placed even though the Curse is gone");
}

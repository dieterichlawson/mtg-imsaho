//! Shared test helpers.

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::{CardId, ObjectId, PlayerId};
use mtg_engine::state::GameState;
use mtg_engine::types::*;

pub const P0: PlayerId = PlayerId(0);
pub const P1: PlayerId = PlayerId(1);

/// Set up a game state at a specific step with the given player as active and having priority.
pub fn game_at_step(step: Step, active: PlayerId) -> GameState {
    let mut state = GameState::new(2);
    state.step = step;
    state.active_player = active;
    state.priority_player = Some(active);
    state.is_first_turn = false;
    state.players[0].life = 20;
    state.players[1].life = 20;
    state
}

/// Place a creature on the battlefield that is ready to act (no summoning sickness).
pub fn ready_creature(state: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    let id = state.create_object(CardId(9999), owner, Zone::Battlefield, Some(power), Some(toughness));
    state.get_object_mut(id).unwrap().summoning_sick = false;
    id
}

/// Place a creature on the battlefield with summoning sickness.
pub fn sick_creature(state: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    state.create_object(CardId(9999), owner, Zone::Battlefield, Some(power), Some(toughness))
}

/// Put a named card into a player's hand. Returns the object ID.
pub fn spell_in_hand(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) -> ObjectId {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {}", name));
    let data = registry.card_data(card_id);
    let power = data.as_ref().and_then(|d| d.power);
    let toughness = data.as_ref().and_then(|d| d.toughness);
    let id = state.create_object(card_id, player, Zone::Hand, power, toughness);
    state.get_object_mut(id).unwrap().name = name.into();
    id
}

/// Add exactly enough mana to a player's pool to pay for a card by name.
pub fn add_mana_for(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {}", name));
    let data = registry.card_data(card_id)
        .unwrap_or_else(|| panic!("No card data for: {}", name));
    if let Some(ref cost) = data.cost {
        for sym in &cost.symbols {
            match sym {
                ManaSymbol::Colored(c) => {
                    let mana_type = match c {
                        Color::White => ManaType::White,
                        Color::Blue => ManaType::Blue,
                        Color::Black => ManaType::Black,
                        Color::Red => ManaType::Red,
                        Color::Green => ManaType::Green,
                    };
                    state.get_player_mut(player).mana_pool.add(mana_type, 1);
                }
                ManaSymbol::Generic(n) => {
                    state.get_player_mut(player).mana_pool.add(ManaType::Colorless, *n);
                }
                _ => {}
            }
        }
    }
}

/// Put a named card in hand and add mana to cast it. Returns the object ID.
pub fn castable_spell(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) -> ObjectId {
    let id = spell_in_hand(state, registry, name, player);
    add_mana_for(state, registry, name, player);
    id
}

/// Cast a spell targeting something, then resolve the top of the stack.
/// Returns the new state after resolution.
pub fn cast_and_resolve(
    state: &GameState,
    registry: &CardRegistry,
    spell_id: ObjectId,
    targets: Vec<Target>,
) -> GameState {
    let mut new_state = mtg_engine::engine::submit_action(
        state,
        &Action::CastSpell { object_id: spell_id, targets, sacrifice: None, exile_count: None },
        registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, registry);
    new_state
}

/// Place a named card on the battlefield, ready to act. Returns the object ID.
pub fn named_creature(
    state: &mut GameState,
    registry: &CardRegistry,
    name: &str,
    owner: PlayerId,
) -> ObjectId {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {}", name));
    let data = registry.card_data(card_id)
        .unwrap_or_else(|| panic!("No card data for: {}", name));
    let id = state.create_object(card_id, owner, Zone::Battlefield, data.power, data.toughness);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.summoning_sick = false;
    id
}

/// Place a named equipment card on the battlefield (unattached). Returns the object ID.
pub fn named_equipment(
    state: &mut GameState,
    registry: &CardRegistry,
    name: &str,
    owner: PlayerId,
) -> ObjectId {
    let id = named_creature(state, registry, name, owner);
    let obj = state.get_object_mut(id).unwrap();
    obj.is_equipment = true;
    id
}

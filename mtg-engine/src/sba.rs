use crate::cards::CardRegistry;
use crate::events::{GameEvent, LossReason};
use crate::state::{GameResult, GameState};
use crate::types::Zone;

/// Perform state-based actions without a registry (for backward compat with tests).
/// Uses raw P/T values only.
pub fn check_state_based_actions(state: &mut GameState) -> bool {
    check_state_based_actions_with_registry(state, None)
}

/// Perform state-based actions. Returns true if any were performed.
/// Per rule 704.3, this is called repeatedly until no actions are taken.
pub fn check_state_based_actions_with_registry(state: &mut GameState, registry: Option<&CardRegistry>) -> bool {
    let mut any_action = false;

    loop {
        let mut took_action = false;

        // Rule 704.5a: A player with 0 or less life loses the game.
        for i in 0..state.players.len() {
            let (lost, life, id) = {
                let p = &state.players[i];
                (p.lost, p.life, p.id)
            };
            if !lost && life <= 0 {
                state.players[i].lost = true;
                state.events.push(GameEvent::PlayerLost {
                    player: id,
                    reason: LossReason::LifeReachedZero,
                });
                took_action = true;
            }
        }

        // Rule 704.5b: A player who attempted to draw from an empty library loses.
        for i in 0..state.players.len() {
            let (lost, drawn_empty, id) = {
                let p = &state.players[i];
                (p.lost, p.has_drawn_from_empty, p.id)
            };
            if !lost && drawn_empty {
                state.players[i].lost = true;
                state.events.push(GameEvent::PlayerLost {
                    player: id,
                    reason: LossReason::DrewFromEmptyLibrary,
                });
                took_action = true;
            }
        }

        // Rule 704.5f: Creature with 0 or less toughness goes to graveyard.
        // Rule 704.5g: Creature with lethal damage is destroyed.
        // Rule 704.5h: Creature dealt damage by a deathtouch source is destroyed.
        let creature_ids: Vec<_> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
            .map(|o| o.id)
            .collect();

        let creatures_to_kill: Vec<_> = creature_ids.into_iter()
            .filter(|&id| {
                let effective_t = registry
                    .and_then(|r| state.effective_toughness(id, r))
                    .or_else(|| state.get_object(id).and_then(|o| o.toughness));
                let obj = state.get_object(id);
                let damage = obj.map(|o| o.damage_marked).unwrap_or(0);
                let deathtouch = obj.map(|o| o.dealt_deathtouch_damage).unwrap_or(false);
                match effective_t {
                    Some(t) => t <= 0 || (damage as i32) >= t || (deathtouch && damage > 0),
                    None => false,
                }
            })
            .collect();

        for id in creatures_to_kill {
            state.events.push(GameEvent::CreatureDied { object: id });
            state.move_object(id, Zone::Graveyard);
            state.creature_died_this_turn = true;
            took_action = true;
        }

        // Rule 704.5m: Aura not attached to anything goes to graveyard.
        let unattached_auras: Vec<_> = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.attached_to.is_some()
                    && {
                        let target_id = o.attached_to.unwrap();
                        state.get_object(target_id)
                            .map(|t| t.zone != Zone::Battlefield)
                            .unwrap_or(true) // target doesn't exist
                    }
            })
            .map(|o| o.id)
            .collect();

        for id in unattached_auras {
            state.move_object(id, Zone::Graveyard);
            took_action = true;
        }

        // Rule 704.5d: A token not on the battlefield ceases to exist.
        let dead_tokens: Vec<_> = state.objects.values()
            .filter(|o| o.is_token && o.zone != Zone::Battlefield)
            .map(|o| o.id)
            .collect();
        for id in dead_tokens {
            state.objects.remove(&id);
            took_action = true;
        }

        // Check for game end: only one (or zero) players alive.
        let alive: Vec<_> = state.players.iter().filter(|p| !p.lost).collect();
        if alive.len() <= 1 && state.result.is_none() {
            let result = if alive.len() == 1 {
                GameResult::Winner(alive[0].id)
            } else {
                GameResult::Draw
            };
            state.events.push(GameEvent::GameEnded { result: result.clone() });
            state.result = Some(result);
            took_action = true;
        }

        if !took_action {
            break;
        }
        any_action = true;
    }

    any_action
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CardId, PlayerId};

    #[test]
    fn creature_dies_from_lethal_damage() {
        let mut state = GameState::new(2);
        let id = state.create_object(CardId(1), PlayerId(0), Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(id).unwrap().damage_marked = 2;

        assert!(check_state_based_actions(&mut state));
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Graveyard);
    }

    #[test]
    fn creature_dies_from_zero_toughness() {
        let mut state = GameState::new(2);
        let id = state.create_object(CardId(1), PlayerId(0), Zone::Battlefield, Some(1), Some(0));

        assert!(check_state_based_actions(&mut state));
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Graveyard);
    }

    #[test]
    fn player_loses_at_zero_life() {
        let mut state = GameState::new(2);
        state.players[0].life = 0;

        assert!(check_state_based_actions(&mut state));
        assert!(state.players[0].lost);
        assert_eq!(state.result, Some(GameResult::Winner(PlayerId(1))));
    }

    #[test]
    fn player_loses_from_empty_library_draw() {
        let mut state = GameState::new(2);
        state.players[1].has_drawn_from_empty = true;

        assert!(check_state_based_actions(&mut state));
        assert!(state.players[1].lost);
    }

    #[test]
    fn no_action_when_everything_fine() {
        let mut state = GameState::new(2);
        state.create_object(CardId(1), PlayerId(0), Zone::Battlefield, Some(2), Some(3));

        assert!(!check_state_based_actions(&mut state));
    }
}

use crate::cards::CardRegistry;
use crate::events::{GameEvent, LossReason};
use crate::ids::ObjectId;
use crate::state::{GameResult, GameState, LogLevel};
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

        // Identify creatures that need to leave the battlefield.
        let creature_ids: Vec<_> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
            .map(|o| o.id)
            .collect();

        // Classify each creature: zero toughness vs lethal damage/deathtouch.
        let mut zero_toughness_ids = Vec::new();
        let mut destroyed_ids = Vec::new();

        for id in creature_ids {
            let effective_t = registry
                .and_then(|r| state.effective_toughness(id, r))
                .or_else(|| state.get_object(id).and_then(|o| o.toughness));
            let obj = state.get_object(id);
            let damage = obj.map(|o| o.damage_marked).unwrap_or(0);
            let deathtouch = obj.map(|o| o.dealt_deathtouch_damage).unwrap_or(false);
            match effective_t {
                Some(t) if t <= 0 => {
                    // Rule 704.5f: 0 or less toughness — not destruction,
                    // indestructible and regeneration do NOT prevent this.
                    zero_toughness_ids.push(id);
                }
                Some(t) if (damage as i32) >= t || (deathtouch && damage > 0) => {
                    // Rules 704.5g/h: lethal damage or deathtouch — destruction,
                    // checked via try_destroy (indestructible / regeneration apply).
                    destroyed_ids.push(id);
                }
                _ => {}
            }
        }

        // Rule 704.5f: zero toughness goes directly to graveyard.
        for id in zero_toughness_ids {
            let (cid, ctrl, damaged_by) = state.get_object(id)
                .map(|o| (o.card_id, o.controller, o.damaged_by.clone()))
                .unwrap_or((crate::ids::CardId(0), crate::ids::PlayerId(0), Vec::new()));
            let last_known_toughness = registry
                .and_then(|r| state.effective_toughness(id, r))
                .or_else(|| state.get_object(id).and_then(|o| o.toughness))
                .unwrap_or(0);
            state.events.push(GameEvent::CreatureDied { object: id, card_id: cid, controller: ctrl, damaged_by, last_known_toughness });
            state.move_object(id, Zone::Graveyard);
            state.creature_died_this_turn = true;
            took_action = true;
        }

        // Rules 704.5g/h: lethal damage or deathtouch — use destruction pipeline.
        for id in destroyed_ids {
            if let Some(reg) = registry {
                use crate::destruction::DestroyResult;
                // Full pipeline: indestructible check + regeneration.
                match crate::destruction::try_destroy(state, id, reg) {
                    DestroyResult::Died => {
                        took_action = true;
                    }
                    DestroyResult::Regenerated => {
                        // State changed (damage cleared, tapped), so next
                        // SBA pass won't re-identify this creature.
                        took_action = true;
                        continue;
                    }
                    DestroyResult::Indestructible => {
                        // No state change — don't set took_action to avoid
                        // infinite loop (creature still has lethal damage).
                        continue;
                    }
                }
            } else {
                // No registry (legacy tests): skip indestructible check,
                // but still handle regeneration shields inline.
                let shields = state.get_object(id).map(|o| o.regeneration_shields).unwrap_or(0);
                if shields > 0 {
                    crate::destruction::remove_from_combat(state, id);
                    if let Some(obj) = state.get_object_mut(id) {
                        obj.tapped = true;
                        obj.damage_marked = 0;
                        obj.dealt_deathtouch_damage = false; obj.damaged_by.clear();
                        obj.regeneration_shields -= 1;
                    }
                    state.log(LogLevel::Event, format!("{} regenerated",
                        state.get_object(id).map(|o| o.name.as_str()).unwrap_or("?")));
                    took_action = true;
                    continue;
                }
                let (cid, ctrl, damaged_by) = state.get_object(id)
                    .map(|o| (o.card_id, o.controller, o.damaged_by.clone()))
                    .unwrap_or((crate::ids::CardId(0), crate::ids::PlayerId(0), Vec::new()));
                let last_known_toughness = state.get_object(id).and_then(|o| o.toughness).unwrap_or(0);
                state.events.push(GameEvent::CreatureDied { object: id, card_id: cid, controller: ctrl, damaged_by, last_known_toughness });
                state.move_object(id, Zone::Graveyard);
                state.creature_died_this_turn = true;
                took_action = true;
            }
        }

        // Rule 704.5m: Aura not attached to anything goes to graveyard.
        // Curses attached to players (attached_to_player) are exempt.
        // Equipment stays on the battlefield when unattached (detaches instead).
        let unattached_auras: Vec<_> = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.attached_to.is_some()
                    && o.attached_to_player.is_none() // player-attached curses are fine
                    && !o.is_equipment // equipment stays on battlefield when unattached
                    && {
                        let target_id = o.attached_to.expect("aura must have attached_to");
                        state.get_object(target_id)
                            .map(|t| t.zone != Zone::Battlefield)
                            .unwrap_or(true) // target doesn't exist
                    }
            })
            .map(|o| o.id)
            .collect();

        // Equipment attached to creatures that left the battlefield: detach (don't destroy).
        let detach_equipment: Vec<ObjectId> = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.is_equipment
                    && o.attached_to.is_some()
                    && {
                        let target_id = o.attached_to.expect("equipment must have attached_to");
                        state.get_object(target_id)
                            .map(|t| t.zone != Zone::Battlefield)
                            .unwrap_or(true)
                    }
            })
            .map(|o| o.id)
            .collect();
        for id in detach_equipment {
            if let Some(obj) = state.get_object_mut(id) {
                obj.attached_to = None;
            }
            took_action = true;
        }

        for id in unattached_auras {
            state.move_object(id, Zone::Graveyard);
            took_action = true;
        }

        // Rule 704.5q: +1/+1 and -1/-1 counters annihilate in pairs.
        let counter_targets: Vec<_> = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && *o.counters.get(&crate::types::CounterType::PlusOnePlusOne).unwrap_or(&0) > 0
                    && *o.counters.get(&crate::types::CounterType::MinusOneMinusOne).unwrap_or(&0) > 0
            })
            .map(|o| o.id)
            .collect();
        for id in counter_targets {
            if let Some(obj) = state.objects.get_mut(&id) {
                let plus = *obj.counters.get(&crate::types::CounterType::PlusOnePlusOne).unwrap_or(&0);
                let minus = *obj.counters.get(&crate::types::CounterType::MinusOneMinusOne).unwrap_or(&0);
                let annihilate = plus.min(minus);
                *obj.counters.entry(crate::types::CounterType::PlusOnePlusOne).or_insert(0) -= annihilate;
                *obj.counters.entry(crate::types::CounterType::MinusOneMinusOne).or_insert(0) -= annihilate;
                took_action = true;
            }
        }

        // Rule 704.5i: A planeswalker with 0 or less loyalty goes to graveyard.
        let pw_zero_loyalty: Vec<_> = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.card_types.contains(&crate::types::CardType::Planeswalker)
                    && *o.counters.get(&crate::types::CounterType::Loyalty).unwrap_or(&0) == 0
            })
            .map(|o| o.id)
            .collect();
        // Also check via registry for non-token planeswalkers.
        let pw_zero_loyalty_registry: Vec<_> = if let Some(reg) = registry {
            state.objects.values()
                .filter(|o| {
                    o.zone == Zone::Battlefield
                        && !o.card_types.contains(&crate::types::CardType::Planeswalker)
                        && reg.card_data(o.card_id)
                            .map(|d| d.card_types.contains(&crate::types::CardType::Planeswalker))
                            .unwrap_or(false)
                        && *o.counters.get(&crate::types::CounterType::Loyalty).unwrap_or(&0) == 0
                })
                .map(|o| o.id)
                .collect()
        } else {
            vec![]
        };
        for id in pw_zero_loyalty.into_iter().chain(pw_zero_loyalty_registry) {
            state.log(LogLevel::Event, format!("{} has 0 loyalty and is put into graveyard",
                state.get_object(id).map(|o| o.name.as_str()).unwrap_or("?")));
            state.move_object(id, Zone::Graveyard);
            took_action = true;
        }

        // State-triggered abilities (CR 603.8): checked during SBA processing.
        // When the condition is met and the trigger is not already on the stack,
        // push a trigger onto pending_triggers so it goes on the stack and can
        // be responded to. The trigger won't fire again while it's on the stack.
        if let Some(reg) = registry {
            let garruk_card_id = reg.get_id_by_name("Garruk Relentless");
            if let Some(gid) = garruk_card_id {
                let garruk_to_trigger: Vec<_> = state.objects.values()
                    .filter(|o| {
                        o.zone == Zone::Battlefield
                            && o.card_id == gid
                            && !o.is_transformed
                            && !o.state_trigger_on_stack
                            && *o.counters.get(&crate::types::CounterType::Loyalty).unwrap_or(&0) <= 2
                    })
                    .map(|o| (o.id, o.controller))
                    .collect();
                for (id, controller) in garruk_to_trigger {
                    if let Some(obj) = state.get_object_mut(id) {
                        obj.state_trigger_on_stack = true;
                    }
                    state.pending_triggers.push(
                        crate::triggers::PendingTrigger::StateTriggered {
                            object_id: id,
                            card_id: gid,
                            controller,
                            description: "When Garruk Relentless has two or fewer loyalty counters on him, transform him".into(),
                        }
                    );
                    state.log(LogLevel::Event,
                        "Garruk Relentless's state-triggered ability triggers (transform)".into());
                    took_action = true;
                }
            }
        }

        // Rule 704.5k: Legend rule — if a player controls two or more legendary
        // permanents with the same name, all are put into the graveyard.
        // (Simplified: keep the newest, remove the rest.)
        {
            use std::collections::HashMap as Map;
            let mut legend_groups: Map<(crate::ids::PlayerId, String), Vec<crate::ids::ObjectId>> = Map::new();
            for obj in state.objects.values() {
                if obj.zone == Zone::Battlefield && obj.is_legendary {
                    legend_groups.entry((obj.controller, obj.name.clone()))
                        .or_default()
                        .push(obj.id);
                }
            }
            for (_, ids) in legend_groups {
                if ids.len() > 1 {
                    // Keep the first (oldest), remove the rest.
                    for &id in &ids[1..] {
                        state.move_object(id, Zone::Graveyard);
                        took_action = true;
                    }
                }
            }
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

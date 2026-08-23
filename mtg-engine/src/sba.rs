use crate::cards::CardRegistry;
use crate::events::{GameEvent, LossReason};
use crate::ids::ObjectId;
use crate::state::{GameResult, GameState, LogLevel};
use crate::types::Zone;

/// Perform state-based actions. Returns true if any were performed.
/// Per rule 704.3, this is called repeatedly until no actions are taken.
///
/// # Panics
/// Panics if an Aura object on the battlefield has no `attached_to` value, which
/// would indicate a malformed game state (Auras must always be attached to
/// something while on the battlefield).
pub fn check_state_based_actions(state: &mut GameState, registry: &CardRegistry) -> bool {
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
            .filter(|o| o.zone == Zone::Battlefield)
            .map(|o| o.id)
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|&id| state.is_creature(id, registry))
            .collect();

        // Classify each creature: zero toughness vs lethal damage/deathtouch.
        let mut zero_toughness_ids = Vec::new();
        let mut destroyed_ids = Vec::new();

        for id in creature_ids {
            let effective_t = state.effective_toughness(id, registry)
                .or_else(|| state.get_object(id).and_then(|o| o.toughness));
            let obj = state.get_object(id);
            let damage = obj.map_or(0, |o| o.damage_marked);
            let deathtouch = obj.is_some_and(|o| o.dealt_deathtouch_damage);
            // Skip creatures whose "enters as a copy" choice is still pending
            // (CR 614.1d) — their printed 0/0 P/T is replaced once the copy
            // resolves. The guard is the transient `entering_copy_source`
            // flag, armed at entry and cleared when the choice concludes
            // (success, decline, or no legal target). It is deliberately NOT
            // the static `enters_as_copy()` card property, which would leave
            // every such permanent permanently unkillable.
            let entering_copy = state.get_object(id).is_some_and(|o| o.entering_copy_source);
            match effective_t {
                Some(t) if t <= 0 && !entering_copy => {
                    // Rule 704.5f: 0 or less toughness — not destruction,
                    // indestructible and regeneration do NOT prevent this.
                    zero_toughness_ids.push(id);
                }
                Some(t) if !entering_copy && (i32::try_from(damage).unwrap_or(i32::MAX) >= t || (deathtouch && damage > 0)) => {
                    // Rules 704.5g/h: lethal damage or deathtouch — destruction,
                    // checked via try_destroy (indestructible / regeneration apply).
                    destroyed_ids.push(id);
                }
                _ => {}
            }
        }

        // Rule 704.5f: zero toughness goes directly to graveyard.
        for id in zero_toughness_ids {
            let (cid, ctrl, damaged_by, is_token) = state.get_object(id)
                .map_or((crate::ids::CardId(0), crate::ids::PlayerId(0), Vec::new(), false), |o| (o.card_id, o.controller, o.damaged_by.clone(), o.is_token));
            let last_known_toughness = state.effective_toughness(id, registry)
                .or_else(|| state.get_object(id).and_then(|o| o.toughness))
                .unwrap_or(0);
            state.events.push(GameEvent::CreatureDied { object: id, card_id: cid, controller: ctrl, damaged_by, last_known_toughness, is_token });
            // move_object handles the death log message.
            state.move_object(id, Zone::Graveyard, registry);
            state.creature_died_this_turn = true;
            took_action = true;
        }

        // Rules 704.5g/h: lethal damage or deathtouch — use destruction pipeline.
        // Per rule 704.3, SBAs happen simultaneously. We must snapshot indestructible
        // status BEFORE processing any deaths, so that e.g. Angelic Overseer retains
        // indestructible even if the Human granting it dies in the same SBA batch.
        let indestructible_snapshot: std::collections::HashSet<ObjectId> = destroyed_ids.iter()
            .filter(|&&id| state.has_keyword(id, crate::types::Keyword::Indestructible, registry))
            .copied()
            .collect();

        for id in destroyed_ids {
            // Check indestructible from the snapshot (before any deaths in this batch).
            if indestructible_snapshot.contains(&id) {
                continue;
            }
            // Still need regeneration check.
            let shields = state.get_object(id).map_or(0, |o| o.regeneration_shields);
            if shields > 0 {
                crate::destruction::regenerate_sba(state, id);
                took_action = true;
                continue;
            }
            crate::destruction::destroy_sba(state, id, registry);
            took_action = true;
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
                            .is_none_or(|t| t.zone != Zone::Battlefield) // target doesn't exist
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
                            .is_none_or(|t| t.zone != Zone::Battlefield)
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
            state.move_object(id, Zone::Graveyard, registry);
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

        // State-triggered abilities (CR 603.8): checked during SBA processing,
        // BEFORE zero-loyalty destruction. Per CR 603.8, state triggers fire as
        // soon as their condition is true — Garruk Relentless's "transform when
        // ≤2 loyalty" must trigger before the zero-loyalty SBA destroys him.
        {
            let candidates: Vec<(ObjectId, crate::ids::CardId, crate::ids::PlayerId)> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield && !o.state_trigger_on_stack)
                .map(|o| (o.id, o.card_id, o.controller))
                .collect();
            let mut triggered = None;
            for (id, card_id, controller) in candidates {
                let Some(behavior) = registry.get(card_id) else { continue };
                if behavior.state_trigger_condition(state, id, registry) {
                    triggered = Some((id, card_id, controller, behavior.state_trigger_description()));
                    break;
                }
            }
            if let Some((id, card_id, controller, description)) = triggered {
                if let Some(obj) = state.get_object_mut(id) {
                    obj.state_trigger_on_stack = true;
                }
                state.log(LogLevel::Event,
                    format!("{}'s state-triggered ability triggers", state.obj_name(id)));
                state.pending_triggers.push(
                    crate::triggers::PendingTrigger::StateTriggered {
                        object_id: id,
                        card_id,
                        controller,
                        description,
                    }
                );
                // Return immediately so the state trigger goes on the stack
                // before any further SBA processing (e.g. zero-loyalty death).
                return true;
            }
        }

        // Rule 704.5i: A planeswalker with 0 or less loyalty goes to graveyard.
        let pw_zero_loyalty: Vec<_> = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && *o.counters.get(&crate::types::CounterType::Loyalty).unwrap_or(&0) == 0
            })
            .map(|o| o.id)
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|&id| state.has_card_type(id, crate::types::CardType::Planeswalker, registry))
            .collect();
        for id in pw_zero_loyalty {
            state.log(LogLevel::Event, format!("{} has 0 loyalty and is put into graveyard",
                state.obj_name(id)));
            state.move_object(id, Zone::Graveyard, registry);
            took_action = true;
        }

        // Rule 704.5j: Legend rule — if a player controls two or more legendary
        // permanents with the same name, that player chooses one of them, and
        // the rest are put into their owners' graveyards.
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
            for ((player, name), ids) in legend_groups {
                if ids.len() > 1 {
                    // Player must choose which to keep.
                    let targets: Vec<crate::actions::Target> = ids.iter()
                        .map(|&id| crate::actions::Target::Object(id))
                        .collect();
                    crate::cards::helpers::present_target_choice(
                        state,
                        ids[0], // source (arbitrary, just for bookkeeping)
                        player,
                        targets,
                        crate::state::PendingEffect::LegendRuleKeep {
                            player,
                            legend_name: name.clone(),
                        },
                        &format!("Legend rule: choose which {name} to keep"),
                        false,
                    );
                    // Don't set took_action — we need to break out of the SBA
                    // loop and let the engine wait for the player's choice.
                    // The next SBA pass (after the choice is resolved) will
                    // find no more duplicates.
                    any_action = true;
                    return any_action;
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
    use crate::cards::CardRegistry;
    use crate::ids::{CardId, PlayerId};

    fn registry() -> CardRegistry {
        CardRegistry::with_all_cards()
    }

    #[test]
    fn creature_dies_from_lethal_damage() {
        let reg = registry();
        let mut state = GameState::new(2);
        let id = state.create_object(CardId(1), PlayerId(0), Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(id).unwrap().damage_marked = 2;

        assert!(check_state_based_actions(&mut state, &reg));
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Graveyard);
    }

    #[test]
    fn creature_dies_from_zero_toughness() {
        let reg = registry();
        let mut state = GameState::new(2);
        let id = state.create_object(CardId(1), PlayerId(0), Zone::Battlefield, Some(1), Some(0));

        assert!(check_state_based_actions(&mut state, &reg));
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Graveyard);
    }

    #[test]
    fn player_loses_at_zero_life() {
        let reg = registry();
        let mut state = GameState::new(2);
        state.players[0].life = 0;

        assert!(check_state_based_actions(&mut state, &reg));
        assert!(state.players[0].lost);
        assert_eq!(state.result, Some(GameResult::Winner(PlayerId(1))));
    }

    #[test]
    fn player_loses_from_empty_library_draw() {
        let reg = registry();
        let mut state = GameState::new(2);
        state.players[1].has_drawn_from_empty = true;

        assert!(check_state_based_actions(&mut state, &reg));
        assert!(state.players[1].lost);
    }

    #[test]
    fn no_action_when_everything_fine() {
        let reg = registry();
        let mut state = GameState::new(2);
        state.create_object(CardId(1), PlayerId(0), Zone::Battlefield, Some(2), Some(3));

        assert!(!check_state_based_actions(&mut state, &reg));
    }
}

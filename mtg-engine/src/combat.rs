
use crate::events::{GameEvent, DamageTarget};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{CombatState, GameState, LogLevel};
use crate::types::Zone;

/// Set up attackers. Validates and taps them.
pub fn declare_attackers(
    state: &mut GameState,
    attackers: &[(ObjectId, PlayerId)],
) {
    let mut combat = CombatState::new();

    for &(attacker_id, defending_player) in attackers {
        if let Some(obj) = state.get_object_mut(attacker_id) {
            // Tap the attacker (per default rules; some creatures have vigilance — future).
            obj.tapped = true;
            state.events.push(GameEvent::Tapped { object: attacker_id });
        }
        combat.attackers.insert(attacker_id, defending_player);
        combat.blocker_assignments.insert(attacker_id, Vec::new());
    }

    state.events.push(GameEvent::AttackersDeclared {
        attackers: attackers.to_vec(),
    });

    state.combat = Some(combat);
}

/// Set up blockers. Validates assignments.
pub fn declare_blockers(
    state: &mut GameState,
    assignments: &[(ObjectId, ObjectId)], // (blocker, attacker)
) {
    if let Some(combat) = &mut state.combat {
        for &(blocker_id, attacker_id) in assignments {
            if let Some(blockers) = combat.blocker_assignments.get_mut(&attacker_id) {
                blockers.push(blocker_id);
            }
        }
    }

    state.events.push(GameEvent::BlockersDeclared {
        assignments: assignments.to_vec(),
    });
}

/// Deal combat damage. All damage is simultaneous (no first strike in Phase 1).
pub fn deal_combat_damage(state: &mut GameState) {
    let combat = match &state.combat {
        Some(c) => c.clone(),
        None => return,
    };

    for (&attacker_id, &defending_player) in &combat.attackers {
        let attacker = match state.get_object(attacker_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => continue, // attacker may have been removed
        };
        let attacker_power = match attacker.power {
            Some(p) if p > 0 => p as u32,
            _ => continue,
        };

        let blockers = combat.blocker_assignments.get(&attacker_id)
            .cloned()
            .unwrap_or_default();

        if blockers.is_empty() {
            // Unblocked: deal damage to defending player.
            let old_life = state.get_player(defending_player).life;
            let new_life = old_life - attacker_power as i32;
            state.get_player_mut(defending_player).life = new_life;

            state.events.push(GameEvent::CombatDamageDealt {
                source: attacker_id,
                target: DamageTarget::Player(defending_player),
                amount: attacker_power,
            });
            state.events.push(GameEvent::LifeChanged {
                player: defending_player,
                old: old_life,
                new_life,
            });
            state.log(LogLevel::Event, format!("p{} took {} combat damage ({})", defending_player.0, attacker_power, new_life));
        } else {
            // Blocked: deal damage to first blocker (Phase 1: no damage ordering).
            // Attacker deals its power to the first blocker.
            // Each blocker deals its power to the attacker.
            for &blocker_id in &blockers {
                let blocker = match state.get_object(blocker_id) {
                    Some(o) if o.zone == Zone::Battlefield => o,
                    _ => continue,
                };
                let blocker_power = blocker.power.unwrap_or(0);

                // Attacker damages blocker.
                state.get_object_mut(blocker_id).unwrap().damage_marked += attacker_power;
                state.events.push(GameEvent::CombatDamageDealt {
                    source: attacker_id,
                    target: DamageTarget::Object(blocker_id),
                    amount: attacker_power,
                });

                // Blocker damages attacker.
                if blocker_power > 0 {
                    state.get_object_mut(attacker_id).unwrap().damage_marked += blocker_power as u32;
                    state.events.push(GameEvent::CombatDamageDealt {
                        source: blocker_id,
                        target: DamageTarget::Object(attacker_id),
                        amount: blocker_power as u32,
                    });
                }
            }
        }
    }
}

/// Clean up combat state at end of combat.
pub fn end_combat(state: &mut GameState) {
    state.combat = None;
}

/// Get all creatures a player controls that are eligible to attack.
pub fn eligible_attackers(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    state.objects.values()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && o.power.is_some()
                && !o.tapped
                && !o.summoning_sick
        })
        .map(|o| o.id)
        .collect()
}

/// Get all creatures a player controls that are eligible to attack,
/// also checking continuous effects (e.g., Pacifism).
pub fn eligible_attackers_with_registry(state: &GameState, player: PlayerId, registry: &crate::cards::CardRegistry) -> Vec<ObjectId> {
    eligible_attackers(state, player).into_iter()
        .filter(|&id| state.can_attack(id, registry))
        .collect()
}

/// Get all creatures a player controls that are eligible to block.
pub fn eligible_blockers(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    state.objects.values()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && o.power.is_some()
                && !o.tapped
        })
        .map(|o| o.id)
        .collect()
}

/// Get all creatures a player controls that are eligible to block,
/// also checking continuous effects (e.g., Pacifism).
pub fn eligible_blockers_with_registry(state: &GameState, player: PlayerId, registry: &crate::cards::CardRegistry) -> Vec<ObjectId> {
    eligible_blockers(state, player).into_iter()
        .filter(|&id| state.can_block(id, registry))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CardId;

    #[test]
    fn unblocked_attacker_deals_damage() {
        let mut state = GameState::new(2);
        let attacker = state.create_object(
            CardId(1), PlayerId(0), Zone::Battlefield, Some(3), Some(3),
        );
        state.get_object_mut(attacker).unwrap().summoning_sick = false;

        let defending = PlayerId(1);
        declare_attackers(&mut state, &[(attacker, defending)]);
        declare_blockers(&mut state, &[]);
        deal_combat_damage(&mut state);

        assert_eq!(state.get_player(defending).life, 40 - 3);
    }

    #[test]
    fn blocked_creature_trades() {
        let mut state = GameState::new(2);
        let attacker = state.create_object(
            CardId(1), PlayerId(0), Zone::Battlefield, Some(2), Some(2),
        );
        state.get_object_mut(attacker).unwrap().summoning_sick = false;

        let blocker = state.create_object(
            CardId(2), PlayerId(1), Zone::Battlefield, Some(2), Some(2),
        );

        let defending = PlayerId(1);
        declare_attackers(&mut state, &[(attacker, defending)]);
        declare_blockers(&mut state, &[(blocker, attacker)]);
        deal_combat_damage(&mut state);

        // Both should have lethal damage marked.
        assert_eq!(state.get_object(attacker).unwrap().damage_marked, 2);
        assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2);
        // Defending player takes no damage (attacker was blocked).
        assert_eq!(state.get_player(defending).life, 40);
    }

    #[test]
    fn eligible_attackers_excludes_sick_and_tapped() {
        let mut state = GameState::new(2);
        let p0 = PlayerId(0);

        // Ready to attack.
        let a = state.create_object(CardId(1), p0, Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(a).unwrap().summoning_sick = false;

        // Summoning sick — can't attack.
        state.create_object(CardId(1), p0, Zone::Battlefield, Some(2), Some(2));

        // Tapped — can't attack.
        let c = state.create_object(CardId(1), p0, Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(c).unwrap().summoning_sick = false;
        state.get_object_mut(c).unwrap().tapped = true;

        let eligible = eligible_attackers(&state, p0);
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0], a);
    }
}

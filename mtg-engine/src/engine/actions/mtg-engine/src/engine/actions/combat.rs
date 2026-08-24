//! Declaring attackers and blockers.

use super::super::Applied;
use crate::cards::SacrificeCost;
use crate::actions::{Action, Target};
use crate::cards::CardRegistry;
use crate::combat;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::mana;
use crate::stack;
use crate::state::{AwaitingAction, GameState, LogLevel};
use crate::triggers;
use crate::types::{Zone, CardType, Supertype, ManaCost, ManaSymbol, ContinuousEffect, Keyword, CounterType, Step, Color};
use super::super::*;

pub(crate) fn declare_attackers(state: &mut GameState, attackers: &[(ObjectId, PlayerId)], registry: &CardRegistry) -> Applied {
        // Validate declarations: only the active player's eligible
        // creatures (untapped, not summoning-sick without haste, no
        // defender/Pacifism — CR 508.1a) may attack, and only a valid
        // defender may be attacked. The engine is the authority; it does
        // not trust the submitted list. Illegal entries are dropped,
        // mirroring how blocker validation filters illegal blocks.
        let eligible = combat::eligible_attackers(&state, state.active_player, registry);
        let valid_defender = state.opponent(state.active_player);
        let attackers: Vec<(ObjectId, PlayerId)> = attackers.iter()
            .filter(|(id, def)| eligible.contains(id) && *def == valid_defender)
            .copied()
            .collect();
        let attackers = &attackers[..];
        if attackers.is_empty() {
            state.log(LogLevel::Debug, "No attackers declared".into());
        } else {
            let names: Vec<String> = attackers.iter()
                .map(|(id, _)| card_name(state, registry, *id))
                .collect();
            state.log(LogLevel::Event, format!("p{} declared attackers: {}", state.active_player.0, names.join(", ")));
        }
        combat::declare_attackers(&mut *state, attackers, registry);

        // Collect forced attackers (creatures with "attacks each combat if able" aura).
        let forced_ids: Vec<crate::ids::ObjectId> = {
            let active = state.active_player;
            let mut forced = Vec::new();
            for creature in state.objects.values() {
                if creature.zone != Zone::Battlefield || creature.controller != active
                    || !state.is_creature(creature.id, registry) || creature.tapped || creature.summoning_sick {
                    continue;
                }
                if state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&creature.id)) {
                    continue; // already attacking
                }
                // Check for Defender — can't be forced to attack.
                if state.has_keyword(creature.id, crate::types::Keyword::Defender, registry) {
                    continue;
                }
                // Respect "can't attack" effects (e.g. Bonds of Faith).
                if !state.can_attack(creature.id, registry) {
                    continue;
                }
                // Check for forced attack effects (e.g., Furor of the Bitten).
                let must_attack = state.has_continuous_effect(creature.id, &|e| {
                    match e {
                        crate::types::ContinuousEffect::ForceAttack { scope } => Some(scope),
                        _ => None,
                    }
                }, registry);
                if must_attack {
                    forced.push(creature.id);
                }
            }
            forced
        };

        // Add forced attackers to combat.
        if !forced_ids.is_empty() {
            let defending = state.opponent(state.active_player);
            if let Some(ref mut combat) = state.combat {
                for id in &forced_ids {
                    if !combat.attackers.contains_key(id) {
                        combat.attackers.insert(*id, defending);
                        combat.blocker_assignments.insert(*id, Vec::new());
                    }
                }
            }
            // Tap forced attackers (unless vigilance).
            for id in &forced_ids {
                let has_vig = state.has_keyword(*id, crate::types::Keyword::Vigilance, registry);
                if !has_vig {
                    if let Some(obj) = state.get_object_mut(*id) {
                        if !obj.tapped {
                            obj.tapped = true;
                        }
                    }
                }
            }
            let names: Vec<String> = forced_ids.iter()
                .map(|id| card_name(&state, registry, *id))
                .collect();
            state.log(LogLevel::Event, format!("Forced attackers: {}", names.join(", ")));
        }

        state.awaiting_action = None;
        state.consecutive_passes = 0;
    Applied::Continue
}

pub(crate) fn declare_blockers(state: &mut GameState, assignments: &[(ObjectId, ObjectId)], registry: &CardRegistry) -> Applied {
        // The defending player is the opponent of the active player.
        let defender = state.opponent(state.active_player);
        combat::declare_blockers_with_registry(&mut *state, assignments, registry);
        // Log after validation so only legal blocks appear in the log.
        let actual_blockers: Vec<(ObjectId, ObjectId)> = state.combat.as_ref()
            .map(|c| c.blocker_assignments.iter()
                .flat_map(|(&att, blockers)| blockers.iter().map(move |&b| (b, att)))
                .collect())
            .unwrap_or_default();
        if actual_blockers.is_empty() {
            state.log(LogLevel::Info, format!("p{} declared no blockers", defender.0));
        } else {
            let descs: Vec<String> = actual_blockers.iter()
                .map(|(b, a)| format!("{} blocks {}", card_name(state, registry, *b), card_name(state, registry, *a)))
                .collect();
            state.log(LogLevel::Event, format!("p{} declared blockers: {}", defender.0, descs.join(", ")));
        }
        state.awaiting_action = None;
        state.consecutive_passes = 0;
    Applied::Continue
}

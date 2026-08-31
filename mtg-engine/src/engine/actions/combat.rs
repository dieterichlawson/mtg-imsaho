//! Declaring attackers and blockers.

use super::super::Applied;
use crate::cards::CardRegistry;
use crate::combat;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
use super::super::*;

pub(crate) fn declare_attackers(state: &mut GameState, attackers: &[(ObjectId, PlayerId)], planeswalker_attacks: &[(ObjectId, ObjectId)], registry: &CardRegistry) -> Applied {
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
        // CR 508.1a: an attacker may instead be sent at a planeswalker the
        // defending player controls. Same authority rule: an entry naming an
        // ineligible attacker, a non-planeswalker, someone else's
        // planeswalker, or an attacker already attacking the player is
        // dropped, not trusted.
        let planeswalker_attacks: Vec<(ObjectId, ObjectId)> = planeswalker_attacks.iter()
            .filter(|(id, walker)| {
                eligible.contains(id)
                    && !attackers.iter().any(|(a, _)| a == id)
                    && state.get_object(*walker).is_some_and(|o|
                        o.zone == crate::types::Zone::Battlefield && o.controller == valid_defender)
                    && state.has_card_type(*walker, crate::types::CardType::Planeswalker, registry)
            })
            .copied()
            .collect();
        let attackers = &attackers[..];
        if attackers.is_empty() && planeswalker_attacks.is_empty() {
            state.log(LogLevel::Debug, "No attackers declared".into());
        } else {
            let mut names: Vec<String> = attackers.iter()
                .map(|(id, _)| card_name(state, registry, *id))
                .collect();
            names.extend(planeswalker_attacks.iter().map(|(id, walker)| format!(
                "{} -> {}", card_name(state, registry, *id), card_name(state, registry, *walker))));
            state.log(LogLevel::Event, format!("p{} declared attackers: {}", state.active_player.0, names.join(", ")));
        }
        combat::declare_attackers(&mut *state, attackers, &planeswalker_attacks, registry);

        // CR 508.1d: a creature that is required to attack and *able* to must
        // be among the declared attackers. "Able" is `eligible_attackers` —
        // the same list the declaration above was validated against, and the
        // same one `legal_actions` filters to build the prompt's `must_attack`.
        //
        // This used to roll its own eligibility check, and the copy had
        // drifted: it stopped at `summoning_sick` where `eligible_attackers`
        // asks `!summoning_sick || has_keyword(Haste)`. So a hasty creature
        // under Curse of the Nightly Hunt was listed in the prompt as having
        // to attack, and then allowed to stay home.
        let forced_ids: Vec<crate::ids::ObjectId> = eligible.iter()
            .copied()
            .filter(|&id| {
                !state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&id))
                    && state.must_attack(id, registry)
            })
            .collect();

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
            // A creature dragged into combat by a "must attack" effect attacked
            // this turn just the same (CR 508.1).
            let turn = state.turn_number;
            for id in &forced_ids {
                if let Some(obj) = state.get_object_mut(*id) {
                    obj.attacked_on_turn = Some(turn);
                }
            }
            // Tap forced attackers (unless vigilance).
            for id in &forced_ids {
                let has_vig = state.has_keyword(*id, crate::types::Keyword::Vigilance, registry);
                if !has_vig {
                    state.tap(*id);
                }
            }
            let names: Vec<String> = forced_ids.iter()
                .map(|id| card_name(&state, registry, *id))
                .collect();
            state.log(LogLevel::Event, format!("Forced attackers: {}", names.join(", ")));
            // These creatures were declared as attackers just the same
            // (CR 508.1d picks the declaration for the player), so the
            // CR 508.8 skip must not treat this combat as attacker-less.
            if let Some(ref mut combat) = state.combat {
                combat.any_attackers_declared = true;
            }
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

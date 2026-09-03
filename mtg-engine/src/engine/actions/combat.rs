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
        // The declaration is a choice of a SET of creatures (CR 508.1a/508.2):
        // a creature is attacking or it is not. A repeated entry used to sail
        // through and fire the creature's attack trigger once per repeat —
        // `0 0` on Kessig Cagebreakers doubled its wolves and won a game
        // (issue #108). De-duplicate here so no player type can submit one.
        let mut seen = std::collections::HashSet::new();
        let attackers: Vec<(ObjectId, PlayerId)> = attackers.iter()
            .filter(|(id, def)| eligible.contains(id) && *def == valid_defender && seen.insert(*id))
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
                    && state.get_object(*walker).is_some_and(|o|
                        o.zone == crate::types::Zone::Battlefield && o.controller == valid_defender)
                    && state.has_card_type(*walker, crate::types::CardType::Planeswalker, registry)
                    // The shared `seen` set drops an attacker already sent at
                    // the player AND a repeat within this list (issue #108).
                    && seen.insert(*id)
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
        //
        // Forced attackers are declared attackers — CR 508.1d picks the
        // declaration for the player — so they go through the same
        // declaration as the chosen ones: tapped, stamped, and in the
        // `AttackersDeclared` event. Inserting them into the combat maps
        // after the event had been pushed left them out of it, and a
        // forced Kessig Cagebreakers made no wolves (CR 508.1m).
        let already: Vec<ObjectId> = attackers.iter().map(|(id, _)| *id)
            .chain(planeswalker_attacks.iter().map(|(id, _)| *id))
            .collect();
        let forced_ids: Vec<ObjectId> = eligible.iter()
            .copied()
            .filter(|id| !already.contains(id) && state.must_attack(*id, registry))
            .collect();
        if !forced_ids.is_empty() {
            let names: Vec<String> = forced_ids.iter()
                .map(|id| card_name(&state, registry, *id))
                .collect();
            state.log(LogLevel::Event, format!("Forced attackers: {}", names.join(", ")));
        }
        let defending = state.opponent(state.active_player);
        let mut declared: Vec<(ObjectId, PlayerId)> = attackers.to_vec();
        declared.extend(forced_ids.iter().map(|&id| (id, defending)));
        combat::declare_attackers(&mut *state, &declared, &planeswalker_attacks, registry);

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

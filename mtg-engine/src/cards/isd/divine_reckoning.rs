use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Divine Reckoning — {2}{W}{W} Sorcery.
/// Each player chooses a creature they control. Destroy the rest.
/// Flashback {5}{W}{W}.
pub struct DivineReckoning;

impl CardBehavior for DivineReckoning {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Divine Reckoning".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Each player chooses a creature they control. Destroy the rest.\nFlashback {5}{W}{W}".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        // Collect players in turn order starting with the active player.
        let active = state.active_player;
        let mut player_order: Vec<PlayerId> = state.players.iter().map(|p| p.id).collect();
        // Rotate so active player is first.
        if let Some(pos) = player_order.iter().position(|&p| p == active) {
            player_order.rotate_left(pos);
        }

        // Filter to only players who control 2+ creatures (those with 0-1 are auto-handled).
        let mut kept: Vec<ObjectId> = Vec::new();
        let mut pending_players: Vec<PlayerId> = Vec::new();

        for &player_id in &player_order {
            let creatures: Vec<ObjectId> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield && o.controller == player_id && o.power.is_some())
                .map(|o| o.id)
                .collect();

            if creatures.len() <= 1 {
                // 0 or 1 creature: auto-keep (no choice needed).
                if let Some(&only) = creatures.first() {
                    kept.push(only);
                    let name = state.get_object(only).map(|o| o.name.clone()).unwrap_or_default();
                    state.log(LogLevel::Event, format!("Divine Reckoning: p{} keeps {} (only creature)", player_id.0, name));
                }
            } else {
                pending_players.push(player_id);
            }
        }

        // Clean up the spell first.
        state.move_spell_after_resolve(object_id);

        if pending_players.is_empty() {
            // All players had 0-1 creatures; destroy everything not kept.
            let all_creatures: Vec<ObjectId> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
                .map(|o| o.id)
                .collect();
            for cid in all_creatures {
                if !kept.contains(&cid) {
                    crate::destruction::try_destroy(state, cid, _registry);
                }
            }
        } else {
            // Present choice to the first pending player.
            let first_player = pending_players[0];
            let remaining = pending_players[1..].to_vec();

            let options: Vec<Target> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield && o.controller == first_player && o.power.is_some())
                .map(|o| Target::Object(o.id))
                .collect();

            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: first_player,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Divine Reckoning: choose a creature you control to keep".into(),
                    options,
                    optional: false,
                    effect: PendingEffect::KeepOneDestroyRest {
                        remaining_players: remaining,
                        kept_so_far: kept,
                        source_name: "Divine Reckoning".into(),
                    },
                },
            });
        }
    }
}

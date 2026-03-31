use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Charmbreaker Devils — {5}{R} 4/4 Devil.
/// At the beginning of your upkeep, return an instant or sorcery card at random
/// from your graveyard to your hand.
/// Whenever you cast an instant or sorcery spell, Charmbreaker Devils gets +4/+0
/// until end of turn.
pub struct CharmbreakerDevils;

impl CardBehavior for CharmbreakerDevils {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Charmbreaker Devils".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Devil".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.\nWhenever you cast an instant or sorcery spell, Charmbreaker Devils gets +4/+0 until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "return a random instant or sorcery from graveyard to hand".into(),
                },
            ],
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        // Find instant or sorcery cards in graveyard.
        let candidates: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Instant | CardType::Sorcery)))
                    .unwrap_or(false)
            })
            .map(|o| o.id)
            .collect();
        if let Some(&chosen) = candidates.first() {
            // "At random" — pick the first one found (deterministic fallback).
            let name = state.get_object(chosen).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(chosen, Zone::Hand);
            state.log(crate::state::LogLevel::Event,
                format!("Charmbreaker Devils: returned {} to hand", name));
        }
    }

    // Note: The "whenever you cast an instant or sorcery spell, +4/+0" trigger
    // requires a SpellCast watcher (not yet in the engine). This ability is
    // tracked as a known limitation until a SpellCast trigger system is added.
}

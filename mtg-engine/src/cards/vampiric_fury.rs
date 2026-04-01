use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{GameState, UntilEndOfTurnKeyword};
use crate::types::*;

/// Vampiric Fury — {1}{R} instant. Vampire creatures you control get +2/+0 and gain first strike until end of turn.
pub struct VampiricFury;

impl CardBehavior for VampiricFury {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Vampiric Fury".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Vampire creatures you control get +2/+0 and gain first strike until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // Find the controller of this spell.
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        // Build a registry to look up subtypes.

        // Collect vampire creature IDs controlled by this player.
        let vampire_ids: Vec<ObjectId> = state.objects.values()
            .filter(|obj| {
                obj.zone == Zone::Battlefield
                    && obj.controller == controller
                    && obj.power.is_some() // is a creature
            })
            .filter(|obj| {
                // Check if this creature has the "Vampire" subtype (registry or object).
                let registry_match = registry.card_data(obj.card_id)
                    .map(|data| data.subtypes.iter().any(|s| s == "Vampire"))
                    .unwrap_or(false);
                registry_match || obj.subtypes.iter().any(|s| s == "Vampire")
            })
            .map(|obj| obj.id)
            .collect();

        for id in vampire_ids {
            state.until_end_of_turn_effects.push(
                crate::state::UntilEndOfTurnEffect {
                    target: id,
                    power_mod: 2,
                    toughness_mod: 0,
                }
            );
            state.until_end_of_turn_keywords.push(
                UntilEndOfTurnKeyword {
                    target: id,
                    keyword: Keyword::FirstStrike,
                }
            );
        }

        state.move_spell_after_resolve(object_id);
    }
}

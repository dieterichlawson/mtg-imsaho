use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::*;

/// Harvest Pyre — {1}{R} Instant.
/// As an additional cost to cast Harvest Pyre, exile any number of cards from your graveyard.
/// Harvest Pyre deals damage to target creature equal to the number of cards exiled this way.
pub struct HarvestPyre;

impl CardBehavior for HarvestPyre {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Harvest Pyre".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As an additional cost to cast Harvest Pyre, exile any number of cards from your graveyard.\nHarvest Pyre deals damage to target creature equal to the number of cards exiled this way.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None, // Engine doesn't enforce this yet; handled in on_resolve.
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Exile all cards from the caster's graveyard (maximize damage).
        // "Any number" means we exile as many as possible for maximum effect.
        let graveyard_cards: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.id != object_id)
            .map(|o| o.id)
            .collect();

        let count = graveyard_cards.len() as u32;
        for card_id in &graveyard_cards {
            state.move_object(*card_id, Zone::Exile);
        }

        if count > 0 {
            state.log(LogLevel::Event, format!("Harvest Pyre exiled {} cards from graveyard", count));

            // Deal damage to the target creature.
            if let Some(Target::Object(target_id)) = targets.first() {
                if let Some(obj) = state.get_object_mut(*target_id) {
                    if obj.zone == Zone::Battlefield {
                        obj.damage_marked += count;
                        let name = obj.name.clone();
                        state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                            source: object_id,
                            target: crate::events::DamageTarget::Object(*target_id),
                            amount: count,
                        });
                        state.log(LogLevel::Event, format!("Harvest Pyre dealt {} damage to {}", count, name));
                    }
                }
            }
        } else {
            state.log(LogLevel::Event, "Harvest Pyre: no cards in graveyard to exile".into());
        }

        state.move_spell_after_resolve(object_id);
    }
}

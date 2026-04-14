use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Harvest Pyre — {1}{R} Instant.
/// As an additional cost to cast this spell, exile X cards from your graveyard.
/// Harvest Pyre deals X damage to target creature.
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
            oracle_text: "As an additional cost to cast this spell, exile X cards from your graveyard.\nHarvest Pyre deals X damage to target creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(crate::cards::AdditionalCost::ExileXFromGraveyard),
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // The exile happened at cast time (additional cost). Read the count.
        let count = state.get_object(object_id)
            .and_then(|o| o.card_state.get("exile_count").copied())
            .map_or(0, |id| u32::try_from(id.0).unwrap_or(u32::MAX));

        if count > 0 {
            if let Some(Target::Object(target_id)) = targets.first() {
                if let Some(obj) = state.get_object_mut(*target_id) {
                    if obj.zone == Zone::Battlefield {
                        obj.damage_marked += count;
                        obj.damaged_by.push(object_id);
                    }
                }
                if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                    state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                        source: object_id,
                        target: crate::events::DamageTarget::Object(*target_id),
                        amount: count,
                    });
                    state.log(LogLevel::Event, format!("Harvest Pyre dealt {} damage to {}", count, state.obj_name(*target_id)));
                }
            }
        }

        state.move_spell_after_resolve(object_id, registry);
    }
}

use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::*;

/// Corpse Lunge — {2}{B} Instant.
/// As an additional cost to cast Corpse Lunge, exile a creature card from your graveyard.
/// Corpse Lunge deals damage equal to the exiled card's power to target creature.
pub struct CorpseLunge;

impl CardBehavior for CorpseLunge {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Corpse Lunge".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As an additional cost to cast Corpse Lunge, exile a creature card from your graveyard.\nCorpse Lunge deals damage equal to the exiled card's power to target creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        // The creature was exiled at cast time (additional cost). Read the stored power.
        let power = state.get_object(object_id)
            .and_then(|o| o.card_state.get("exiled_power").copied())
            .map(|id| id.0 as i32)
            .unwrap_or(0);

        {
            let damage = power.max(0) as u32;
            if damage > 0 {
                // Deal damage to the target creature.
                if let Some(Target::Object(target_id)) = targets.first() {
                    if let Some(obj) = state.get_object_mut(*target_id) {
                        if obj.zone == Zone::Battlefield {
                            obj.damage_marked += damage;
                            obj.damaged_by.push(object_id);
                            let name = obj.name.clone();
                            state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                                source: object_id,
                                target: crate::events::DamageTarget::Object(*target_id),
                                amount: damage,
                            });
                            state.log(LogLevel::Event, format!("Corpse Lunge dealt {} damage to {}", damage, name));
                        }
                    }
                }
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

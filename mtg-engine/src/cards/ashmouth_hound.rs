use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Ashmouth Hound — {1}{R} 2/1 Elemental Hound.
/// Whenever Ashmouth Hound blocks or becomes blocked by a creature,
/// Ashmouth Hound deals 1 damage to that creature.
pub struct AshmouthHound;

impl CardBehavior for AshmouthHound {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ashmouth Hound".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Elemental".into(), "Hound".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Whenever Ashmouth Hound blocks or becomes blocked by a creature, Ashmouth Hound deals 1 damage to that creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Blocks,
                    description: "deal 1 damage to blocked/blocking creature".into(),
                },
            ],
        }
    }

    fn on_blocks(&self, state: &mut GameState, self_id: ObjectId, blocked_attacker: ObjectId, _registry: &CardRegistry) {
        // Deal 1 damage to the creature Ashmouth Hound is blocking.
        if let Some(obj) = state.get_object_mut(blocked_attacker) {
            if obj.zone == Zone::Battlefield {
                obj.damage_marked += 1;
                obj.damaged_by.push(self_id);
                let name = obj.name.clone();
                state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                    source: self_id,
                    target: crate::events::DamageTarget::Object(blocked_attacker),
                    amount: 1,
                });
                state.log(crate::state::LogLevel::Event,
                    format!("Ashmouth Hound deals 1 damage to {}", name));
            }
        }
    }
}

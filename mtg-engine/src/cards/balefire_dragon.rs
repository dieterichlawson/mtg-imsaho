use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Balefire Dragon — {5}{R}{R} 6/6 Dragon. Flying.
/// Whenever Balefire Dragon deals combat damage to a player, it deals that much damage
/// to each creature that player controls.
pub struct BalefireDragon;

impl CardBehavior for BalefireDragon {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Balefire Dragon".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Dragon".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Flying\nWhenever Balefire Dragon deals combat damage to a player, it deals that much damage to each creature that player controls.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "deal that much damage to each creature that player controls".into(),
                },
            ],
        }
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, damaged_player: PlayerId, amount: u32, _registry: &CardRegistry) {
        if !state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            return;
        }

        // Collect all creatures the damaged player controls
        let creatures: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == damaged_player)
            .map(|o| o.id)
            .collect();

        // Deal that much damage to each of those creatures.
        // This is triggered ability damage, NOT combat damage.
        for creature_id in creatures {
            if let Some(obj) = state.get_object_mut(creature_id) {
                obj.damage_marked += amount;
                obj.damaged_by.push(self_id);
            }
            state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                source: self_id,
                target: crate::events::DamageTarget::Object(creature_id),
                amount,
            });
        }
    }
}

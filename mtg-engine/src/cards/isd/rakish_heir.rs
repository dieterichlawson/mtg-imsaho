use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};

/// Rakish Heir — {2}{R} 2/2 Vampire.
/// Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on that Vampire.
pub struct RakishHeir;

impl CardBehavior for RakishHeir {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rakish Heir".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCombatDamageToPlayer,
                    description: "put a +1/+1 counter on that Vampire".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn on_any_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, source_id: ObjectId, _damaged_player: PlayerId, _amount: u32, registry: &CardRegistry) {
        // Whenever a Vampire YOU control deals combat damage to a player.
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Check if the source creature is a Vampire we control.
        let source = match state.get_object(source_id) {
            Some(o) if o.zone == Zone::Battlefield && o.controller == controller => o,
            _ => return,
        };
        let is_vampire = registry.card_data(source.card_id)
            .is_some_and(|d| d.subtypes.iter().any(|s| s == "Vampire"))
            || source.subtypes.iter().any(|s| s == "Vampire");
        if is_vampire {
            // Put a +1/+1 counter on THAT Vampire (the source).
            state.add_counters(source_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

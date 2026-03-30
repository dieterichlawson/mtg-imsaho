use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Crossway Vampire — 3/2 for {1}{R}{R}. Vampire.
/// When Crossway Vampire enters the battlefield, target creature can't block this turn.
pub struct CrosswayVampire;

impl CardBehavior for CrosswayVampire {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Crossway Vampire".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "When Crossway Vampire enters the battlefield, target creature can't block this turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "target creature can't block this turn".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        // Auto-target opponent's strongest creature.
        let target = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller != controller && o.power.is_some() && o.id != object_id)
            .max_by_key(|o| o.power.unwrap_or(0))
            .map(|o| o.id);
        if let Some(target_id) = target {
            state.until_end_of_turn_cant_block.push(target_id);
            let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("{} can't block this turn (Crossway Vampire)", name));
        }
    }
}

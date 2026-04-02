use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Village Bell-Ringer — {2}{W} 1/4 Human Scout. Flash. When this creature enters, untap all creatures you control.
pub struct VillageBellRinger;

impl CardBehavior for VillageBellRinger {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Village Bell-Ringer".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Scout".into()],
            power: Some(1),
            toughness: Some(4),
            oracle_text: "Flash (You may cast this spell any time you could cast an instant.)\nWhen this creature enters, untap all creatures you control.".into(),
            keywords: vec![Keyword::Flash],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "untap all creatures you control".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let to_untap: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == controller && o.power.is_some() && o.tapped)
            .map(|o| o.id)
            .collect();
        for id in to_untap {
            state.get_object_mut(id).unwrap().tapped = false;
        }
    }
}

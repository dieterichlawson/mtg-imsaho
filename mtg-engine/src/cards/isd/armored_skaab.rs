use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Armored Skaab — 1/4 for {2}{U}. Zombie Warrior.
/// When this creature enters, mill four cards.
pub struct ArmoredSkaab;

impl CardBehavior for ArmoredSkaab {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Armored Skaab".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Warrior".into()],
            power: Some(1),
            toughness: Some(4),
            oracle_text: "When this creature enters, mill four cards.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "mill four cards".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        crate::engine::mill_cards(state, controller, 4);
        state.log(crate::state::LogLevel::Event,
            "Armored Skaab enters — milled 4 cards".to_string());
    }
}

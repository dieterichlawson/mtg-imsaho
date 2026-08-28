use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};
use crate::actions::Target;

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
            subtypes: vec!["Zombie".into(), "Warrior".into()],
            power: Some(1),
            toughness: Some(4),
            oracle_text: "When this creature enters, mill four cards.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "mill four cards".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        crate::engine::mill_cards(state, controller, 4, registry);
        state.log(crate::state::LogLevel::Event,
            "Armored Skaab enters — milled 4 cards".to_string());
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};
use crate::actions::Target;

/// Doomed Traveler — {W} 1/1 Human Soldier. When it dies, create a 1/1 white Spirit token with flying.
pub struct DoomedTraveler;

impl CardBehavior for DoomedTraveler {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Doomed Traveler".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "When this creature dies, create a 1/1 white Spirit creature token with flying.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "create a 1/1 white Spirit token with flying".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);
        state.create_token_with_subtypes("", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()], registry);
        state.log(crate::state::LogLevel::Event,
            "Doomed Traveler: created a 1/1 white Spirit token with flying".into());
    }
}

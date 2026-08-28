use crate::cards::helpers;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};
use crate::actions::Target;

/// Tormented Pariah {3}{R} 3/2 Human Warrior Werewolf // Rampaging Werewolf 6/4 Werewolf
pub struct TormentedPariah;


impl CardBehavior for TormentedPariah {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Tormented Pariah".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Warrior".into(), "Werewolf".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "At the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Rampaging Werewolf".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Werewolf".into()],
            power: Some(6),
            toughness: Some(4),
            // CR 204.2: the back face has no mana cost, so its color is the
            // indicator printed beside its type line — red.
            color_indicator: vec![Color::Red],
            oracle_text: "At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform back if 2+ spells cast".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        })
    }

    fn should_trigger(&self, state: &GameState, self_id: ObjectId, kind: &TriggerKind, registry: &CardRegistry) -> bool {
        helpers::werewolf_should_trigger(self, state, self_id, kind, registry)
    }

    fn should_transform(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
        helpers::werewolf_should_transform(state, object_id)
    }


    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        helpers::werewolf_on_upkeep(self, state, self_id, registry);
    }
}

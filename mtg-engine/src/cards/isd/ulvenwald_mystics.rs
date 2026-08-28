use crate::actions::Target;
use crate::cards::helpers;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Ulvenwald Mystics {2}{G}{G} 3/3 Human Shaman Werewolf
/// // Ulvenwald Primordials 5/5 Werewolf with {G}: Regenerate
pub struct UlvenwaldMystics;


impl CardBehavior for UlvenwaldMystics {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ulvenwald Mystics".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Shaman".into(), "Werewolf".into()],
            power: Some(3),
            toughness: Some(3),
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
            name: "Ulvenwald Primordials".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Werewolf".into()],
            power: Some(5),
            toughness: Some(5),
            // CR 204.2: the back face has no mana cost, so its color is the
            // indicator printed beside its type line — green.
            color_indicator: vec![Color::Green],
            oracle_text: "{G}: Regenerate this creature.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.".into(),
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


    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // The regenerate ability belongs to Ulvenwald Primordials, the back
        // face, so it is offered only while transformed.
        if !state.get_object(object_id)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.is_transformed)
        {
            return vec![];
        }
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{G}: Regenerate".into(),
            cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        state.add_regeneration_shield(object_id);
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        helpers::werewolf_on_upkeep(self, state, self_id, registry);
    }
}

use crate::cards::helpers;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectScope, Zone};
use crate::actions::Target;

/// Hanweir Watchkeep {2}{R} 1/5 Human Warrior with Defender // Bane of Hanweir 5/5 Werewolf that attacks each combat
pub struct HanweirWatchkeep;


impl CardBehavior for HanweirWatchkeep {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Hanweir Watchkeep".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Warrior".into(), "Werewolf".into()],
            power: Some(1),
            toughness: Some(5),
            oracle_text: "Defender\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
            keywords: vec![Keyword::Defender],
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
            name: "Bane of Hanweir".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Werewolf".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "This creature attacks each combat if able.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.".into(),
            continuous_effects: vec![
                ContinuousEffect::ForceAttack { scope: EffectScope::OnSelf },
            ],
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
        if state.get_object(self_id).is_none_or(|o| o.zone != Zone::Battlefield) {
            return;
        }
        if self.should_transform(state, self_id, registry) {
            let old_name = state.get_object(self_id).map(|o| o.name.clone()).unwrap_or_default();
            helpers::apply_transform(state, self_id, registry);
            let new_name = state.get_object(self_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("{old_name} transforms into {new_name}"));
        }
    }
}

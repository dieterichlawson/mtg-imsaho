use crate::cards::helpers;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};
use crate::actions::Target;

/// Hollowhenge Scavenger — 4/5 for {3}{G}{G}. Elemental.
/// Morbid — When Hollowhenge Scavenger enters the battlefield, if a creature died this turn,
/// you gain 5 life.
pub struct HollowhengeScavenger;

impl CardBehavior for HollowhengeScavenger {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Hollowhenge Scavenger".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Elemental".into()],
            power: Some(4),
            toughness: Some(5),
            oracle_text: "Morbid — When this creature enters, if a creature died this turn, you gain 5 life.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "if morbid, gain 5 life".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn should_trigger(&self, state: &GameState, _self_id: ObjectId, kind: &TriggerKind, _registry: &CardRegistry) -> bool {
        helpers::morbid_should_trigger(state, kind)
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], _registry: &CardRegistry) {
        if state.creature_died_this_turn {
            let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);
            state.change_life(controller, 5);
            state.log(crate::state::LogLevel::Event,
                "Hollowhenge Scavenger enters with morbid — gained 5 life".to_string());
        }
    }
}

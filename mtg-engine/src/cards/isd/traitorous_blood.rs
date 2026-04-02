use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, UntilEndOfTurnKeyword};
use crate::types::*;

/// Traitorous Blood — {1}{R}{R} Sorcery.
/// Gain control of target creature until end of turn. Untap it.
/// It gains trample and haste until end of turn.
pub struct TraiterousBlood;

impl CardBehavior for TraiterousBlood {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Traitorous Blood".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if state.get_object(*creature_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
                // Save original controller for revert at end of turn.
                let original = state.get_object(*creature_id).map(|o| o.controller).unwrap_or(PlayerId(0));
                state.until_end_of_turn_control_changes.push((*creature_id, original));
                // Change controller and untap.
                if let Some(obj) = state.get_object_mut(*creature_id) {
                    obj.controller = controller;
                    obj.tapped = false;
                }
                // Grant haste and trample.
                state.until_end_of_turn_keywords.push(UntilEndOfTurnKeyword {
                    target: *creature_id,
                    keyword: Keyword::Haste,
                });
                state.until_end_of_turn_keywords.push(UntilEndOfTurnKeyword {
                    target: *creature_id,
                    keyword: Keyword::Trample,
                });
                let name = state.get_object(*creature_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("Traitorous Blood steals {}, untaps it, grants haste and trample", name));
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

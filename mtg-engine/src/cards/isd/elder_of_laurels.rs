use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Elder of Laurels — {2}{G} 2/3 Human Advisor.
/// {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
pub struct ElderOfLaurels;

impl CardBehavior for ElderOfLaurels {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Elder of Laurels".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Advisor".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "{3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{3}{G}: Target creature gets +X/+X where X is creatures you control".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(3),
                    ManaSymbol::Colored(Color::Green),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::Creature),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let creature_count = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.power.is_some())
            .count() as i32;

        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                state.until_end_of_turn.push(
                    crate::state::TemporaryEffect::ModifyPT {
                        target: *target_id,
                        power_mod: creature_count,
                        toughness_mod: creature_count,
                    }
                );
                state.log(crate::state::LogLevel::Event,
                    format!("Elder of Laurels gives target creature +{}/+{}", creature_count, creature_count));
            }
        }
    }
}

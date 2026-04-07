use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Kessig Wolf Run — Land.
/// {T}: Add {C}.
/// {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
pub struct KessigWolfRun;

impl CardBehavior for KessigWolfRun {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Kessig Wolf Run".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {C}.\n{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ManaAbilityDef {
                ability_index: 0,
                description: "Add {C}".into(),
                produced: vec![(ManaType::Colorless, 1)],
                requires_tap: true,
            }]
        } else {
            vec![]
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ActivatedAbilityDef {
                ability_index: 1,
                description: "{X}{R}{G}, {T}: Target creature gets +X/+0 and trample until EOT".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::X,
                    ManaSymbol::Colored(Color::Red),
                    ManaSymbol::Colored(Color::Green),
                ]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::Creature),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let x = state.last_activated_x_value.unwrap_or(0) as i32;
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                // Grant +X/+0 until end of turn.
                state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
                    target: *target_id,
                    power_mod: x,
                    toughness_mod: 0,
                });
                // Grant trample until end of turn.
                state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantKeyword {
                    target: *target_id,
                    keyword: Keyword::Trample,
                });
                let name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("Kessig Wolf Run gives {} +{}/+0 and trample until end of turn", name, x));
            }
        }
    }
}

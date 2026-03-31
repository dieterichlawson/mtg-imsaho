use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Kessig Wolf Run — Land.
/// {T}: Add {C}.
/// {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
///
/// Simplified: Since the engine doesn't yet support choosing X for activated abilities,
/// the ability costs {1}{R}{G} and gives +1/+0 and trample. Players can activate
/// it multiple times for larger boosts.
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
            oracle_text: "{T}: Add {C}.\n{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn. (Simplified: {1}{R}{G} for +1/+0)".into(),
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

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ActivatedAbilityDef {
                ability_index: 1,
                description: "{1}{R}{G}, {T}: Target creature gets +1/+0 and trample until EOT (simplified X=1)".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(1),
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

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                // Grant +1/+0 until end of turn.
                state.until_end_of_turn_effects.push(crate::state::UntilEndOfTurnEffect {
                    target: *target_id,
                    power_mod: 1,
                    toughness_mod: 0,
                });
                // Grant trample until end of turn.
                state.until_end_of_turn_keywords.push(crate::state::UntilEndOfTurnKeyword {
                    target: *target_id,
                    keyword: Keyword::Trample,
                });
                let name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("Kessig Wolf Run gives {} +1/+0 and trample until end of turn", name));
            }
        }
    }
}

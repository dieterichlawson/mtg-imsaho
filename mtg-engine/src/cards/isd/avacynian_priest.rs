use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Avacynian Priest — {1}{W} 1/2 Human Cleric. {1}, {T}: Tap target non-Human creature.
pub struct AvacynianPriest;

impl CardBehavior for AvacynianPriest {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Avacynian Priest".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "{1}, {T}: Tap target non-Human creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}, {T}: Tap target non-Human creature".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(1),
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

    /// Filter targets to exclude Human creatures.
    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| {
                        let is_human = registry.card_data(o.card_id)
                            .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                            .unwrap_or(false)
                            || o.subtypes.iter().any(|s| s == "Human");
                        o.zone == Zone::Battlefield
                            && o.power.is_some()
                            && !is_human
                    })
                    .unwrap_or(false)
            }
            Target::Player(_) => false,
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        for target in targets {
            if let Target::Object(target_id) = target {
                if let Some(obj) = state.get_object_mut(*target_id) {
                    obj.tapped = true;
                }
            }
        }
    }
}

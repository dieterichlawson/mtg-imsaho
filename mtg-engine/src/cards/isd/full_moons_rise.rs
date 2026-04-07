use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Full Moon's Rise — {1}{G} Enchantment.
/// Werewolf and Wolf creatures you control get +1/+0 and have trample.
/// Sacrifice Full Moon's Rise: Regenerate all Werewolf and Wolf creatures you control.
pub struct FullMoonsRise;

impl CardBehavior for FullMoonsRise {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Full Moon's Rise".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Werewolf creatures you control get +1/+0 and have trample.\nSacrifice this enchantment: Regenerate all Werewolf creatures you control.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 1,
                    toughness: 0,
                    scope: EffectScope::Global(CreatureFilter::And(vec![
                        CreatureFilter::ControlledByYou,
                        CreatureFilter::HasSubtype("Werewolf".into()),
                    ])),
                },
                ContinuousEffect::GrantKeyword {
                    keyword: Keyword::Trample,
                    scope: EffectScope::Global(CreatureFilter::And(vec![
                        CreatureFilter::ControlledByYou,
                        CreatureFilter::HasSubtype("Werewolf".into()),
                    ])),
                },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Sacrifice: Regenerate all Wolf and Werewolf creatures you control".into(),
                cost: ManaCost::free(),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::SacrificeThis,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Regenerate all Wolf and Werewolf creatures you control.
        let wolves_and_werewolves: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| {
                o.power.is_some() && {
                    let subtypes = registry.card_data(o.card_id)
                        .map(|d| d.subtypes.clone())
                        .unwrap_or_default();
                    let all_subtypes: Vec<&str> = o.subtypes.iter().map(|s| s.as_str())
                        .chain(subtypes.iter().map(|s| s.as_str()))
                        .collect();
                    all_subtypes.iter().any(|&s| s == "Werewolf")
                }
            })
            .map(|o| o.id)
            .collect();

        for cid in wolves_and_werewolves {
            if let Some(obj) = state.get_object_mut(cid) {
                obj.regeneration_shields += 1;
            }
        }
        state.log(crate::state::LogLevel::Event,
            "Full Moon's Rise: all Werewolf creatures gain regeneration".into());
    }
}

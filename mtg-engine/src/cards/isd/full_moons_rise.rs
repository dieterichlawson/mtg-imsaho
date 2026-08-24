use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope, CreatureFilter, Keyword, Zone};

/// Full Moon's Rise — {1}{G} Enchantment.
/// Werewolf creatures you control get +1/+0 and have trample.
/// Sacrifice Full Moon's Rise: Regenerate all Werewolf creatures you control.
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
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Sacrifice: Regenerate all Werewolf creatures you control".into(),
                cost: ManaCost::free(),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::SacrificeThis,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        let werewolves: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            // `registry.card_data` reads the front face only; `has_subtype`
            // reads the active one and unions in runtime-granted subtypes.
            .filter(|o| o.power.is_some() && state.has_subtype(o.id, "Werewolf", registry))
            .map(|o| o.id)
            .collect();

        for cid in werewolves {
            if let Some(obj) = state.get_object_mut(cid) {
                obj.regeneration_shields += 1;
            }
        }
        state.log(crate::state::LogLevel::Event,
            "Full Moon's Rise: all Werewolf creatures gain regeneration".into());
    }
}

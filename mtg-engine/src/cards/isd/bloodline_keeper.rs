use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Bloodline Keeper {2}{B}{B} 3/3 Vampire // Lord of Lineage 5/5 Vampire.
/// {T}: Create a 2/2 black Vampire creature token with flying.
/// {B}: Transform Bloodline Keeper. Activate only if you control five or more Vampires.
/// Lord of Lineage: Other Vampire creatures you control get +2/+2.
/// {T}: Create a 2/2 black Vampire creature token with flying.
pub struct BloodlineKeeper;

impl BloodlineKeeper {
    fn count_vampires(state: &GameState, controller: crate::ids::PlayerId, registry: &CardRegistry) -> usize {
        state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| {
                // Check subtypes on object (for tokens) or via registry.
                if o.subtypes.iter().any(|s| s == "Vampire") {
                    return true;
                }
                registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Vampire"))
                    .unwrap_or(false)
            })
            .count()
    }
}

impl CardBehavior for BloodlineKeeper {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bloodline Keeper".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Flying\n{T}: Create a 2/2 black Vampire creature token with flying.\n{B}: Transform Bloodline Keeper. Activate only if you control five or more Vampires.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Lord of Lineage".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "Flying\nOther Vampire creatures you control get +2/+2.\n{T}: Create a 2/2 black Vampire creature token with flying.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 2,
                    toughness: 2,
                    scope: EffectScope::GlobalOther(CreatureFilter::And(vec![
                        CreatureFilter::You,
                        CreatureFilter::HasSubtype("Vampire".into()),
                    ])),
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![],
        })
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((5, 5))
        } else {
            None
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        let is_transformed = obj.is_transformed;
        let controller = obj.controller;
        let mut abilities = vec![];

        // Both faces: {T}: Create a 2/2 Vampire token with flying.
        abilities.push(ActivatedAbilityDef {
            ability_index: 0,
            description: "{T}: Create a 2/2 Vampire token with flying".into(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
        });

        // Front face only: {B}: Transform (requires 5+ Vampires).
        if !is_transformed {
            let vampire_count = Self::count_vampires(state, controller, &CardRegistry::with_all_cards());
            if vampire_count >= 5 {
                abilities.push(ActivatedAbilityDef {
                    ability_index: 1,
                    description: "{B}: Transform Bloodline Keeper (5+ Vampires)".into(),
                    cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Black)]),
                    requires_tap: false,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                });
            }
        }

        abilities
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        match ability_index {
            0 => {
                // Create a 2/2 Vampire token with flying.
                state.create_token_with_subtypes(
                    "Vampire",
                    controller,
                    2, 2,
                    vec![Color::Black],
                    vec![CardType::Creature],
                    vec![Keyword::Flying],
                    vec!["Vampire".into()],
                );
                let face_name = state.get_object(object_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("{}: created a 2/2 Vampire token with flying", face_name));
            }
            1 => {
                // Transform into Lord of Lineage.
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.is_transformed = true;
                    obj.name = "Lord of Lineage".into();
                }
                state.log(crate::state::LogLevel::Event,
                    "Bloodline Keeper transforms into Lord of Lineage".into());
            }
            _ => {}
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

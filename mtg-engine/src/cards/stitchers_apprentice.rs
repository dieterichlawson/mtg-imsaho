use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Stitcher's Apprentice — {1}{U} 1/2 Homunculus.
/// {1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
pub struct StitchersApprentice;

impl CardBehavior for StitchersApprentice {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stitcher's Apprentice".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Homunculus".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "{1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}{U}, {T}: Create a 2/2 blue Homunculus token, then sacrifice a creature".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(1),
                    ManaSymbol::Colored(Color::Blue),
                ]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
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

        // Create a 2/2 blue Homunculus creature token.
        state.create_token_with_subtypes(
            "Homunculus", controller, 2, 2,
            vec![Color::Blue], vec![CardType::Creature],
            vec![], vec!["Homunculus".into()],
        );
        state.log(LogLevel::Event, "Stitcher's Apprentice created a 2/2 Homunculus token".into());

        // Then sacrifice a creature you control — present choice.
        let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.power.is_some())
            .map(|o| o.id)
            .collect();

        if creatures.len() == 1 {
            let name = state.get_object(creatures[0]).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::sacrifice(state, creatures[0], registry);
            state.log(LogLevel::Event, format!("Stitcher's Apprentice: sacrificed {}", name));
        } else if creatures.len() > 1 {
            let options: Vec<Target> = creatures.iter()
                .map(|&id| Target::Object(id))
                .collect();
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Stitcher's Apprentice: choose a creature to sacrifice".into(),
                    options,
                    optional: false,
                    effect: PendingEffect::CardCallbackWithTarget { source_id: object_id },
                },
            });
        }
    }

    fn on_target_chosen(&self, state: &mut GameState, _self_id: ObjectId, target: &Target, registry: &CardRegistry) {
        if let Target::Object(sac_id) = target {
            let name = state.get_object(*sac_id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::sacrifice(state, *sac_id, registry);
            state.log(LogLevel::Event, format!("Stitcher's Apprentice: sacrificed {}", name));
        }
    }
}

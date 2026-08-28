use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            subtypes: vec!["Homunculus".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "{1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
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
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Create a 2/2 blue Homunculus creature token.
        let _token_ids = state.create_token_with_subtypes(
            "", controller, 2, 2,
            vec![Color::Blue], vec![CardType::Creature],
            vec![], vec!["Homunculus".into()],
            registry,
        );
        state.log(LogLevel::Event, "Stitcher's Apprentice created a 2/2 Homunculus token".to_string());

        // Then sacrifice a creature you control.
        // The controller chooses which creature to sacrifice.
        let creatures: Vec<Target> = crate::cards::helpers::creatures_controlled_by(state, controller, registry);

        if creatures.is_empty() {
            state.log(LogLevel::Event, "Stitcher's Apprentice: no creatures to sacrifice".into());
            return;
        }

        crate::cards::helpers::present_target_choice(
            state,
            object_id,
            controller,
            creatures,
            PendingEffect::SacrificeCreature {
                source_name: "Stitcher's Apprentice".into(),
            },
            "Stitcher's Apprentice: choose a creature to sacrifice",
            false, // mandatory
            registry,
        );
    }
}

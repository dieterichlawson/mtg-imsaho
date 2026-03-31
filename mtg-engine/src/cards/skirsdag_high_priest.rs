use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Skirsdag High Priest — {1}{B} 1/2 Human Cleric.
/// Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon
/// creature token with flying. Activate only as a sorcery.
pub struct SkirsdagHighPriest;

impl CardBehavior for SkirsdagHighPriest {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Skirsdag High Priest".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only as a sorcery.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        // Must be on battlefield, untapped, not summoning sick, morbid active,
        // and have at least 2 other untapped creatures to tap.
        if obj.zone != Zone::Battlefield || obj.tapped || obj.summoning_sick {
            return vec![];
        }
        if !state.creature_died_this_turn {
            return vec![];
        }
        let controller = obj.controller;
        let other_untapped_creatures = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.id != object_id && o.power.is_some() && !o.tapped && !o.summoning_sick)
            .count();
        if other_untapped_creatures < 2 {
            return vec![];
        }
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Morbid — {T}, Tap two creatures: Create a 5/5 Demon with flying".into(),
            cost: ManaCost::new(vec![]),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: true,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Tap two other untapped creatures we control.
        let to_tap: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.id != object_id && o.power.is_some() && !o.tapped && !o.summoning_sick)
            .take(2)
            .map(|o| o.id)
            .collect();
        for cid in &to_tap {
            if let Some(obj) = state.get_object_mut(*cid) {
                obj.tapped = true;
            }
        }

        // Create a 5/5 black Demon creature token with flying.
        state.create_token_with_subtypes(
            "Demon",
            controller,
            5, 5,
            vec![Color::Black],
            vec![CardType::Creature],
            vec![Keyword::Flying],
            vec!["Demon".into()],
        );

        state.log(crate::state::LogLevel::Event,
            "Skirsdag High Priest creates a 5/5 black Demon token with flying".to_string());
    }
}

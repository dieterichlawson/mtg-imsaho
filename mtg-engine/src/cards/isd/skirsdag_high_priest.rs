use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Skirsdag High Priest — {1}{B} 1/2 Human Cleric.
/// Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon
/// creature token with flying. Activate only if a creature died this turn.
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
            oracle_text: "Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
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
        // Collect the other untapped creatures the player controls, sorted by
        // ID for a stable, deterministic ordering.
        let mut candidates: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.id != object_id && o.power.is_some() && !o.tapped)
            .map(|o| o.id)
            .collect();
        candidates.sort_by_key(|id| id.0);
        let n = candidates.len();
        if n < 2 {
            return vec![];
        }
        // Return one ActivatedAbilityDef per C(n, 2) combination, each with a
        // unique ability_index encoding the combination index (0-based).
        let mut abilities = Vec::new();
        let mut combo_index = 0usize;
        for i in 0..n {
            for j in (i + 1)..n {
                abilities.push(ActivatedAbilityDef {
                    ability_index: combo_index,
                    description: format!(
                        "Morbid — {{T}}, Tap two creatures: Create a 5/5 Demon with flying (tap {:?} & {:?})",
                        candidates[i], candidates[j]
                    ),
                    cost: ManaCost::new(vec![]),
                    requires_tap: true,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                });
                combo_index += 1;
            }
        }
        abilities
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Reconstruct the same sorted candidate list used in activated_abilities.
        // Note: by the time this is called the High Priest itself is already tapped
        // (the engine pays the {T} cost before calling on_activate_ability), so we
        // still filter on !o.tapped to find only un-tapped creatures.
        let mut candidates: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.id != object_id && o.power.is_some() && !o.tapped)
            .map(|o| o.id)
            .collect();
        candidates.sort_by_key(|id| id.0);
        let n = candidates.len();

        // Decode ability_index back to the (i, j) combination.
        let mut combo_index = 0usize;
        let mut to_tap: Option<(ObjectId, ObjectId)> = None;
        'outer: for i in 0..n {
            for j in (i + 1)..n {
                if combo_index == ability_index {
                    to_tap = Some((candidates[i], candidates[j]));
                    break 'outer;
                }
                combo_index += 1;
            }
        }

        // Tap the chosen pair.
        if let Some((c1, c2)) = to_tap {
            if let Some(obj) = state.get_object_mut(c1) { obj.tapped = true; }
            if let Some(obj) = state.get_object_mut(c2) { obj.tapped = true; }
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

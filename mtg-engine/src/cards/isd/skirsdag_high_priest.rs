use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, Keyword};

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
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        // The {T} part of the cost — untapped, and past summoning sickness
        // unless hasty (CR 302.6) — is the engine's to check. Spelling it out
        // here shadowed that check and dropped the haste exception, so a
        // Priest holding Lightning Greaves couldn't be activated at all.
        // What's particular to this ability: morbid, and two other untapped
        // creatures to tap.
        if obj.zone != Zone::Battlefield {
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
            .filter(|o| o.id != object_id && state.is_creature(o.id, registry) && !o.tapped)
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
                let name_i = state.get_object(candidates[i]).map_or_else(|| "?".to_string(), |o| o.name.clone());
                let name_j = state.get_object(candidates[j]).map_or_else(|| "?".to_string(), |o| o.name.clone());
                abilities.push(ActivatedAbilityDef {
                    ability_index: combo_index,
                    description: format!(
                        "Morbid — {{T}}, Tap two creatures: Create a 5/5 Demon with flying (tap {name_i} & {name_j})",
                    ),
                    cost: ManaCost::new(vec![]),
                    requires_tap: true,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                    counter_cost: None,
                });
                combo_index += 1;
            }
        }
        abilities
    }

    fn pay_activation_cost(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Tap the two chosen creatures (cost payment — happens before stack push).
        let mut candidates: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.id != object_id && state.is_creature(o.id, registry) && !o.tapped)
            .map(|o| o.id)
            .collect();
        candidates.sort_by_key(|id| id.0);
        let n = candidates.len();

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

        if let Some((c1, c2)) = to_tap {
            if let Some(obj) = state.get_object_mut(c1) { obj.tapped = true; }
            if let Some(obj) = state.get_object_mut(c2) { obj.tapped = true; }
        }

    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        state.create_token_with_subtypes(
            "Demon",
            controller,
            5, 5,
            vec![Color::Black],
            vec![CardType::Creature],
            vec![Keyword::Flying],
            vec!["Demon".into()],
            registry,
        );

        state.log(crate::state::LogLevel::Event,
            "Skirsdag High Priest creates a 5/5 black Demon token with flying".to_string());
    }
}

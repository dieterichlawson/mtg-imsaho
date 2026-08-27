use crate::actions::Target;
use crate::cards::helpers;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, CounterType, Keyword, Zone};

/// Ludevic's Test Subject {1}{U} 0/3 Lizard Egg // Ludevic's Abomination 13/13 Lizard Horror
/// with Trample.
/// {1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters
/// on it, remove all of them and transform it.
///
/// Implementation: hatchling counters live in the engine's counter pipeline
/// (`CounterType::Hatchling`) so proliferate / counter-removal effects can
/// interact with them per CR 122.
pub struct LudevicsTestSubject;

impl CardBehavior for LudevicsTestSubject {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ludevic's Test Subject".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Lizard".into(), "Egg".into()],
            power: Some(0),
            toughness: Some(3),
            oracle_text: "Defender\n{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.".into(),
            keywords: vec![Keyword::Defender],
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Ludevic's Abomination".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Lizard".into(), "Horror".into()],
            power: Some(13),
            toughness: Some(13),
            oracle_text: "Trample".into(),
            keywords: vec![Keyword::Trample],
            ..Default::default()
        })
    }


    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield && !o.is_transformed => o,
            _ => return vec![],
        };
        let _ = obj;
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}{U}: Put a hatchling counter. At 5, transform.".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        // If already transformed, this ability shouldn't do anything (back face has no activated abilities).
        if state.get_object(object_id).is_some_and(|o| o.is_transformed) {
            return;
        }
        // Add a hatchling counter via the standard counter pipeline.
        state.add_counters(object_id, CounterType::Hatchling, 1);
        let new_count = state.get_object(object_id)
            .and_then(|o| o.counters.get(&CounterType::Hatchling).copied())
            .unwrap_or(0);

        if new_count >= 5 {
            // Remove all hatchling counters and transform.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.counters.remove(&CounterType::Hatchling);
            }
            helpers::apply_transform(state, object_id, registry);
            state.log(crate::state::LogLevel::Event,
                "Ludevic's Test Subject transforms into Ludevic's Abomination (13/13 Trample)!".into());
        } else {
            state.log(crate::state::LogLevel::Event,
                format!("Ludevic's Test Subject: hatchling counter added ({new_count}/5)"));
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, ActivatedAbilityDef, SacrificeCost};
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Disciple of Griselbrand — {1}{B} 1/1 Human Cleric.
/// {1}, Sacrifice a creature: You gain life equal to that creature's toughness.
pub struct DiscipleOfGriselbrand;

impl CardBehavior for DiscipleOfGriselbrand {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Disciple of Griselbrand".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}, Sacrifice a creature: Gain life equal to its toughness".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeCreature,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        // The engine already sacrificed a creature before calling this.
        // Find the most recent CreatureDied event to get the sacrificed creature's toughness.
        let toughness = state.events.iter().rev()
            .find_map(|e| match e {
                GameEvent::CreatureDied { last_known_toughness, .. } => Some(*last_known_toughness),
                _ => None,
            })
            .unwrap_or(0)
            .max(0);

        if toughness > 0 {
            let old = state.get_player(controller).life;
            let new_life = old + toughness;
            state.get_player_mut(controller).life = new_life;
            state.events.push(GameEvent::LifeChanged { player: controller, old, new_life });
            state.log(crate::state::LogLevel::Event,
                format!("Disciple of Griselbrand: gained {} life", toughness));
        }
    }
}

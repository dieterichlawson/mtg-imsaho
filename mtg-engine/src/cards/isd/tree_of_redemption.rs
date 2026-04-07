use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Tree of Redemption — {3}{G} 0/13 Plant with Defender.
/// {T}: Exchange your life total with Tree of Redemption's toughness.
pub struct TreeOfRedemption;

impl CardBehavior for TreeOfRedemption {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Tree of Redemption".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Plant".into()],
            power: Some(0),
            toughness: Some(13),
            oracle_text: "Defender\n{T}: Exchange your life total with this creature's toughness.".into(),
            keywords: vec![Keyword::Defender],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{T}: Exchange life total with Tree's toughness".into(),
                cost: ManaCost::free(),
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
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };
        // Get effective toughness (base + counters + buffs).
        let current_toughness = state.effective_toughness(object_id, registry).unwrap_or(13);
        let current_life = state.get_player(controller).life;

        // Set life to Tree's toughness.
        let old_life = current_life;
        state.get_player_mut(controller).life = current_toughness;
        state.events.push(crate::events::GameEvent::LifeChanged {
            player: controller,
            old: old_life,
            new_life: current_toughness,
        });

        // Set Tree's base toughness to old life total.
        // We adjust the base toughness by modifying the object's toughness field.
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.toughness = Some(current_life);
        }

        state.log(crate::state::LogLevel::Event,
            format!("Tree of Redemption: exchanged life ({}) with toughness ({})",
                old_life, current_toughness));
    }
}

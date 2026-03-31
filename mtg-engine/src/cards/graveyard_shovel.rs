use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Graveyard Shovel — {2} Artifact.
/// {2}, {T}: Exile target card from a graveyard. If it was a creature card, you gain 2 life.
pub struct GraveyardShovel;

impl CardBehavior for GraveyardShovel {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Graveyard Shovel".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{2}, {T}: Exile target card from a graveyard. If it was a creature card, you gain 2 life.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone != Zone::Battlefield || obj.tapped {
            return vec![];
        }
        let has_graveyard_card = state.objects.values()
            .any(|o| o.zone == Zone::Graveyard);
        if !has_graveyard_card {
            return vec![];
        }
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{2}, {T}: Exile a card from a graveyard, gain 2 life if creature".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(2)]),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::GraveyardCard),
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        if let Some(Target::Object(target_id)) = targets.first() {
            let is_creature = state.get_object(*target_id)
                .map(|o| {
                    registry.card_data(o.card_id)
                        .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                        .unwrap_or(o.power.is_some())
                })
                .unwrap_or(false);
            let name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();

            state.move_object(*target_id, Zone::Exile);
            state.log(crate::state::LogLevel::Event,
                format!("Graveyard Shovel exiled {} from graveyard", name));

            if is_creature {
                let old_life = state.get_player(controller).life;
                let new_life = old_life + 2;
                state.get_player_mut(controller).life = new_life;
                state.events.push(crate::events::GameEvent::LifeChanged {
                    player: controller,
                    old: old_life,
                    new_life,
                });
                state.log(crate::state::LogLevel::Event,
                    format!("Graveyard Shovel: p{} gained 2 life (creature exiled)", controller.0));
            }
        }
    }
}

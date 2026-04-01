use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Infernal Plunge — {R} Sorcery.
/// As an additional cost to cast Infernal Plunge, sacrifice a creature.
/// Add {R}{R}{R}.
pub struct InfernalPlunge;

impl CardBehavior for InfernalPlunge {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Infernal Plunge".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As an additional cost to cast this spell, sacrifice a creature.\nAdd {R}{R}{R}.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::SacrificeCreature),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id)
            .map(|o| o.controller)
            .unwrap_or(crate::ids::PlayerId(0));

        let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.power.is_some())
            .map(|o| o.id)
            .collect();

        if creatures.len() == 1 {
            infernal_plunge_sac(state, object_id, creatures[0], controller, registry);
        } else if creatures.len() > 1 {
            let options: Vec<Target> = creatures.iter().map(|&id| Target::Object(id)).collect();
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Infernal Plunge: choose a creature to sacrifice".into(),
                    options,
                    optional: false,
                    effect: PendingEffect::CardCallbackWithTarget { source_id: object_id },
                },
            });
            return;
        }
        state.move_spell_after_resolve(object_id);
    }

    fn on_target_chosen(&self, state: &mut GameState, self_id: ObjectId, target: &Target, registry: &CardRegistry) {
        if let Target::Object(sac_id) = target {
            let controller = state.get_object(self_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
            infernal_plunge_sac(state, self_id, *sac_id, controller, registry);
        }
        state.move_spell_after_resolve(self_id);
    }
}

fn infernal_plunge_sac(state: &mut GameState, _spell_id: ObjectId, sac_id: ObjectId, controller: crate::ids::PlayerId, registry: &CardRegistry) {
    let name = state.get_object(sac_id).map(|o| o.name.clone()).unwrap_or_default();
    crate::destruction::sacrifice(state, sac_id, registry);
    state.get_player_mut(controller).mana_pool.add(ManaType::Red, 3);
    state.log(crate::state::LogLevel::Event,
        format!("Infernal Plunge: sacrificed {}, added {{R}}{{R}}{{R}}", name));
}

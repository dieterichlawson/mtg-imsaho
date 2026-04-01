use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Corpse Lunge — {2}{B} Instant.
/// As an additional cost to cast Corpse Lunge, exile a creature card from your graveyard.
/// Corpse Lunge deals damage equal to the exiled card's power to target creature.
pub struct CorpseLunge;

fn exile_and_deal_damage(state: &mut GameState, spell_id: ObjectId, exile_id: ObjectId) {
    let power = state.get_object(exile_id).and_then(|o| o.power).unwrap_or(0);
    let exile_name = state.get_object(exile_id).map(|o| o.name.clone()).unwrap_or_default();
    state.move_object(exile_id, Zone::Exile);
    state.log(LogLevel::Event, format!("Corpse Lunge exiled {} from graveyard", exile_name));

    let damage = power.max(0) as u32;
    if damage > 0 {
        let target_id = state.get_object(spell_id)
            .and_then(|o| o.card_state.get("damage_target").copied());
        if let Some(target_id) = target_id {
            if state.get_object(target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.mark_damage_on_creature(target_id, damage, spell_id);
                state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                    source: spell_id,
                    target: crate::events::DamageTarget::Object(target_id),
                    amount: damage,
                });
                state.log(LogLevel::Event, format!("Corpse Lunge dealt {} damage to {}", damage, name));
            }
        }
    }
}

impl CardBehavior for CorpseLunge {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Corpse Lunge".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As an additional cost to cast Corpse Lunge, exile a creature card from your graveyard.\nCorpse Lunge deals damage equal to the exiled card's power to target creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Store spell target in card_state for use after exile choice.
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.card_state.insert("damage_target".into(), *target_id);
            }
        }

        // Exile a creature card from graveyard — present choice if multiple.
        let gy_creatures: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.power.is_some())
            .map(|o| o.id)
            .collect();

        if gy_creatures.len() == 1 {
            exile_and_deal_damage(state, object_id, gy_creatures[0]);
            state.move_spell_after_resolve(object_id);
        } else if gy_creatures.len() > 1 {
            let options: Vec<Target> = gy_creatures.iter()
                .map(|&id| Target::Object(id))
                .collect();
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Corpse Lunge: choose a creature card to exile from graveyard".into(),
                    options,
                    optional: false,
                    effect: PendingEffect::CardCallbackWithTarget { source_id: object_id },
                },
            });
        } else {
            state.move_spell_after_resolve(object_id);
        }
    }

    fn on_target_chosen(&self, state: &mut GameState, self_id: ObjectId, target: &Target, _registry: &CardRegistry) {
        if let Target::Object(exile_id) = target {
            exile_and_deal_damage(state, self_id, *exile_id);
        }
        state.move_spell_after_resolve(self_id);
    }
}

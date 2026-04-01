use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::*;

/// Harvest Pyre — {1}{R} Instant.
/// As an additional cost to cast this spell, exile X cards from your graveyard.
/// Harvest Pyre deals X damage to target creature.
pub struct HarvestPyre;

impl CardBehavior for HarvestPyre {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Harvest Pyre".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As an additional cost to cast this spell, exile X cards from your graveyard.\nHarvest Pyre deals X damage to target creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Count graveyard cards (excluding Harvest Pyre itself).
        let max_cards = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.id != object_id)
            .count() as u32;

        if max_cards == 0 {
            state.log(LogLevel::Event, "Harvest Pyre: no cards in graveyard to exile".into());
            state.move_spell_after_resolve(object_id);
            return;
        }

        // Store the target for later use in on_number_chosen.
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.card_state.insert("damage_target".into(), *target_id);
            }
        }

        // Present choice: how many cards to exile (1..=max).
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: object_id,
            choice: ResolutionChoiceKind::ChooseNumber {
                description: format!("Harvest Pyre: exile how many cards from graveyard? (1-{})", max_cards),
                min: 1,
                max: max_cards,
                source_id: object_id,
            },
        });
    }

    fn on_number_chosen(&self, state: &mut GameState, self_id: ObjectId, number: u32, _registry: &CardRegistry) {
        let controller = state.get_object(self_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Exile that many cards from graveyard.
        let graveyard_cards: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.id != self_id)
            .map(|o| o.id)
            .take(number as usize)
            .collect();

        let count = graveyard_cards.len() as u32;
        for card_id in &graveyard_cards {
            state.move_object(*card_id, Zone::Exile);
        }
        state.log(LogLevel::Event, format!("Harvest Pyre exiled {} cards from graveyard", count));

        // Deal damage to the target creature.
        let target_id = state.get_object(self_id)
            .and_then(|o| o.card_state.get("damage_target").copied());
        if let Some(target_id) = target_id {
            if state.get_object(target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.mark_damage_on_creature(target_id, count, self_id);
                state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                    source: self_id,
                    target: crate::events::DamageTarget::Object(target_id),
                    amount: count,
                });
                state.log(LogLevel::Event, format!("Harvest Pyre dealt {} damage to {}", count, name));
            }
        }

        state.move_spell_after_resolve(self_id);
    }
}

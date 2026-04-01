use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Night Terrors — {2}{B} Sorcery.
/// Target player reveals their hand. You choose a nonland card from it and exile that card.
pub struct NightTerrors;

impl CardBehavior for NightTerrors {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Night Terrors".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target player reveals their hand. You choose a nonland card from it and exile that card.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        let caster = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        if let Some(Target::Player(target_player)) = targets.first() {
            // Reveal target player's hand — find nonland cards.
            let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, *target_player)
                .iter()
                .map(|o| o.id)
                .collect();

            let nonland_cards: Vec<ObjectId> = hand.iter().filter(|&&card_id| {
                let is_land = registry.card_data(
                    state.get_object(card_id).map(|o| o.card_id).unwrap_or(crate::ids::CardId(0))
                )
                .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Land)))
                .unwrap_or(false);
                !is_land
            }).copied().collect();

            if nonland_cards.len() == 1 {
                let exile_id = nonland_cards[0];
                let name = state.get_object(exile_id).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(exile_id, Zone::Exile);
                state.log(crate::state::LogLevel::Event,
                    format!("Night Terrors: exiled {} from p{}'s hand", name, target_player.0));
            } else if nonland_cards.len() > 1 {
                let options: Vec<Target> = nonland_cards.iter()
                    .map(|&id| Target::Object(id))
                    .collect();
                state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                    player: caster,
                    source: object_id,
                    choice: ResolutionChoiceKind::ChooseTarget {
                        description: "Night Terrors: choose a nonland card to exile".into(),
                        options,
                        optional: false,
                        effect: PendingEffect::CardCallbackWithTarget { source_id: object_id },
                    },
                });
                // Don't move to graveyard yet — wait for choice.
                return;
            } else {
                state.log(crate::state::LogLevel::Event,
                    format!("Night Terrors: no nonland card in p{}'s hand", target_player.0));
            }
        }
        state.move_spell_after_resolve(object_id);
    }

    fn on_target_chosen(&self, state: &mut GameState, self_id: ObjectId, target: &Target, _registry: &CardRegistry) {
        if let Target::Object(exile_id) = target {
            let name = state.get_object(*exile_id).map(|o| o.name.clone()).unwrap_or_default();
            let owner = state.get_object(*exile_id).map(|o| o.owner.0).unwrap_or(0);
            state.move_object(*exile_id, Zone::Exile);
            state.log(crate::state::LogLevel::Event,
                format!("Night Terrors: exiled {} from p{}'s hand", name, owner));
        }
        state.move_spell_after_resolve(self_id);
    }
}

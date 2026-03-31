use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, ActivatedAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Brain Weevil — {3}{B} 1/1 Insect. Intimidate.
/// Sacrifice Brain Weevil: Target player discards two cards. Activate only as a sorcery.
pub struct BrainWeevil;

impl CardBehavior for BrainWeevil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Brain Weevil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Insect".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Intimidate\nSacrifice Brain Weevil: Target player discards two cards. Activate only as a sorcery.".into(),
            keywords: vec![Keyword::Intimidate],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Sacrifice: Target player discards two cards".into(),
            cost: ManaCost::new(vec![]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeThis,
            target_requirement: Some(TargetRequirement::PlayerOnly),
            once_per_turn: false,
            sorcery_speed_only: true,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Player(target_player)) = targets.first() {
            let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, *target_player)
                .iter()
                .map(|o| o.id)
                .collect();

            // Discard up to 2 cards (random selection for now).
            let to_discard = hand.len().min(2);
            for i in 0..to_discard {
                let card_id = hand[i];
                state.move_object(card_id, Zone::Graveyard);
                state.events.push(crate::events::GameEvent::Discarded {
                    player: *target_player,
                    object: card_id,
                });
            }

            state.log(crate::state::LogLevel::Event,
                format!("Brain Weevil: p{} discarded {} card(s)", target_player.0, to_discard));
        }
    }
}

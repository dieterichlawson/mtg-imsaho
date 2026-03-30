use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Sturmgeist — {3}{U}{U} */* Spirit. Flying.
/// Sturmgeist's power and toughness are each equal to the number of cards in your hand.
/// Whenever Sturmgeist deals combat damage to a player, draw a card.
pub struct Sturmgeist;

impl CardBehavior for Sturmgeist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sturmgeist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Flying\nSturmgeist's power and toughness are each equal to the number of cards in your hand.\nWhenever Sturmgeist deals combat damage to a player, draw a card.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "draw a card".into(),
                },
            ],
        }
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        let controller = state.get_object(object_id)?.controller;
        let hand_size = state.objects_in_zone(Zone::Hand, controller).len() as i32;
        Some((hand_size, hand_size))
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        crate::engine::draw_cards(state, controller, 1);
    }
}

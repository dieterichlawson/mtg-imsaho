use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Murder of Crows — {3}{U}{U} 4/4 Bird. Flying.
/// Whenever another creature dies, you may draw a card. If you do, discard a card.
pub struct MurderOfCrows;

impl CardBehavior for MurderOfCrows {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Murder of Crows".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Bird".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Flying\nWhenever another creature dies, you may draw a card. If you do, discard a card.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "draw a card, then discard a card".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // TODO: The draw should be optional ("you may draw a card") and the discard
        // should be player-chosen. Currently auto-draw and auto-discard because we
        // don't have a "discard 1 from N" choice mechanism yet.

        // Draw a card.
        crate::engine::draw_cards(state, controller, 1);

        // Discard: pick the first non-land card in hand, or fall back to the last card.
        let hand: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Hand && o.owner == controller)
            .map(|o| o.id)
            .collect();

        if hand.is_empty() {
            return;
        }

        let to_discard = hand.iter()
            .find(|&&id| {
                state.get_object(id)
                    .map(|o| !o.card_types.contains(&CardType::Land))
                    .unwrap_or(true)
            })
            .copied()
            .unwrap_or(*hand.last().unwrap());

        state.move_object(to_discard, Zone::Graveyard);
        let name = state.get_object(to_discard).map(|o| o.name.clone()).unwrap_or_default();
        state.log(crate::state::LogLevel::Event,
            format!("Murder of Crows: p{} drew a card and discarded {}", controller.0, name));
    }
}

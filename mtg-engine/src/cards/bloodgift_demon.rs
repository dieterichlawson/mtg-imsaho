use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Bloodgift Demon — {3}{B}{B} 5/4 flying Demon.
/// At the beginning of your upkeep, target player draws a card and loses 1 life.
pub struct BloodgiftDemon;

impl CardBehavior for BloodgiftDemon {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bloodgift Demon".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Demon".into()],
            power: Some(5),
            toughness: Some(4),
            oracle_text: "Flying\nAt the beginning of your upkeep, target player draws a card and loses 1 life.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "target player draws a card and loses 1 life".into(),
                },
            ],
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Only trigger on your upkeep.
        if state.active_player != controller {
            return;
        }
        // "Target player draws a card and loses 1 life"
        // Auto-target self in 2-player (usually want to draw for yourself).
        crate::engine::draw_cards(state, controller, 1);
        let old = state.get_player(controller).life;
        let new_life = old - 1;
        state.get_player_mut(controller).life = new_life;
        state.events.push(crate::events::GameEvent::LifeChanged { player: controller, old, new_life });
        state.log(crate::state::LogLevel::Event,
            format!("Bloodgift Demon: p{} drew a card and lost 1 life", controller.0));
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Selhoff Occultist — {2}{U} 2/3 Human Rogue.
/// Whenever Selhoff Occultist or another creature dies, target player mills a card.
pub struct SelhoffOccultist;

impl CardBehavior for SelhoffOccultist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Selhoff Occultist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Rogue".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "Whenever Selhoff Occultist or another creature dies, target player mills a card.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "target player mills a card".into(),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "target player mills a card".into(),
                },
            ],
        }
    }

    /// When Selhoff Occultist itself dies, mill 1 from opponent.
    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let owner = state.get_object(object_id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
        let opponent = state.opponent(owner);
        crate::engine::mill_cards(state, opponent, 1);
    }

    /// When another creature dies, mill 1 from opponent.
    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        let opponent = state.opponent(controller);
        crate::engine::mill_cards(state, opponent, 1);
    }
}

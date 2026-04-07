use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Deranged Assistant — {1}{U} 1/1 Human Wizard.
/// {T}, Mill a card: Add {C}.
///
/// Note: The mill-a-card cost is handled as part of the mana ability resolution.
/// The engine's mana ability system calls `on_activate_mana_ability` (via the
/// standard tap-for-mana path), so we mill during the mana production step.
pub struct DerangedAssistant;

impl CardBehavior for DerangedAssistant {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Deranged Assistant".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "{T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        // Need to have a card in library to mill.
        let controller = obj.controller;
        let has_library = !state.get_player(controller).library_order.is_empty();
        if obj.zone == Zone::Battlefield && !obj.tapped && !obj.summoning_sick && has_library {
            vec![ManaAbilityDef {
                ability_index: 0,
                description: "Mill a card, add {C}".into(),
                produced: vec![(ManaType::Colorless, 1)],
                requires_tap: true,
                has_side_effects: true,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_mana_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };
        // Mill a card as part of the mana ability cost.
        crate::engine::mill_cards(state, controller, 1, registry);
        state.log(crate::state::LogLevel::Event,
            "Deranged Assistant: milled a card".into());
    }
}

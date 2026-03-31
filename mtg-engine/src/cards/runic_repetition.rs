use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Runic Repetition — {2}{U} Sorcery.
/// Return target exiled card you own with flashback to your hand.
///
/// Simplified: returns a card with flashback from exile to your hand (auto-selects).
pub struct RunicRepetition;

impl CardBehavior for RunicRepetition {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Runic Repetition".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Return target exiled card you own with flashback to your hand.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Find a card in exile owned by controller that has flashback.
        let flashback_card = state.objects.values()
            .filter(|o| o.zone == Zone::Exile && o.owner == controller && !o.is_token)
            .find(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.flashback_cost.is_some())
                    .unwrap_or(false)
            })
            .map(|o| o.id);

        if let Some(card_id) = flashback_card {
            let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(card_id, Zone::Hand);
            state.log(crate::state::LogLevel::Event,
                format!("Runic Repetition returned {} from exile to hand", name));
        } else {
            state.log(crate::state::LogLevel::Event,
                "Runic Repetition: no valid target in exile".into());
        }

        state.move_spell_after_resolve(object_id);
    }
}

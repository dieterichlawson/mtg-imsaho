use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Nevermore — {1}{W}{W} Enchantment.
/// As this enchantment enters, choose a nonland card name.
/// Spells with the chosen name can't be cast.
///
/// Stores the chosen name as a PreventCastingNamed instance continuous effect.
/// The engine checks for this effect in legal_actions.
pub struct Nevermore;

impl CardBehavior for Nevermore {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Nevermore".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As this enchantment enters, choose a nonland card name.\nSpells with the chosen name can't be cast.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let opponent = state.opponent(controller);

        // Choose the most threatening card in the opponent's hand.
        // In a real game, the player would choose; here we auto-select
        // a nonland card from the opponent's hand if possible.
        let chosen_name = state.objects.values()
            .filter(|o| o.zone == Zone::Hand && o.owner == opponent)
            .filter_map(|o| {
                registry.card_data(o.card_id).and_then(|d| {
                    if !d.card_types.contains(&CardType::Land) {
                        Some(d.name)
                    } else {
                        None
                    }
                })
            })
            .next()
            .unwrap_or_else(|| "Lightning Bolt".into()); // Default if nothing found.

        // Store the restriction as an instance continuous effect.
        if let Some(obj) = state.get_object_mut(object_id) {
            let effect = ContinuousEffect::PreventCastingNamed { name: chosen_name.clone() };
            obj.instance_continuous_effects = Some(vec![effect]);
        }
        state.log(crate::state::LogLevel::Event,
            format!("Nevermore names \"{}\"", chosen_name));
    }
}

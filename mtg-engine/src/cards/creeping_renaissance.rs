use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Creeping Renaissance — {3}{G}{G} Sorcery.
/// Choose a permanent type. Return all cards of the chosen type from your graveyard
/// to your hand. Flashback {5}{G}{G}.
///
/// Simplified: automatically chooses "creature" as the permanent type, since that's
/// the most commonly relevant type for graveyard recursion in Innistrad.
pub struct CreepingRenaissance;

impl CardBehavior for CreepingRenaissance {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Creeping Renaissance".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.\nFlashback {5}{G}{G}".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Auto-choose "creature" as the permanent type.
        let to_return: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                    .unwrap_or(o.power.is_some())
            })
            .map(|o| o.id)
            .collect();

        let count = to_return.len();
        for id in to_return {
            state.move_object(id, Zone::Hand);
        }
        state.log(crate::state::LogLevel::Event,
            format!("Creeping Renaissance returned {} creature cards from graveyard to hand", count));
        state.move_spell_after_resolve(object_id);
    }
}

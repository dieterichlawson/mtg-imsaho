use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Altar's Reap — {1}{B} Instant.
/// As an additional cost to cast Altar's Reap, sacrifice a creature.
/// Draw two cards.
pub struct AltarsReap;

impl CardBehavior for AltarsReap {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Altar's Reap".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As an additional cost to cast this spell, sacrifice a creature.\nDraw two cards.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::SacrificeCreature),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id)
            .map(|o| o.controller)
            .unwrap_or(crate::ids::PlayerId(0));

        // SIMPLIFICATION: In real MTG, the sacrifice happens as part of casting (before
        // the spell goes on the stack). Here we sacrifice on resolution because the engine
        // doesn't yet support multi-step casting with additional costs.
        let creature_to_sac = state.objects.values()
            .find(|o| o.zone == Zone::Battlefield && o.controller == controller && o.power.is_some())
            .map(|o| o.id);

        if let Some(sac_id) = creature_to_sac {
            crate::destruction::sacrifice(state, sac_id, registry);
        }

        crate::engine::draw_cards(state, controller, 2);
        state.move_spell_after_resolve(object_id);
    }
}

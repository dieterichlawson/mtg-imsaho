use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Hysterical Blindness — {2}{U} instant. Creatures your opponents control get -4/-0 until end of turn.
pub struct HystericalBlindness;

impl CardBehavior for HystericalBlindness {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Hysterical Blindness".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Creatures your opponents control get -4/-0 until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        // Collect creature IDs controlled by opponents.
        let opponent_creature_ids: Vec<ObjectId> = state.objects.values()
            .filter(|obj| {
                obj.zone == Zone::Battlefield
                    && obj.controller != controller
                    && obj.power.is_some() // is a creature
            })
            .map(|obj| obj.id)
            .collect();

        for id in opponent_creature_ids {
            state.until_end_of_turn.push(
                crate::state::TemporaryEffect::ModifyPT {
                    target: id,
                    power_mod: -4,
                    toughness_mod: 0,
                }
            );
        }

        state.move_spell_after_resolve(object_id);
    }
}

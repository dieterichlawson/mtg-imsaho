use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Rally the Peasants — {2}{W} instant. Creatures you control get +2/+0 until end of turn.
pub struct RallyThePeasants;

impl CardBehavior for RallyThePeasants {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rally the Peasants".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Creatures you control get +2/+0 until end of turn.\nFlashback {2}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Red)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        // Collect creature IDs controlled by this player.
        let creature_ids: Vec<ObjectId> = state.objects.values()
            .filter(|obj| {
                obj.zone == Zone::Battlefield
                    && obj.controller == controller
                    && obj.power.is_some() // is a creature
            })
            .map(|obj| obj.id)
            .collect();

        for id in creature_ids {
            state.until_end_of_turn.push(
                crate::state::TemporaryEffect::ModifyPT {
                    target: id,
                    power_mod: 2,
                    toughness_mod: 0,
                }
            );
        }

        state.move_spell_after_resolve(object_id);
    }
}

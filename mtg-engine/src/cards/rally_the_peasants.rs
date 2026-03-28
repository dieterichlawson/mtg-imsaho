use crate::actions::Target;
use crate::cards::{CardBehavior, CardData};
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
            oracle_text: "Creatures you control get +2/+0 until end of turn.".into(),
            keywords: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target]) {
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
            state.until_end_of_turn_effects.push(
                crate::state::UntilEndOfTurnEffect {
                    target: id,
                    power_mod: 2,
                    toughness_mod: 0,
                }
            );
        }

        state.move_object(object_id, Zone::Graveyard);
    }
}

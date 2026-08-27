use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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
            oracle_text: "Creatures you control get +2/+0 until end of turn.\nFlashback {2}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Red)])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        // CR 611.2c: a continuous effect created by a resolving spell or
        // ability affects the set of objects that existed when it resolved, and
        // that set never changes. This is the line between Glorious Anthem (a
        // permanent's static ability, which picks up newcomers) and a pump
        // spell (which does not) — so the creatures are snapshotted here rather
        // than matched by a live filter every time P/T is computed.
        for id in state.creatures_controlled_snapshot(controller, registry) {
            state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
                target: id, power_mod: 2, toughness_mod: 0,
            });
        }

    }
}

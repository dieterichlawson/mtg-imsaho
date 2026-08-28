use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Gnaw to the Bone — {2}{G} instant. You gain 2 life for each creature card in your graveyard.
pub struct GnawToTheBone;

impl CardBehavior for GnawToTheBone {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Gnaw to the Bone".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "You gain 2 life for each creature card in your graveyard.\nFlashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Green)])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(PlayerId(0), |o| o.controller);
        // Count creature cards in controller's graveyard (the spell is still on the stack, not in graveyard).
        let creature_count = state.objects_in_zone(Zone::Graveyard, controller).into_iter()
            .filter(|o| state.is_creature(o.id, registry) && state.is_card(o.id) && o.id != object_id)
            .count();
        let life_gain = i32::try_from(creature_count).unwrap_or(i32::MAX) * 2;
        if life_gain > 0 {
            state.change_life(controller, life_gain);
        }
    }
}

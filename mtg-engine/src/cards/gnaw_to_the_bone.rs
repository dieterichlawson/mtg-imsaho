use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "You gain 2 life for each creature card in your graveyard.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Green)])),
            continuous_effects: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
        // Count creature cards in controller's graveyard (the spell is still on the stack, not in graveyard).
        let creature_count = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.power.is_some() && o.id != object_id)
            .count();
        let life_gain = (creature_count as i32) * 2;
        if life_gain > 0 {
            let old_life = state.get_player(controller).life;
            let new_life = old_life + life_gain;
            state.get_player_mut(controller).life = new_life;
            state.events.push(GameEvent::LifeChanged {
                player: controller,
                old: old_life,
                new_life,
            });
        }
        state.move_spell_after_resolve(object_id);
    }
}

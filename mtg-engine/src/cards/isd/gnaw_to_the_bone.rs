use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::events::GameEvent;
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
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "You gain 2 life for each creature card in your graveyard.\nFlashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Green)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(PlayerId(0), |o| o.controller);
        // Count creature cards in controller's graveyard (the spell is still on the stack, not in graveyard).
        let creature_count = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.power.is_some() && !o.is_token && o.id != object_id)
            .count();
        let life_gain = i32::try_from(creature_count).unwrap_or(i32::MAX) * 2;
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
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Ghoulcaller's Chant — {B} Sorcery.
/// Choose one — Return target creature card from your graveyard to your hand;
/// or return two target Zombie creature cards from your graveyard to your hand.
///
/// Simplified: returns up to two creature cards from your graveyard to hand,
/// preferring Zombies.
pub struct GhoulcallersChant;

impl CardBehavior for GhoulcallersChant {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ghoulcaller's Chant".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Choose one —\n• Return target creature card from your graveyard to your hand.\n• Return two target Zombie creature cards from your graveyard to your hand.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Check for two Zombies in graveyard first (mode 2).
        let zombies: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                let is_creature = registry.card_data(o.card_id)
                    .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                    .unwrap_or(o.power.is_some());
                let is_zombie = registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
                    .unwrap_or(false);
                is_creature && is_zombie
            })
            .map(|o| o.id)
            .collect();

        if zombies.len() >= 2 {
            // Mode 2: return two Zombies.
            for &zid in zombies.iter().take(2) {
                let name = state.get_object(zid).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(zid, Zone::Hand);
                state.log(crate::state::LogLevel::Event,
                    format!("Ghoulcaller's Chant returned {} to hand", name));
            }
        } else {
            // Mode 1: return one creature card.
            let creature = state.objects_in_zone(Zone::Graveyard, controller)
                .iter()
                .filter(|o| {
                    registry.card_data(o.card_id)
                        .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                        .unwrap_or(o.power.is_some())
                })
                .map(|o| o.id)
                .next();

            if let Some(cid) = creature {
                let name = state.get_object(cid).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(cid, Zone::Hand);
                state.log(crate::state::LogLevel::Event,
                    format!("Ghoulcaller's Chant returned {} to hand", name));
            }
        }

        state.move_spell_after_resolve(object_id);
    }
}

use rand::seq::SliceRandom;

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Ghoulraiser — {1}{B}{B} 2/2 Zombie.
/// When Ghoulraiser enters the battlefield, return a Zombie creature card at random
/// from your graveyard to your hand.
pub struct Ghoulraiser;

impl CardBehavior for Ghoulraiser {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ghoulraiser".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "When Ghoulraiser enters the battlefield, return a Zombie creature card at random from your graveyard to your hand.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "return a random Zombie from graveyard to hand".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // Find Zombie creature cards in graveyard.
        let mut zombies: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
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

        if !zombies.is_empty() {
            let mut rng = rand::thread_rng();
            zombies.shuffle(&mut rng);
            let chosen = zombies[0];
            let name = state.get_object(chosen).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(chosen, Zone::Hand);
            state.log(crate::state::LogLevel::Event,
                format!("Ghoulraiser returned {} to hand", name));
        }
    }
}

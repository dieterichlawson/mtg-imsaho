use rand::seq::SliceRandom;

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};
use crate::actions::Target;

/// Ghoulraiser — {1}{B}{B} 2/2 Zombie.
/// When this creature enters, return a Zombie card at random from your graveyard
/// to your hand.
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
            oracle_text: "When this creature enters, return a Zombie card at random from your graveyard to your hand.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "return a random Zombie from graveyard to hand".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // Find Zombie cards in graveyard (not restricted to creatures).
        let mut zombies: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .is_some_and(|d| d.subtypes.iter().any(|s| s == "Zombie"))
            })
            .map(|o| o.id)
            .collect();

        if !zombies.is_empty() {
            let mut rng = rand::thread_rng();
            zombies.shuffle(&mut rng);
            let chosen = zombies[0];
            let name = state.get_object(chosen).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(chosen, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Ghoulraiser returned {name} to hand"));
        }
    }
}

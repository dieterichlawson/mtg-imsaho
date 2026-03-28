use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Slayer of the Wicked — {3}{W} 3/2 Human Soldier. ETB: destroy target Vampire, Werewolf, or Zombie.
pub struct SlayerOfTheWicked;

impl CardBehavior for SlayerOfTheWicked {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Slayer of the Wicked".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "When Slayer of the Wicked enters the battlefield, you may destroy target Vampire, Werewolf, or Zombie.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        // Find an opponent's Vampire/Werewolf/Zombie creature.
        let targets: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller != controller && o.power.is_some())
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie"))
                    .unwrap_or(false)
            })
            .map(|o| o.id)
            .collect();
        if let Some(&target) = targets.first() {
            state.move_object(target, Zone::Graveyard);
            state.log(crate::state::LogLevel::Event, format!("Slayer of the Wicked destroyed a creature"));
        }
    }
}

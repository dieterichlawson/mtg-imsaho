use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
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
            oracle_text: "When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "destroy target Vampire, Werewolf, or Zombie".into(),
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // "Target Vampire, Werewolf, or Zombie" — any controller, not just opponent.
        let targets: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != object_id)
            .filter(|o| {
                let target_subtypes = ["Vampire", "Werewolf", "Zombie"];
                let registry_has = registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| target_subtypes.contains(&s.as_str())))
                    .unwrap_or(false);
                let instance_has = o.subtypes.iter().any(|s| target_subtypes.contains(&s.as_str()));
                registry_has || instance_has
            })
            .map(|o| Target::Object(o.id))
            .collect();
        // "You may" — always present choice.
        crate::cards::helpers::present_optional_target_choice(
            state, object_id, controller, targets,
            PendingEffect::Destroy { source_name: "Slayer of the Wicked".into() },
            "Slayer of the Wicked: you may destroy target Vampire, Werewolf, or Zombie",
        );
    }
}

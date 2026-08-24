use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "destroy target Vampire, Werewolf, or Zombie".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // "Target Vampire, Werewolf, or Zombie" — any controller, not just opponent.
        let targets: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != object_id)
            .filter(|o| ["Vampire", "Werewolf", "Zombie"].iter()
                .any(|st| state.has_subtype(o.id, st, registry)))
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

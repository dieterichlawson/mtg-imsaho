use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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
                    // CR 603.3d: declaring the requirement makes the engine
                    // lock the target as the trigger goes on the stack, which
                    // is also where hexproof and protection are filtered out.
                    target_requirement: Some(crate::cards::TargetRequirement::CreatureWithFilter(
                        crate::cards::TargetFilter::SubtypeOrCardType {
                            subtypes: vec!["Vampire".into(), "Werewolf".into(), "Zombie".into()],
                            card_types: vec![],
                        },
                    )),
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, chosen_targets: &[Target], _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // CR 603.3d: the target was chosen when the trigger went on the stack
        // and its legality re-checked before resolution. Only the "you may"
        // decision is left — offer the locked target, never a fresh pick.
        let Some(target) = chosen_targets.first().cloned() else { return };
        crate::cards::helpers::present_optional_target_choice(
            state, object_id, controller, vec![target],
            PendingEffect::Destroy { source_name: "Slayer of the Wicked".into() },
            "Slayer of the Wicked: you may destroy the targeted creature",
        );
    }
}

use crate::cards::{AttackInfo, CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};
use crate::actions::Target;

/// Hamlet Captain — {1}{G} 2/2 Human Warrior.
/// Whenever this creature attacks or blocks, other Humans you control
/// get +1/+1 until end of turn.
pub struct HamletCaptain;

impl CardBehavior for HamletCaptain {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Hamlet Captain".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Warrior".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "other Humans get +1/+1 until end of turn".into(),
                target_requirement: None,
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::Blocks,
                    description: "other Humans get +1/+1 until end of turn".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, _attack: AttackInfo, _chosen_targets: &[Target], registry: &CardRegistry) {
        Self::buff_humans(state, self_id, registry);
    }

    fn on_blocks(&self, state: &mut GameState, self_id: ObjectId, _blocked_attacker: ObjectId, registry: &CardRegistry) {
        Self::buff_humans(state, self_id, registry);
    }
}

impl HamletCaptain {
    fn buff_humans(state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        // CR 113.7a: the trigger resolves whether or not the Captain survived
        // it, and CR 608.2g says "you" is then its last known controller. This
        // used to bail outright if the Captain had left the battlefield, so
        // killing it in response to its own attack trigger cancelled the pump
        // for the rest of the team. Nothing in "other Humans you control get
        // +1/+1 until end of turn" is about the Captain.
        let controller = crate::cards::helpers::controller_of(state, self_id);

        // Find other Human creatures you control.
        let humans: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller).into_iter()
            .filter(|o| {
                state.is_creature(o.id, registry)
                    && o.id != self_id
            })
            // `registry.card_data` always returns FRONT-face data, so a
            // transformed werewolf still looked Human and got buffed. CR 712.8d:
            // a DFC on the battlefield has only its current face's
            // characteristics — `has_subtype` reads that face.
            .filter(|o| state.has_subtype(o.id, "Human", registry))
            .map(|o| o.id)
            .collect();

        for id in &humans {
            state.until_end_of_turn.push(
                crate::state::TemporaryEffect::ModifyPT {
                    target: *id,
                    power_mod: 1,
                    toughness_mod: 1,
                }
            );
        }

        if !humans.is_empty() {
            state.log(crate::state::LogLevel::Event,
                format!("Hamlet Captain: {} other Humans get +1/+1 until end of turn", humans.len()));
        }
    }
}

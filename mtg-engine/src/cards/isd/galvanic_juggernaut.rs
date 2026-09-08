use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope};
use crate::actions::Target;

/// Galvanic Juggernaut — {4} 5/5 Artifact Creature — Juggernaut.
/// Galvanic Juggernaut attacks each combat if able.
/// Galvanic Juggernaut doesn't untap during your untap step.
/// Whenever another creature dies, untap Galvanic Juggernaut.
pub struct GalvanicJuggernaut;

impl CardBehavior for GalvanicJuggernaut {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Galvanic Juggernaut".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
            ])),
            card_types: vec![CardType::Artifact, CardType::Creature],
            subtypes: vec!["Juggernaut".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "This creature attacks each combat if able.\nThis creature doesn't untap during your untap step.\nWhenever another creature dies, untap this creature.".into(),
            continuous_effects: vec![
                ContinuousEffect::ForceAttack {
                    scope: EffectScope::OnSelf,
                },
                ContinuousEffect::PreventUntap {
                    scope: EffectScope::OnSelf,
                },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "untap Galvanic Juggernaut".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], _registry: &CardRegistry) {
        // CR 400.7: a Juggernaut that has left the battlefield is a different
        // object, and there is nothing there to untap.
        // `untap` is itself the guard — it is a no-op on an untapped
        // permanent (CR 701.20a) and says so — so the hand-rolled "was it
        // tapped" check this used to keep is the same question asked twice.
        if state.untap(self_id) {
            state.log(crate::state::LogLevel::Event,
                "Galvanic Juggernaut untapped (creature died)".to_string());
        }
    }
}

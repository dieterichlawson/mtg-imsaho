use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec!["Juggernaut".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "Galvanic Juggernaut attacks each combat if able.\nGalvanic Juggernaut doesn't untap during your untap step.\nWhenever another creature dies, untap Galvanic Juggernaut.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ForceAttack {
                    scope: EffectScope::OnSelf,
                },
                ContinuousEffect::PreventUntap {
                    scope: EffectScope::OnSelf,
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "untap Galvanic Juggernaut".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _registry: &CardRegistry) {
        if let Some(obj) = state.get_object_mut(self_id) {
            if obj.zone == Zone::Battlefield && obj.tapped {
                obj.tapped = false;
                state.log(crate::state::LogLevel::Event,
                    "Galvanic Juggernaut untapped (creature died)".to_string());
            }
        }
    }
}

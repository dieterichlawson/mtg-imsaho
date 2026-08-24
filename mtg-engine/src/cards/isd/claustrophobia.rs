use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry, TriggeredAbilityDef, TriggerKind};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope};

/// Claustrophobia — {1}{U}{U} aura enchantment. When Claustrophobia enters the battlefield,
/// tap enchanted creature. Enchanted creature doesn't untap during its controller's untap step.
pub struct Claustrophobia;

impl CardBehavior for Claustrophobia {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Claustrophobia".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into()],
            oracle_text: "Enchant creature\nWhen this Aura enters, tap enchanted creature.\nEnchanted creature doesn't untap during its controller's untap step.".into(),
            continuous_effects: vec![
                ContinuousEffect::PreventUntap { scope: EffectScope::Attached },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "tap enchanted creature".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets, registry);
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], _registry: &CardRegistry) {
        if let Some(target_id) = state.get_object(object_id).and_then(|o| o.attached_to) {
            if let Some(target) = state.get_object_mut(target_id) {
                target.tapped = true;
            }
            state.log(crate::state::LogLevel::Event,
                "Claustrophobia taps enchanted creature".to_string());
        }
    }
}

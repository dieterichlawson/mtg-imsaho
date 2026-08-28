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

    /// "When this Aura enters, tap enchanted creature."
    ///
    /// `attached_creature` rather than `o.attached_to`, because CR 113.7a says
    /// the trigger resolves whether or not the Aura survived it and CR 608.2g
    /// says "enchanted creature" is then the one it was last attached to.
    /// Leaving the battlefield clears `attached_to`, so destroying
    /// Claustrophobia in response to its own enters trigger used to mean the
    /// creature was never tapped at all.
    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], _registry: &CardRegistry) {
        if let Some(target_id) = state.attached_creature(object_id) {
            state.tap(target_id);
            state.log(crate::state::LogLevel::Event,
                "Claustrophobia taps enchanted creature".to_string());
        }
    }
}

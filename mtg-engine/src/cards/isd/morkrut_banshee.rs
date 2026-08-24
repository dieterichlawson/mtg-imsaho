use crate::cards::helpers;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};
use crate::actions::Target;

/// Morkrut Banshee — 4/4 for {3}{B}{B}. Spirit.
/// Morbid — When Morkrut Banshee enters the battlefield, if a creature died this turn,
/// target creature gets -4/-4 until end of turn.
pub struct MorkrutBanshee;

impl CardBehavior for MorkrutBanshee {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Morkrut Banshee".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Spirit".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Morbid — When this creature enters, if a creature died this turn, target creature gets -4/-4 until end of turn.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "if morbid, target creature gets -4/-4 until end of turn".into(),
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::Creature),
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn should_trigger(&self, state: &GameState, _self_id: ObjectId, kind: &TriggerKind, _registry: &CardRegistry) -> bool {
        helpers::morbid_should_trigger(state, kind)
    }

    /// Enforce morbid at target selection: if no creature died this turn,
    /// no creature is a legal target, so the trigger is removed per CR 603.3c.
    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, _target: &Target, _registry: &CardRegistry) -> bool {
        state.creature_died_this_turn
    }

    fn on_enter_battlefield(&self, state: &mut GameState, _object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // Morbid — only apply if a creature died this turn (also enforced at
        // stack-time via is_valid_target).
        if !state.creature_died_this_turn {
            return;
        }
        // CR 603.3d: target was chosen when the trigger went on the stack.
        let Some(target) = chosen_targets.first() else { return };
        let effect = PendingEffect::DebuffUntilEOT {
            power: -4,
            toughness: -4,
            source_name: "Morkrut Banshee".into(),
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}

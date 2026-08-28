use crate::cards::helpers;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
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

    /// Morbid is an intervening-if (CR 603.4), so it belongs in
    /// `should_trigger` and is checked again here on resolution — the ruling
    /// says as much: "If no creatures have died by the time it enters the
    /// battlefield, its ability won't trigger at all."
    ///
    /// It used to be an `is_valid_target` that ignored its target and answered
    /// `state.creature_died_this_turn`, on the reasoning that no legal target
    /// removes the trigger under CR 603.3c. That reaches the same board state
    /// by the wrong route: 603.3c puts the ability on the stack and then takes
    /// it off, and the engine says so in the game log, where 603.4 means it
    /// never triggered. It also left the card asserting that *any* object is a
    /// legal target for it, which is only harmless because the engine's
    /// `TargetRequirement::Creature` is checked alongside it. Reaper from the
    /// Abyss had the same thing and lost it; this one was missed.
    fn on_enter_battlefield(&self, state: &mut GameState, _object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 603.4 checks the intervening-if a second time on resolution.
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

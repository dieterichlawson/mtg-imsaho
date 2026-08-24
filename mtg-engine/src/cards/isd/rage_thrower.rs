use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Rage Thrower — {5}{R} 4/2 Human Shaman.
/// Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.
pub struct RageThrower;

impl CardBehavior for RageThrower {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rage Thrower".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Shaman".into()],
            power: Some(4),
            toughness: Some(2),
            oracle_text: "Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "deal 2 damage to target player or planeswalker".into(),
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::PlayerOrPlaneswalker),
                },
            ],
            ..Default::default()
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, chosen_targets: &[Target], registry: &CardRegistry) {
        // No battlefield check. A death trigger fires even when its watcher
        // died in the same event, and the damage is dealt to a player, not to
        // or from anything that has to still be there — CR 608.2h: a source
        // that has left the battlefield still deals its damage, using last
        // known information. Requiring the Thrower to survive made the trigger
        // a no-op in the board-wipe case it exists for.
        //
        // (Contrast Lumberknot and Unruly Mob nearby, whose triggers put a
        // counter on *themselves*: a permanent that has left the battlefield
        // can't receive counters, CR 121.1, so those guards are right.)
        //
        // CR 603.3d: target was chosen when the trigger went on the stack.
        let Some(target) = chosen_targets.first() else { return };
        let effect = PendingEffect::DealDamage {
            amount: 2,
            source_id: self_id,
            source_name: "Rage Thrower".into(),
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}

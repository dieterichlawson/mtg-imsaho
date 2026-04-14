use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Shaman".into()],
            power: Some(4),
            toughness: Some(2),
            oracle_text: "Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "deal 2 damage to target player or planeswalker".into(),
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::PlayerOrPlaneswalker),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, chosen_targets: &[Target], registry: &CardRegistry) {
        // Must still be on battlefield to deal damage.
        if !state.get_object(self_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            return;
        }
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

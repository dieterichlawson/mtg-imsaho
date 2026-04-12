use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::*;

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
                    description: "deal 2 damage to target player".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // "Target player or planeswalker" — include both players and planeswalkers.
        let mut targets: Vec<Target> = state.players.iter()
            .filter(|p| !p.lost)
            .filter(|p| !state.player_has_hexproof(p.id, registry) || p.id == controller)
            .map(|p| Target::Player(p.id))
            .collect();
        // Add planeswalkers on the battlefield as valid targets.
        for obj in state.objects.values() {
            if obj.zone == Zone::Battlefield && obj.card_types.contains(&CardType::Planeswalker) {
                targets.push(Target::Object(obj.id));
            }
        }
        crate::cards::helpers::present_target_choice(
            state, self_id, controller, targets,
            PendingEffect::DealDamage { amount: 2, source_id: self_id, source_name: "Rage Thrower".into() },
            "Rage Thrower: deal 2 damage to target player",
            false,
        );
    }
}

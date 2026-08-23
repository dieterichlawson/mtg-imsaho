use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Ashmouth Hound — {1}{R} 2/1 Elemental Hound.
/// Whenever this creature blocks or becomes blocked by a creature,
/// this creature deals 1 damage to that creature.
pub struct AshmouthHound;

impl CardBehavior for AshmouthHound {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ashmouth Hound".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Elemental".into(), "Dog".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Blocks,
                    description: "deal 1 damage to blocked creature".into(),
                target_requirement: None,
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::BecomesBlocked,
                    description: "deal 1 damage to blocking creature".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn on_blocks(&self, state: &mut GameState, self_id: ObjectId, blocked_attacker: ObjectId, registry: &CardRegistry) {
        // When Ashmouth Hound blocks: deal 1 damage to the creature it's blocking.
        deal_1_damage(state, self_id, blocked_attacker, registry);
    }

    fn on_becomes_blocked(&self, state: &mut GameState, self_id: ObjectId, blocker_id: ObjectId, registry: &CardRegistry) {
        // When Ashmouth Hound becomes blocked: deal 1 damage to the blocking creature.
        deal_1_damage(state, self_id, blocker_id, registry);
    }
}

fn deal_1_damage(state: &mut GameState, source: ObjectId, target: ObjectId, registry: &CardRegistry) {
    crate::damage::deal_damage(state, source,
        crate::events::DamageTarget::Object(target), 1,
        crate::damage::DamageKind::NonCombat, registry);
}

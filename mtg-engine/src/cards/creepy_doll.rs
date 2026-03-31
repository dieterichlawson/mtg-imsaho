use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;
use rand::Rng;

/// Creepy Doll — {5} 1/1 Artifact Creature — Construct with Indestructible.
/// Whenever Creepy Doll deals combat damage to a creature, flip a coin.
/// If you win the flip, destroy that creature.
pub struct CreepyDoll;

impl CardBehavior for CreepyDoll {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Creepy Doll".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
            ])),
            card_types: vec![CardType::Artifact, CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Construct".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Indestructible\nWhenever Creepy Doll deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.".into(),
            keywords: vec![Keyword::Indestructible],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "flip a coin; if you win, destroy that creature".into(),
                },
            ],
        }
    }

    fn on_blocks(&self, state: &mut GameState, self_id: ObjectId, blocked_attacker: ObjectId, registry: &CardRegistry) {
        // Creepy Doll deals combat damage to the creature it blocks.
        // We handle the coin flip here when blocking (simplified approach).
        if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
            return;
        }
        let won = rand::thread_rng().gen_bool(0.5);
        if won {
            state.log(crate::state::LogLevel::Event,
                "Creepy Doll won the coin flip! Destroying blocked creature.".to_string());
            crate::destruction::try_destroy(state, blocked_attacker, registry);
        } else {
            state.log(crate::state::LogLevel::Event,
                "Creepy Doll lost the coin flip.".to_string());
        }
    }
}

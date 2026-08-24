use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Keyword, Zone};
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
            subtypes: vec!["Construct".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Indestructible\nWhenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.".into(),
            keywords: vec![Keyword::Indestructible],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::DealsCombatDamageToCreature,
                    description: "flip a coin; if you win, destroy that creature".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_deals_combat_damage_to_creature(&self, state: &mut GameState, self_id: ObjectId, damaged_creature: ObjectId, _amount: u32, registry: &CardRegistry) {
        if state.get_object(self_id).is_none_or(|o| o.zone != Zone::Battlefield) {
            return;
        }
        let won = rand::thread_rng().gen_bool(0.5);
        if won {
            let name = state.get_object(damaged_creature).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("Creepy Doll won the coin flip! Destroying {name}"));
            crate::destruction::try_destroy(state, damaged_creature, registry);
        } else {
            state.log(crate::state::LogLevel::Event,
                "Creepy Doll lost the coin flip.".to_string());
        }
    }
}

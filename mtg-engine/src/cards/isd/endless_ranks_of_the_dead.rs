use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Endless Ranks of the Dead — {2}{B}{B} Enchantment.
/// At the beginning of your upkeep, create X 2/2 black Zombie creature tokens,
/// where X is half the number of Zombies you control, rounded down.
pub struct EndlessRanksOfTheDead;

impl CardBehavior for EndlessRanksOfTheDead {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Endless Ranks of the Dead".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "create Zombie tokens".into(),
                },
            ],
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        // Count Zombies you control.
        let zombie_count = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.power.is_some()) // creatures only
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
                    .unwrap_or(false)
                || o.subtypes.iter().any(|s| s == "Zombie")
            })
            .count();
        let tokens_to_create = zombie_count / 2;
        for _ in 0..tokens_to_create {
            state.create_token_with_subtypes(
                "Zombie",
                controller,
                2, 2,
                vec![Color::Black],
                vec![CardType::Creature],
                vec![],
                vec!["Zombie".into()],
                registry,
            );
        }
        if tokens_to_create > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Endless Ranks of the Dead: created {} Zombie token(s)", tokens_to_create));
        }
    }
}

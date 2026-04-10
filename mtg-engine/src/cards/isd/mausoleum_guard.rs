use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Mausoleum Guard — {3}{W} 2/2 Human Scout. When it dies, create two 1/1 white Spirit tokens with flying.
pub struct MausoleumGuard;

impl CardBehavior for MausoleumGuard {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mausoleum Guard".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Scout".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "When this creature dies, create two 1/1 white Spirit creature tokens with flying.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "create two 1/1 white Spirit tokens with flying".into(),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        // Use controller (not owner) — if the creature was stolen, tokens go to the controller.
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        for _ in 0..2 {
            state.create_token_with_subtypes("Spirit", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()], registry);
        }
        state.log(crate::state::LogLevel::Event,
            "Mausoleum Guard: created two 1/1 white Spirit tokens with flying".into());
    }
}

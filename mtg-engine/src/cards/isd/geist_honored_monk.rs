use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Geist-Honored Monk — {3}{W}{W} */* Human Monk. Vigilance.
/// Power and toughness are each equal to the number of creatures you control.
/// When Geist-Honored Monk enters the battlefield, create two 1/1 white Spirit creature tokens with flying.
pub struct GeistHonoredMonk;

impl CardBehavior for GeistHonoredMonk {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Geist-Honored Monk".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Monk".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Vigilance\nGeist-Honored Monk's power and toughness are each equal to the number of creatures you control.\nWhen this creature enters, create two 1/1 white Spirit creature tokens with flying.".into(),
            keywords: vec![Keyword::Vigilance],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "create two 1/1 white Spirit tokens with flying".into(),
                },
            ],
        }
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        let controller = state.get_object(object_id)?.controller;
        let creature_count = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == controller && o.power.is_some())
            .count() as i32;
        Some((creature_count, creature_count))
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        for _ in 0..2 {
            state.create_token_with_subtypes("Spirit", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()], registry);
        }
    }
}

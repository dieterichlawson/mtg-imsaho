use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};
use crate::actions::Target;

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
            subtypes: vec!["Human".into(), "Monk".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Vigilance\nGeist-Honored Monk's power and toughness are each equal to the number of creatures you control.\nWhen this creature enters, create two 1/1 white Spirit creature tokens with flying.".into(),
            keywords: vec![Keyword::Vigilance],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "create two 1/1 white Spirit tokens with flying".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Option<(i32, i32)> {
        let controller = state.get_object(object_id)?.controller;
        let creature_count = i32::try_from(state.objects_in_zone(Zone::Battlefield, controller).into_iter()
            .filter(|o| state.is_creature(o.id, registry))
            .count()).unwrap_or(i32::MAX);
        Some((creature_count, creature_count))
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);
        for _ in 0..2 {
            state.create_token_with_subtypes("", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()], registry);
        }
        state.log(crate::state::LogLevel::Event,
            "Geist-Honored Monk: created two 1/1 white Spirit tokens with flying".into());
    }
}

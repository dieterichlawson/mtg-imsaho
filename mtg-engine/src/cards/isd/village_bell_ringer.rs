use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};
use crate::actions::Target;

/// Village Bell-Ringer — {2}{W} 1/4 Human Scout. Flash. When this creature enters, untap all creatures you control.
pub struct VillageBellRinger;

impl CardBehavior for VillageBellRinger {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Village Bell-Ringer".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Scout".into()],
            power: Some(1),
            toughness: Some(4),
            oracle_text: "Flash (You may cast this spell any time you could cast an instant.)\nWhen this creature enters, untap all creatures you control.".into(),
            keywords: vec![Keyword::Flash],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "untap all creatures you control".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);
        let to_untap: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller).into_iter()
            .filter(|o| state.is_creature(o.id, registry) && o.tapped)
            .map(|o| o.id)
            .collect();
        for id in to_untap {
            state.get_object_mut(id).unwrap().tapped = false;
        }
    }
}

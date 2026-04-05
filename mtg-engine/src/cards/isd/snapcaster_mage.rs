use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Snapcaster Mage — {1}{U} 2/1 Human Wizard. Flash.
/// When this creature enters, target instant or sorcery card in your graveyard
/// gains flashback until end of turn. The flashback cost is equal to its mana cost.
///
/// Simplified: On ETB, finds the best instant/sorcery in the graveyard and
/// grants it flashback until end of turn.
pub struct SnapcasterMage;

impl CardBehavior for SnapcasterMage {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Snapcaster Mage".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Flash\nWhen this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![Keyword::Flash],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "grant flashback to instant or sorcery in graveyard".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Find all eligible instant/sorcery cards in graveyard without innate flashback.
        let eligible: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller)
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| {
                        (d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery))
                            && d.flashback_cost.is_none()
                    })
                    .unwrap_or(false)
            })
            .filter(|o| !state.until_end_of_turn_flashback.iter().any(|(id, _)| *id == o.id))
            .map(|o| o.id)
            .collect();

        if eligible.is_empty() {
            return;
        }

        if eligible.len() == 1 {
            // Only one legal option — auto-select.
            let target_id = eligible[0];
            let cost = registry.card_data(state.get_object(target_id).unwrap().card_id)
                .and_then(|d| d.cost.clone())
                .unwrap_or(ManaCost::free());
            state.until_end_of_turn_flashback.push((target_id, cost));
            let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("Snapcaster Mage grants flashback to {}", name));
        } else {
            // Multiple eligible — player chooses via ChooseTarget.
            let targets: Vec<crate::actions::Target> = eligible.iter()
                .map(|&id| crate::actions::Target::Object(id))
                .collect();
            crate::cards::helpers::present_target_choice(
                state, object_id, controller, targets,
                crate::state::PendingEffect::GrantFlashback { source_name: "Snapcaster Mage".into() },
                "Snapcaster Mage: choose an instant or sorcery to grant flashback",
                false,
            );
        }
    }
}

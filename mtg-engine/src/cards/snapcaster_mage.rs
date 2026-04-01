use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Snapcaster Mage — {1}{U} 2/1 Human Wizard. Flash.
/// When Snapcaster Mage enters the battlefield, target instant or sorcery card
/// in your graveyard gains flashback until end of turn. The flashback cost is
/// equal to its mana cost.
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
            oracle_text: "Flash\nWhen Snapcaster Mage enters the battlefield, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.".into(),
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

        // Find all instants/sorceries in graveyard without flashback.
        let targets: Vec<crate::actions::Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller)
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| {
                        let is_instant_or_sorcery = d.card_types.contains(&CardType::Instant)
                            || d.card_types.contains(&CardType::Sorcery);
                        is_instant_or_sorcery && d.flashback_cost.is_none()
                            && !state.until_end_of_turn_flashback.iter().any(|(id, _)| *id == o.id)
                    })
                    .unwrap_or(false)
            })
            .map(|o| crate::actions::Target::Object(o.id))
            .collect();

        if targets.len() == 1 {
            // Only one valid target — auto-grant flashback.
            if let crate::actions::Target::Object(target_id) = targets[0] {
                let cost = state.get_object(target_id)
                    .and_then(|o| registry.card_data(o.card_id))
                    .and_then(|d| d.cost.clone())
                    .unwrap_or(ManaCost::free());
                state.until_end_of_turn_flashback.push((target_id, cost));
                let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("Snapcaster Mage grants flashback to {}", name));
            }
        } else if targets.len() > 1 {
            // Multiple targets — let the player choose.
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Snapcaster Mage: choose an instant or sorcery to grant flashback".into(),
                    options: targets,
                    optional: false,
                    effect: PendingEffect::GrantFlashback,
                },
            });
        }
    }
}

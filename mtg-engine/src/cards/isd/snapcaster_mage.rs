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
            oracle_text: "Flash\nWhen this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.".into(),
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

        // Find an instant or sorcery in the graveyard that doesn't already have flashback.
        let target = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller)
            .filter_map(|o| {
                registry.card_data(o.card_id).and_then(|d| {
                    let is_instant_or_sorcery = d.card_types.contains(&CardType::Instant)
                        || d.card_types.contains(&CardType::Sorcery);
                    if is_instant_or_sorcery && d.flashback_cost.is_none() {
                        // Prefer the highest mana value card (most powerful to reuse).
                        let cost = d.cost.as_ref().map(|c| c.mana_value()).unwrap_or(0);
                        Some((o.id, d.cost.clone().unwrap_or(ManaCost::free()), cost))
                    } else {
                        None
                    }
                })
            })
            .max_by_key(|(_, _, mv)| *mv);

        if let Some((target_id, cost, _)) = target {
            // Also check if it's already been granted dynamic flashback.
            let already_has = state.until_end_of_turn_flashback.iter()
                .any(|(id, _)| *id == target_id);
            if !already_has {
                state.until_end_of_turn_flashback.push((target_id, cost));
                let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("Snapcaster Mage grants flashback to {}", name));
            }
        }
    }
}

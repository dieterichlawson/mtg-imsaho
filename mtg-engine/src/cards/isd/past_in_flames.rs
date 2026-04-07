use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Past in Flames — {3}{R} Sorcery.
/// Each instant and sorcery card in your graveyard gains flashback until end of turn.
/// The flashback cost is equal to its mana cost.
/// Flashback {4}{R}.
pub struct PastInFlames;

impl CardBehavior for PastInFlames {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Past in Flames".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.\nFlashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Move Past in Flames itself to graveyard/exile first, so it's in the graveyard
        // and can also get flashback if we want (but it already has flashback printed).
        state.move_spell_after_resolve(object_id);

        // Grant flashback to each instant and sorcery in the graveyard.
        let targets: Vec<(ObjectId, ManaCost)> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.id != object_id)
            .filter_map(|o| {
                registry.card_data(o.card_id).and_then(|d| {
                    let is_instant_or_sorcery = d.card_types.contains(&CardType::Instant)
                        || d.card_types.contains(&CardType::Sorcery);
                    if is_instant_or_sorcery {
                        Some((o.id, d.cost.clone().unwrap_or(ManaCost::free())))
                    } else {
                        None
                    }
                })
            })
            .collect();

        let mut count = 0;
        for (target_id, cost) in targets {
            // Don't duplicate if already granted flashback.
            let already_has = state.until_end_of_turn.iter()
                .any(|e| matches!(e, crate::state::TemporaryEffect::GrantFlashback { target, .. } if *target == target_id));
            if !already_has {
                state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantFlashback { target: target_id, cost });
                count += 1;
            }
        }

        if count > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Past in Flames grants flashback to {} instant/sorcery cards", count));
        }
    }
}

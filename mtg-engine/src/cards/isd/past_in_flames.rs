use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            oracle_text: "Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.\nFlashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // "Each instant and sorcery card in your graveyard gains flashback until
        // end of turn." Read at resolution — a card put into the graveyard later
        // in the turn does not gain it. Past in Flames itself is still on the
        // stack here, and the engine moves it afterwards, so it is not in its
        // own list.
        let targets: Vec<(ObjectId, ManaCost)> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && state.is_card(o.id) && o.id != object_id)
            .filter_map(|o| {
                state.face_data(o.id, registry).and_then(|d| {
                    let is_instant_or_sorcery = d.card_types.contains(&CardType::Instant)
                        || d.card_types.contains(&CardType::Sorcery);
                    // CR 702.33a: the flashback cost is "equal to its mana
                    // cost", so a card with no mana cost gains no usable
                    // flashback — it is skipped, not given a free cost.
                    if is_instant_or_sorcery {
                        d.cost.clone().map(|c| (o.id, c))
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
                format!("Past in Flames grants flashback to {count} instant/sorcery cards"));
        }
    }
}

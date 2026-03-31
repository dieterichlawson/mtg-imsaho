use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
use crate::types::*;

/// Divine Reckoning — {2}{W}{W} Sorcery.
/// Each player chooses a creature they control, then sacrifices the rest.
/// Flashback {5}{W}{W}.
pub struct DivineReckoning;

impl CardBehavior for DivineReckoning {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Divine Reckoning".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Each player chooses a creature they control, then sacrifices the rest.\nFlashback {5}{W}{W}".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // For each player, auto-keep the creature with the highest toughness.
        // (Simplification: in real MTG, each player would choose.)
        let player_ids: Vec<PlayerId> = state.players.iter().map(|p| p.id).collect();

        for player_id in player_ids {
            let creatures: Vec<(ObjectId, i32)> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield && o.controller == player_id && o.power.is_some())
                .map(|o| {
                    let toughness = state.effective_toughness(o.id, registry).unwrap_or(0);
                    (o.id, toughness)
                })
                .collect();

            if creatures.len() <= 1 {
                // 0 or 1 creatures — nothing to sacrifice.
                continue;
            }

            // Find the creature with the highest toughness to keep.
            let keeper = creatures.iter()
                .max_by_key(|&&(_, t)| t)
                .map(|&(id, _)| id)
                .unwrap();

            let keeper_name = state.get_object(keeper).map(|o| o.name.clone()).unwrap_or_default();
            state.log(LogLevel::Event, format!("p{} keeps {}", player_id.0, keeper_name));

            // Sacrifice everything else.
            let to_sac: Vec<ObjectId> = creatures.iter()
                .filter(|&&(id, _)| id != keeper)
                .map(|&(id, _)| id)
                .collect();

            for sac_id in to_sac {
                crate::destruction::sacrifice(state, sac_id, registry);
            }
        }

        state.move_spell_after_resolve(object_id);
    }
}

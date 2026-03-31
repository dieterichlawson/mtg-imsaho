use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Reaper from the Abyss — {3}{B}{B}{B} 6/6 flying Demon.
/// Morbid — At the beginning of each end step, if a creature died this turn,
/// destroy target non-Demon creature.
pub struct ReaperFromTheAbyss;

impl CardBehavior for ReaperFromTheAbyss {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Reaper from the Abyss".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Demon".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Flying\nMorbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EndStep,
                    description: "destroy target non-Demon creature".into(),
                },
            ],
        }
    }

    fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Morbid — only trigger if a creature died this turn.
        if !state.creature_died_this_turn {
            return;
        }
        // Destroy target non-Demon creature.
        // Auto-target: pick an opponent's non-Demon creature if possible.
        let opponent = state.opponent(controller);
        let target = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != self_id)
            .filter(|o| {
                let is_demon = registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Demon"))
                    .unwrap_or(false)
                    || o.subtypes.iter().any(|s| s == "Demon");
                !is_demon
            })
            // Prefer opponent's creatures.
            .min_by_key(|o| if o.controller == opponent { 0 } else { 1 })
            .map(|o| o.id);
        if let Some(target_id) = target {
            let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, target_id, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Reaper from the Abyss: destroyed {}", name));
        }
    }
}

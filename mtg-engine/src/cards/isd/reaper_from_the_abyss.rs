use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

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
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EndStep,
                    description: "destroy target non-Demon creature".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Morbid — only trigger if a creature died this turn.
        if !state.creature_died_this_turn {
            return;
        }
        // Collect non-Demon creatures as targets.
        let targets: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != self_id)
            .filter(|o| {
                let is_demon = registry.card_data(o.card_id)
                    .is_some_and(|d| d.subtypes.iter().any(|s| s == "Demon"))
                    || o.subtypes.iter().any(|s| s == "Demon");
                !is_demon
            })
            .map(|o| Target::Object(o.id))
            .collect();
        // Present choice to controller.
        crate::cards::helpers::present_target_choice(
            state, self_id, controller, targets,
            PendingEffect::DestroyCreature { source_name: "Reaper from the Abyss".into() },
            "Reaper from the Abyss: destroy target non-Demon creature",
            false,
        );
    }
}

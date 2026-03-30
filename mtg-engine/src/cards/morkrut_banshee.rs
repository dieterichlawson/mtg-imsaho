use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Morkrut Banshee — 4/4 for {3}{B}{B}. Spirit.
/// Morbid — When Morkrut Banshee enters the battlefield, if a creature died this turn,
/// target creature gets -4/-4 until end of turn.
pub struct MorkrutBanshee;

impl CardBehavior for MorkrutBanshee {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Morkrut Banshee".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Morbid — When Morkrut Banshee enters the battlefield, if a creature died this turn, target creature gets -4/-4 until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "if morbid, target creature gets -4/-4 until end of turn".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        if !state.creature_died_this_turn {
            return;
        }

        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Can target ANY creature (including your own).
        let targets: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != object_id)
            .map(|o| Target::Object(o.id))
            .collect();

        if targets.is_empty() {
            // No valid targets, do nothing.
        } else if targets.len() == 1 {
            // Auto-apply to the only target.
            if let Target::Object(target_id) = targets[0] {
                let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.until_end_of_turn_effects.push(
                    crate::state::UntilEndOfTurnEffect {
                        target: target_id,
                        power_mod: -4,
                        toughness_mod: -4,
                    }
                );
                state.log(LogLevel::Event,
                    format!("Morkrut Banshee's morbid — {} gets -4/-4 until end of turn", name));
            }
        } else {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Morkrut Banshee: target creature gets -4/-4 until end of turn".into(),
                    options: targets,
                    optional: false,
                    effect: PendingEffect::DebuffUntilEOT { power: -4, toughness: -4, source_name: "Morkrut Banshee".into() },
                },
            });
        }
    }
}

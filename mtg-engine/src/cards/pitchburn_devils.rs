use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Pitchburn Devils — {4}{R} 3/3 Devil. When it dies, deal 3 damage to any target.
pub struct PitchburnDevils;

impl CardBehavior for PitchburnDevils {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Pitchburn Devils".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Devil".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "When Pitchburn Devils dies, it deals 3 damage to any target.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "deal 3 damage to any target".into(),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let opponent = state.opponent(controller);

        let mut options: Vec<Target> = Vec::new();
        // Add opponent creatures.
        for obj in state.objects.values() {
            if obj.zone == Zone::Battlefield && obj.power.is_some() && obj.controller == opponent {
                options.push(Target::Object(obj.id));
            }
        }
        // Add opponent player.
        options.push(Target::Player(opponent));
        // Add own creatures (Pitchburn can target any target).
        for obj in state.objects.values() {
            if obj.zone == Zone::Battlefield && obj.power.is_some() && obj.controller == controller && obj.id != object_id {
                options.push(Target::Object(obj.id));
            }
        }
        // Add self as player target.
        options.push(Target::Player(controller));

        if options.len() <= 1 {
            // Auto-resolve: damage the only target.
            if let Some(target) = options.first() {
                match target {
                    Target::Player(pid) => {
                        let old = state.get_player(*pid).life;
                        state.get_player_mut(*pid).life = old - 3;
                        state.events.push(GameEvent::LifeChanged { player: *pid, old, new_life: old - 3 });
                    }
                    Target::Object(id) => {
                        if let Some(obj) = state.get_object_mut(*id) {
                            obj.damage_marked += 3;
                        }
                    }
                }
                state.log(LogLevel::Event, "Pitchburn Devils dealt 3 damage".into());
            }
        } else {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Pitchburn Devils: deal 3 damage to any target".into(),
                    options,
                    optional: false,
                    effect: PendingEffect::DealDamage { amount: 3, source_name: "Pitchburn Devils".into() },
                },
            });
        }
    }
}

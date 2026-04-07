use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::*;

/// Frightful Delusion — {2}{U} instant. Counter target spell unless its controller pays {1}.
/// That player discards a card.
pub struct FrightfulDelusion;

impl CardBehavior for FrightfulDelusion {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Frightful Delusion".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Counter target spell unless its controller pays {1}. That player discards a card.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Spell
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Stack)
                    .unwrap_or(false)
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Stack {
                    let controller = obj.controller;
                    let can_pay = state.get_player(controller).mana_pool.total() >= 1;

                    if can_pay {
                        // Opponent has mana -- ask them to choose.
                        let spell_name = state.obj_name(*target_id);
                        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                            player: controller,
                            source: object_id,
                            choice: ResolutionChoiceKind::PayOrNot {
                                description: format!("Pay {{1}} to prevent {} from being countered?", spell_name),
                                spell_id: *target_id,
                                source_spell_id: object_id,
                            },
                        });
                        return; // Don't clean up yet
                    }

                    // Can't pay -- auto-counter.
                    let countered_name = state.obj_name(*target_id);
                    state.stack.retain(|e| e.as_spell() != Some(*target_id));
                    state.move_spell_after_resolve(*target_id, registry);
                    state.log(LogLevel::Event, format!("{} was countered", countered_name));

                    // Force discard — player chooses which card.
                    let hand: Vec<_> = state.objects_in_zone(Zone::Hand, controller)
                        .iter().map(|o| o.id).collect();
                    if hand.len() == 1 {
                        state.move_object(hand[0], Zone::Graveyard, registry);
                        state.events.push(crate::events::GameEvent::Discarded { player: controller, object: hand[0] });
                        state.log(LogLevel::Event, format!("p{} discarded a card", controller.0));
                    } else if !hand.is_empty() {
                        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                            player: controller,
                            source: object_id,
                            choice: ResolutionChoiceKind::ChooseCardFromHand {
                                description: "Frightful Delusion: choose a card to discard".into(),
                                player: controller,
                                cards: hand,
                            },
                        });
                        // Move spell to graveyard before the discard choice.
                        state.move_spell_after_resolve(object_id, registry);
                        return;
                    }
                }
            }
        }
        state.move_spell_after_resolve(object_id, registry);
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::engine::draw_cards;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Curiosity — {U} Aura. Enchant creature.
/// Whenever enchanted creature deals damage to an opponent, you may draw a card.
pub struct Curiosity;

impl CardBehavior for Curiosity {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curiosity".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant creature\nWhenever enchanted creature deals damage to an opponent, you may draw a card.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyDamageToPlayer,
                    description: "you may draw a card".into(),
                },
            ],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets, registry);
    }

    fn on_any_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, source_id: ObjectId, damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        // "Whenever enchanted creature deals damage to an opponent"
        let aura = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return,
        };
        let Some(attached_to) = aura.attached_to else { return; };
        // Only trigger if the source is the enchanted creature.
        if source_id != attached_to {
            return;
        }
        // Only trigger if damage was dealt to an opponent (not the controller).
        let controller = aura.controller;
        if damaged_player == controller {
            return;
        }
        // "You may draw a card" — present choice to the player.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description: "Curiosity: draw a card?".into(),
                source_card: self_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            return;
        }
        let controller = state.get_object(self_id)
            .map_or(PlayerId(0), |o| o.controller);
        draw_cards(state, controller, 1, registry);
        state.log(LogLevel::Event, "Curiosity: drew a card".into());
    }
}

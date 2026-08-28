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
            subtypes: vec!["Aura".into()],
            oracle_text: "Enchant creature\nWhenever enchanted creature deals damage to an opponent, you may draw a card.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyDamageToPlayer,
                    description: "you may draw a card".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets, registry);
    }

    /// "Whenever enchanted creature deals damage to an opponent" — CR 603.2:
    /// both halves of that are part of the triggering event, so they are read
    /// at dispatch. Checking them at resolution instead meant the ability went
    /// on the stack every time ANY permanent damaged ANY player and then
    /// quietly did nothing, handing everyone a priority window each time.
    fn should_trigger_on_damage_to_player(&self, state: &GameState, self_id: ObjectId, source_id: ObjectId, damaged_player: PlayerId, _registry: &CardRegistry) -> bool {
        let Some(aura) = state.get_object(self_id).filter(|o| o.zone == Zone::Battlefield) else {
            return false;
        };
        // The source must be the enchanted creature, and the damage must have
        // gone to an opponent.
        aura.attached_to == Some(source_id) && damaged_player != aura.controller
    }

    fn on_any_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, _source_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        // CR 113.7a: the draw happens even if Curiosity is destroyed in
        // response to its own trigger — and CR 608.2g says the "you" is then
        // the player who last controlled it, which is what `controller_of`
        // answers. Reading `o.controller` gave the owner in that one case,
        // because leaving the battlefield resets the field.
        let controller = crate::cards::helpers::controller_of(state, self_id);
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
        let controller = crate::cards::helpers::controller_of(state, self_id);
        let _ = draw_cards(state, controller, 1, registry);
        state.log(LogLevel::Event, "Curiosity: drew a card".into());
    }
}

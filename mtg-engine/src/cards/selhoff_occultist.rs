use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Selhoff Occultist — {2}{U} 2/3 Human Rogue.
/// Whenever Selhoff Occultist or another creature dies, target player mills a card.
pub struct SelhoffOccultist;

impl CardBehavior for SelhoffOccultist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Selhoff Occultist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Rogue".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "Whenever Selhoff Occultist or another creature dies, target player mills a card.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "target player mills a card".into(),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "target player mills a card".into(),
                },
            ],
        }
    }

    /// When Selhoff Occultist itself dies, target player mills a card.
    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        present_mill_choice(state, object_id, controller);
    }

    /// When another creature dies, target player mills a card.
    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        present_mill_choice(state, self_id, controller);
    }
}

fn present_mill_choice(state: &mut GameState, source_id: ObjectId, controller: PlayerId) {
    let options: Vec<Target> = state.players.iter()
        .map(|p| Target::Player(p.id))
        .collect();

    if options.is_empty() {
        return;
    }

    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: controller,
        source: source_id,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: "Selhoff Occultist: target player mills a card".into(),
            options,
            optional: false,
            effect: PendingEffect::Mill { count: 1, source_name: "Selhoff Occultist".into() },
        },
    });
}

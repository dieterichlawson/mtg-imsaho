use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
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
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Spell
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Stack)
                    .unwrap_or(false)
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Stack {
                    let controller = obj.controller;
                    let name = obj.name.clone();
                    state.stack.retain(|&id| id != *target_id);
                    state.move_object(*target_id, Zone::Graveyard);
                    state.log(LogLevel::Event, format!("{} was countered", name));

                    // Force discard
                    let hand: Vec<_> = state.objects_in_zone(Zone::Hand, controller)
                        .iter().map(|o| o.id).collect();
                    if let Some(&card) = hand.first() {
                        state.move_object(card, Zone::Graveyard);
                        state.log(LogLevel::Event, format!("p{} discarded a card", controller.0));
                    }
                }
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

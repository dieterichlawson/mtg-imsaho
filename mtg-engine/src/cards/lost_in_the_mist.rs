use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
use crate::types::*;

/// Lost in the Mist — {3}{U}{U} instant. Counter target spell. Return target permanent to its
/// owner's hand.
pub struct LostInTheMist;

impl CardBehavior for LostInTheMist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Lost in the Mist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Counter target spell. Return target permanent to its owner's hand.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::TwoTargets(
            Box::new(TargetRequirement::Spell),
            Box::new(TargetRequirement::PermanentWithFilter("any".into())),
        )
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Stack || o.zone == Zone::Battlefield)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        // Counter the spell (first target)
        if let Some(Target::Object(spell_id)) = targets.first() {
            if let Some(obj) = state.get_object(*spell_id) {
                if obj.zone == Zone::Stack {
                    let name = obj.name.clone();
                    state.stack.retain(|&id| id != *spell_id);
                    state.move_spell_after_resolve(*spell_id);
                    state.log(LogLevel::Event, format!("{} was countered", name));
                }
            }
        }
        // Bounce the permanent (second target)
        if let Some(Target::Object(perm_id)) = targets.get(1) {
            if let Some(obj) = state.get_object(*perm_id) {
                if obj.zone == Zone::Battlefield {
                    let name = obj.name.clone();
                    state.move_object(*perm_id, Zone::Hand);
                    state.log(LogLevel::Event, format!("{} was returned to hand", name));
                }
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

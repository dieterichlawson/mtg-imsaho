use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            oracle_text: "Counter target spell. Return target permanent to its owner's hand.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::TwoTargets(
            Box::new(TargetRequirement::Spell),
            Box::new(TargetRequirement::PermanentWithFilter(TargetFilter::Any)),
        )
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .is_some_and(|o| o.zone == Zone::Stack || o.zone == Zone::Battlefield)
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // Counter the spell (first target)
        if let Some(Target::Object(spell_id)) = targets.first() {
            if let Some(obj) = state.get_object(*spell_id) {
                if obj.zone == Zone::Stack {
                    let countered_name = state.obj_name(*spell_id);
                    state.stack.retain(|e| e.as_spell() != Some(*spell_id));
                    state.move_countered_spell(*spell_id, registry);
                    state.log(LogLevel::Event, format!("{countered_name} was countered"));
                }
            }
        }
        // Bounce the permanent (second target)
        if let Some(Target::Object(perm_id)) = targets.get(1) {
            if let Some(obj) = state.get_object(*perm_id) {
                if obj.zone == Zone::Battlefield {
                    let bounced_name = state.obj_name(*perm_id);
                    state.move_object(*perm_id, Zone::Hand, registry);
                    state.log(LogLevel::Event, format!("{bounced_name} was returned to hand"));
                }
            }
        }
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Travel Preparations — {1}{G} sorcery. Put a +1/+1 counter on each of up to two target creatures.
/// Simplified: targets one creature and puts a +1/+1 counter on it.
/// TODO: implement multi-target casting flow for "up to two targets".
pub struct TravelPreparations;

impl CardBehavior for TravelPreparations {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Travel Preparations".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Put a +1/+1 counter on each of up to two target creatures.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)])),
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                state.add_counters(*target_id, CounterType::PlusOnePlusOne, 1);
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

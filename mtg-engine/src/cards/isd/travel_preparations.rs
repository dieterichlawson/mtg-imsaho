use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};

/// Travel Preparations — {1}{G} sorcery. Put a +1/+1 counter on each of up to two target creatures.
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
            oracle_text: "Put a +1/+1 counter on each of up to two target creatures.\nFlashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature))
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        for target in targets {
            if let Target::Object(target_id) = target {
                if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                    state.add_counters(*target_id, CounterType::PlusOnePlusOne, 1);
                }
            }
        }
        state.move_spell_after_resolve(object_id, registry);
    }
}

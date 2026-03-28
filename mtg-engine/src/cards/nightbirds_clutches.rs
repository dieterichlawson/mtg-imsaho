use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Nightbird's Clutches — {1}{R} sorcery. Up to two target creatures can't block this turn.
pub struct NightbirdsClutches;

impl CardBehavior for NightbirdsClutches {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Nightbird's Clutches".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Up to two target creatures can't block this turn.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)])),
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature))
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        for target in targets {
            if let Target::Object(target_id) = target {
                if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                    state.until_end_of_turn_cant_block.push(*target_id);
                    let name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
                    state.log(crate::state::LogLevel::Event, format!("{} can't block this turn", name));
                }
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

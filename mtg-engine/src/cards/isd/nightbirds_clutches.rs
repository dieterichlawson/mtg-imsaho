use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            oracle_text: "Up to two target creatures can't block this turn.\nFlashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature))
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        for target in targets {
            if let Target::Object(target_id) = target {
                if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                    state.until_end_of_turn.push(crate::state::TemporaryEffect::CantBlock { target: *target_id });
                    state.log(crate::state::LogLevel::Event, format!("{} can't block this turn", state.obj_name(*target_id)));
                }
            }
        }
    }
}

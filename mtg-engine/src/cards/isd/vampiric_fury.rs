use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{GameState, TemporaryEffect};
use crate::types::*;

/// Vampiric Fury — {1}{R} instant. Vampire creatures you control get +2/+0 and gain first strike until end of turn.
pub struct VampiricFury;

impl CardBehavior for VampiricFury {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Vampiric Fury".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Vampire creatures you control get +2/+0 and gain first strike until end of turn. (They deal combat damage before creatures without first strike.)".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // Find the controller of this spell.
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        // Build a registry to look up subtypes.

        let vampire_filter = Some(crate::types::CreatureFilter::HasSubtype("Vampire".into()));
        state.until_end_of_turn.push(
            crate::state::TemporaryEffect::ModifyPTAll {
                controller,
                filter: vampire_filter.clone(),
                power_mod: 2,
                toughness_mod: 0,
            }
        );
        state.until_end_of_turn.push(
            TemporaryEffect::GrantKeywordAll {
                controller,
                filter: vampire_filter,
                keyword: Keyword::FirstStrike,
            }
        );

        state.move_spell_after_resolve(object_id, registry);
    }
}

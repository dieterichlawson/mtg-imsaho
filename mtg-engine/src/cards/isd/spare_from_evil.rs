use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Spare from Evil — {1}{W} instant.
/// Creatures you control gain protection from non-Human creatures until end of turn.
pub struct SpareFromEvil;

impl CardBehavior for SpareFromEvil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Spare from Evil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Creatures you control gain protection from non-Human creatures until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        // Grant protection from non-Human creatures until end of turn.
        let filter = CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into())));
        state.until_end_of_turn.push(
            crate::state::TemporaryEffect::GrantProtectionAll {
                controller,
                protection_filter: filter,
            }
        );

        state.log(crate::state::LogLevel::Event,
            "Spare from Evil: creatures gain protection from non-Human creatures until end of turn".into());

        state.move_spell_after_resolve(object_id, registry);
    }
}

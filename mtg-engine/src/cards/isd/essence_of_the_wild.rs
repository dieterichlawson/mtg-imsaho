use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Essence of the Wild {3}{G}{G}{G} 6/6 Avatar.
/// Creatures you control enter as a copy of Essence of the Wild.
///
/// This is a replacement effect (CR 614.1d): the creature never exists in its
/// original form on the battlefield. Implemented by setting `entering_copy_source`
/// on the GameObject so that `GameState::move_object` applies the copy replacement
/// before the `EnteredBattlefield` event is emitted.
pub struct EssenceOfTheWild;

impl CardBehavior for EssenceOfTheWild {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Essence of the Wild".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Avatar".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Creatures you control enter as a copy of this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        // Mark this permanent as an entering-battlefield copy source.
        // Any creature entering the battlefield under the same controller
        // will enter as a copy of this permanent instead.
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.entering_copy_source = true;
            obj.instance_oracle_text = Some("Creatures you control enter as a copy of this creature.".into());
            // Populate copiable values on the object so that the replacement
            // effect in move_object can read them without the registry.
            obj.subtypes = vec!["Avatar".into()];
            obj.colors = vec![Color::Green];
        }
    }
}

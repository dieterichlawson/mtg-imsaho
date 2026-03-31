use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Stitched Drake — {1}{U}{U} 3/4 Zombie Drake with Flying.
/// As an additional cost to cast Stitched Drake, exile a creature card from your graveyard.
pub struct StitchedDrake;

impl CardBehavior for StitchedDrake {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stitched Drake".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Drake".into()],
            power: Some(3),
            toughness: Some(4),
            oracle_text: "Flying\nAs an additional cost to cast Stitched Drake, exile a creature card from your graveyard.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Exile a creature card from graveyard as additional cost.
        let exile_candidate = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                    .unwrap_or(o.power.is_some())
            })
            .map(|o| o.id)
            .next();

        if let Some(exile_id) = exile_candidate {
            let name = state.get_object(exile_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(exile_id, Zone::Exile);
            state.log(crate::state::LogLevel::Event,
                format!("Stitched Drake exiled {} from graveyard", name));
        }

        state.move_object(object_id, Zone::Battlefield);
    }
}

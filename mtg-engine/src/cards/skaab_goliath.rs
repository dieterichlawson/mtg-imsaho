use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Skaab Goliath — {5}{U} 6/9 Zombie Giant with Trample.
/// As an additional cost to cast Skaab Goliath, exile two creature cards from your graveyard.
pub struct SkaabGoliath;

impl CardBehavior for SkaabGoliath {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Skaab Goliath".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Giant".into()],
            power: Some(6),
            toughness: Some(9),
            oracle_text: "Trample\nAs an additional cost to cast Skaab Goliath, exile two creature cards from your graveyard.".into(),
            keywords: vec![Keyword::Trample],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(2)),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Exile two creature cards from graveyard as additional cost.
        let exile_candidates: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                    .unwrap_or(o.power.is_some())
            })
            .map(|o| o.id)
            .take(2)
            .collect();

        for exile_id in &exile_candidates {
            let name = state.get_object(*exile_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*exile_id, Zone::Exile);
            state.log(crate::state::LogLevel::Event,
                format!("Skaab Goliath exiled {} from graveyard", name));
        }

        state.move_object(object_id, Zone::Battlefield);
    }
}

use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Skaab Ruinator — {1}{U}{U} 5/6 Zombie Horror with Flying.
/// As an additional cost to cast, exile three creature cards from your graveyard.
/// You may cast Skaab Ruinator from your graveyard.
pub struct SkaabRuinator;

impl CardBehavior for SkaabRuinator {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Skaab Ruinator".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Horror".into()],
            power: Some(5),
            toughness: Some(6),
            oracle_text: "Flying\nAs an additional cost to cast Skaab Ruinator, exile three creature cards from your graveyard.\nYou may cast Skaab Ruinator from your graveyard.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(3)),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Exile three creature cards from graveyard as additional cost.
        let exile_candidates: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                o.id != object_id && // Don't exile self
                registry.card_data(o.card_id)
                    .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                    .unwrap_or(o.power.is_some())
            })
            .map(|o| o.id)
            .take(3)
            .collect();

        for exile_id in &exile_candidates {
            let name = state.get_object(*exile_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*exile_id, Zone::Exile);
            state.log(crate::state::LogLevel::Event,
                format!("Skaab Ruinator exiled {} from graveyard", name));
        }

        state.move_object(object_id, Zone::Battlefield);
    }
}

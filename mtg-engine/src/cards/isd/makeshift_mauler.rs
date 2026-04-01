use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Makeshift Mauler — {3}{U} 4/5 Zombie.
/// As an additional cost to cast Makeshift Mauler, exile a creature card from your graveyard.
pub struct MakeshiftMauler;

impl CardBehavior for MakeshiftMauler {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Makeshift Mauler".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Horror".into()],
            power: Some(4),
            toughness: Some(5),
            oracle_text: "As an additional cost to cast Makeshift Mauler, exile a creature card from your graveyard.".into(),
            keywords: vec![],
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
                format!("Makeshift Mauler exiled {} from graveyard", name));
        }

        state.move_object(object_id, Zone::Battlefield);
    }
}

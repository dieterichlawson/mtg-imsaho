use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Back from the Brink — {4}{U}{U} Enchantment.
/// Exile a creature card from your graveyard and pay its mana cost:
/// Create a token that's a copy of that card. Activate only as a sorcery.
pub struct BackFromTheBrink;

impl CardBehavior for BackFromTheBrink {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Back from the Brink".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone != Zone::Battlefield {
            return vec![];
        }
        let controller = obj.controller;
        let creatures: Vec<_> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| o.power.is_some())
            .map(|o| (o.id, o.card_id, o.name.clone()))
            .collect();

        creatures.iter().enumerate().map(|(i, (_obj_id, card_id, name))| {
            let cost = registry.card_data(*card_id)
                .and_then(|d| d.cost.clone())
                .unwrap_or_else(|| ManaCost::new(vec![]));
            ActivatedAbilityDef {
                ability_index: i,
                description: format!("Exile {} from graveyard, create token copy (pay {})", name, cost),
                cost,
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: true,
            }
        }).collect()
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // The ability_index corresponds to the creature's position in the graveyard list.
        let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| o.power.is_some())
            .map(|o| o.id)
            .collect();

        if let Some(&creature_id) = creatures.get(ability_index) {
            back_from_brink_exile(state, creature_id, controller, registry);
        }
    }
}

fn back_from_brink_exile(state: &mut GameState, creature_id: ObjectId, controller: crate::ids::PlayerId, registry: &CardRegistry) {
    let name = state.get_object(creature_id).map(|o| o.name.clone()).unwrap_or_default();
    state.create_token_copy(creature_id, controller, registry);
    state.move_object(creature_id, Zone::Exile);
    state.log(crate::state::LogLevel::Event,
        format!("Back from the Brink: exiled {} from graveyard, created token copy", name));
}

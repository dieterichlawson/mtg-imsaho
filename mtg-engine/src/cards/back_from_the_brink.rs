use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Back from the Brink — {4}{U}{U} Enchantment.
/// Exile a creature card from your graveyard and pay its mana cost:
/// Create a token that's a copy of that card. Activate only as a sorcery.
///
/// Simplified: activated ability exiles the first creature from graveyard and
/// creates a token copy. The mana cost requirement is approximated by a high
/// generic cost.
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

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone != Zone::Battlefield {
            return vec![];
        }
        // Only show ability if there's a creature in the graveyard.
        // NOTE: Oracle says "pay its mana cost" but activated_abilities doesn't have
        // access to the registry to look up the creature's cost. Using Generic(2) as
        // an approximation. A proper fix requires adding registry to the trait method.
        let controller = obj.controller;
        let has_creature = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .any(|o| o.power.is_some());
        if !has_creature {
            return vec![];
        }
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Exile creature from graveyard, create token copy".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Generic(2),
            ]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: true,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(_object_id) {
            Some(o) => o.controller,
            None => return,
        };
        // Find first creature in graveyard.
        let creature_id = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .find(|o| o.power.is_some())
            .map(|o| o.id);
        let creature_id = match creature_id {
            Some(id) => id,
            None => return,
        };

        let name = state.get_object(creature_id).map(|o| o.name.clone()).unwrap_or_default();

        // Create a token copy.
        state.create_token_copy(creature_id, controller, registry);

        // Exile the creature card.
        state.move_object(creature_id, Zone::Exile);

        state.log(crate::state::LogLevel::Event,
            format!("Back from the Brink: exiled {} from graveyard, created token copy", name));
    }
}

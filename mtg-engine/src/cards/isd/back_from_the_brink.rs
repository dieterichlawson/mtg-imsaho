use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Back from the Brink — {4}{U}{U} Enchantment.
/// Exile a creature card from your graveyard and pay its mana cost:
/// Create a token that's a copy of that card. Activate only as a sorcery.
///
/// Implementation: generates one activated ability per creature card in the
/// controller's graveyard. Each ability's mana cost matches the creature's
/// mana cost, and the ability_index encodes the creature's ObjectId so that
/// on_activate_ability can identify which creature to exile.
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

        // Generate one ability per creature card in the controller's graveyard.
        let creatures: Vec<_> = state.objects_in_zone(Zone::Graveyard, controller)
            .into_iter()
            .filter(|o| {
                // A creature card: check the object's power (set for creature cards)
                // or fall back to the registry card data.
                o.power.is_some()
                    || registry.card_data(o.card_id)
                        .map(|d| d.card_types.contains(&CardType::Creature))
                        .unwrap_or(false)
            })
            .collect();

        creatures.into_iter().map(|creature| {
            // Look up the creature's mana cost from the registry.
            let mana_cost = registry.card_data(creature.card_id)
                .and_then(|d| d.cost.clone())
                .unwrap_or_else(|| ManaCost::new(vec![]));

            // Use the creature's ObjectId as the ability_index so we can
            // identify which creature to exile in on_activate_ability.
            let ability_index = creature.id.0 as usize;

            ActivatedAbilityDef {
                ability_index,
                description: format!(
                    "Exile {} from graveyard, pay its mana cost, create a token copy",
                    creature.name
                ),
                cost: mana_cost,
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: true,
            }
        }).collect()
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(_object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // The ability_index encodes the ObjectId of the creature to exile.
        let creature_id = ObjectId(ability_index as u64);

        // Verify the creature is still in the graveyard and belongs to the controller.
        let valid = state.get_object(creature_id)
            .map(|o| o.zone == Zone::Graveyard && o.owner == controller)
            .unwrap_or(false);
        if !valid {
            return;
        }

        let name = state.get_object(creature_id).map(|o| o.name.clone()).unwrap_or_default();

        // Exile the creature card first (part of the cost — everything before the colon).
        // Per oracle: "Exile a creature card from your graveyard and pay its mana cost:"
        state.move_object(creature_id, Zone::Exile);

        // Create a token copy (the effect — after the colon).
        state.create_token_copy(creature_id, controller, registry);

        state.log(crate::state::LogLevel::Event,
            format!("Back from the Brink: exiled {} from graveyard, created token copy", name));
    }
}

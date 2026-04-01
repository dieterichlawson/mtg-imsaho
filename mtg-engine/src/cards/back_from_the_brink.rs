use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
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

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Find creatures in graveyard — present choice if multiple.
        let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| o.power.is_some())
            .map(|o| o.id)
            .collect();

        if creatures.len() == 1 {
            back_from_brink_exile(state, creatures[0], controller, registry);
        } else if creatures.len() > 1 {
            let options: Vec<Target> = creatures.iter()
                .map(|&id| Target::Object(id))
                .collect();
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Back from the Brink: choose a creature to exile and copy".into(),
                    options,
                    optional: false,
                    effect: PendingEffect::CardCallbackWithTarget { source_id: object_id },
                },
            });
        }
    }

    fn on_target_chosen(&self, state: &mut GameState, self_id: ObjectId, target: &Target, registry: &CardRegistry) {
        let controller = state.get_object(self_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        if let Target::Object(creature_id) = target {
            back_from_brink_exile(state, *creature_id, controller, registry);
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

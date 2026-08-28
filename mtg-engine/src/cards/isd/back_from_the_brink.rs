use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Back from the Brink — {4}{U}{U} Enchantment.
/// Exile a creature card from your graveyard and pay its mana cost:
/// Create a token that's a copy of that card. Activate only as a sorcery.
///
/// Implementation: generates one activated ability per creature card in the
/// controller's graveyard. Each ability's mana cost matches the creature's
/// mana cost, and the `ability_index` encodes the creature's `ObjectId` so that
/// `pay_activation_cost` can identify which creature to exile.
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
            oracle_text: "Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone != Zone::Battlefield {
            return vec![];
        }
        let controller = obj.controller;

        // Generate one ability per creature card in the controller's graveyard.
        let creatures: Vec<_> = state.objects_in_zone(Zone::Graveyard, controller)
            .into_iter()
            .filter(|o| {
                // "exile a creature **card**" — CR 109.1: a token in a graveyard
                // is not a card, and is only there until the next SBA check.
                //
                // `&&` binds tighter than `||`, so the second half of this used
                // to read `... || face_data says Creature` with no `is_card`
                // guard at all — saying the opposite of the comment above it.
                // It was also redundant: `is_creature` already consults the
                // active face's card types.
                state.is_card(o.id) && state.is_creature(o.id, registry)
            })
            .collect();

        creatures.into_iter().map(|creature| {
            // Look up the creature's mana cost from the registry. X is 0 in a
            // mana cost paid other than by casting the spell, and there is no
            // announcement (CR 107.3e) — without this the engine saw {X} in
            // the ability's cost and put the player through the X-funding
            // prompt, letting them dump mana into a value that can only be 0.
            let mana_cost = state.face_data(creature.id, registry)
                .and_then(|d| d.cost.clone())
                .unwrap_or_else(ManaCost::free)
                .without_x();

            // Use the creature's ObjectId as the ability_index so we can
            // identify which creature to exile when the cost is paid.
            let ability_index = usize::try_from(creature.id.0).unwrap_or(usize::MAX);

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
                counter_cost: None,
            }
        }).collect()
    }

    fn pay_activation_cost(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        let creature_id = ObjectId(ability_index as u64);

        let valid = state.get_object(creature_id)
            .is_some_and(|o| o.zone == Zone::Graveyard && o.owner == controller);
        if !valid {
            return;
        }

        // Exile the creature card (cost — before the colon in oracle text).
        state.move_object(creature_id, Zone::Exile, _registry);

    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        // CR 602.2a: an activated ability's controller is the player who
        // activated it, which the engine records; CR 608.2g falls back to the
        // source's last known controller. Reading `o.controller` here gave the
        // *current* controller, so an opponent taking the permanent in
        // response to the ability collected the effect — and `None => return`
        // threw the whole effect away if the source had left, against
        // CR 113.7a.
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        let creature_id = ObjectId(ability_index as u64);
        let name = state.get_object(creature_id).map(|o| o.name.clone()).unwrap_or_default();

        state.create_token_copy(creature_id, controller, registry);

        state.log(crate::state::LogLevel::Event,
            format!("Back from the Brink: exiled {name} from graveyard, created token copy"));
    }
}

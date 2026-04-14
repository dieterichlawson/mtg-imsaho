use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{Zone, CardType, ManaType};
use crate::actions::Target;

/// Sulfur Falls — Land.
/// This land enters tapped unless you control an Island or a Mountain.
/// {T}: Add {U} or {R}.
pub struct SulfurFalls;

impl SulfurFalls {
    fn controller_has_matching_land(state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> bool {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return false,
        };
        state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .any(|o| {
                if o.id == object_id {
                    return false;
                }
                let has_subtype = |name: &str| {
                    o.subtypes.iter().any(|s| s == name)
                        || registry.card_data(o.card_id)
                            .is_some_and(|d| d.subtypes.iter().any(|s| s == name))
                };
                has_subtype("Island") || has_subtype("Mountain")
            })
    }
}

impl CardBehavior for SulfurFalls {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sulfur Falls".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "This land enters tapped unless you control an Island or a Mountain.\n{T}: Add {U} or {R}.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "enters tapped unless you control an Island or a Mountain".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        if !Self::controller_has_matching_land(state, object_id, registry) {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.tapped = true;
            }
            state.log(crate::state::LogLevel::Info, "Sulfur Falls enters tapped".into());
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![
                ManaAbilityDef {
                    ability_index: 0,
                    description: "Add {U}".into(),
                    produced: vec![(ManaType::Blue, 1)],
                    requires_tap: true,
                    has_side_effects: false,
                },
                ManaAbilityDef {
                    ability_index: 1,
                    description: "Add {R}".into(),
                    produced: vec![(ManaType::Red, 1)],
                    requires_tap: true,
                    has_side_effects: false,
                },
            ]
        } else {
            vec![]
        }
    }
}

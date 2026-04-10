use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Hinterland Harbor — Land.
/// Hinterland Harbor enters the battlefield tapped unless you control a Forest or an Island.
/// {T}: Add {G} or {U}.
pub struct HinterlandHarbor;

impl HinterlandHarbor {
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
                            .map_or(false, |d| d.subtypes.iter().any(|s| s == name))
                };
                has_subtype("Forest") || has_subtype("Island")
            })
    }
}

impl CardBehavior for HinterlandHarbor {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Hinterland Harbor".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "This land enters tapped unless you control a Forest or an Island.\n{T}: Add {G} or {U}.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        if !Self::controller_has_matching_land(state, object_id, registry) {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.tapped = true;
            }
            state.log(crate::state::LogLevel::Info, "Hinterland Harbor enters tapped".into());
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![
                ManaAbilityDef {
                    ability_index: 0,
                    description: "Add {G}".into(),
                    produced: vec![(ManaType::Green, 1)],
                    requires_tap: true,
                    has_side_effects: false,
                },
                ManaAbilityDef {
                    ability_index: 1,
                    description: "Add {U}".into(),
                    produced: vec![(ManaType::Blue, 1)],
                    requires_tap: true,
                    has_side_effects: false,
                },
            ]
        } else {
            vec![]
        }
    }
}

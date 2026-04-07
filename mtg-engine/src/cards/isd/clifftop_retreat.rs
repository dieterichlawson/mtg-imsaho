use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Clifftop Retreat — Land.
/// This land enters tapped unless you control a Mountain or a Plains.
/// {T}: Add {R} or {W}.
pub struct ClifftopRetreat;

impl ClifftopRetreat {
    fn controller_has_matching_land(state: &GameState, object_id: ObjectId) -> bool {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return false,
        };
        state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .any(|o| {
                o.id != object_id
                    && (o.subtypes.iter().any(|s| s == "Mountain")
                        || o.subtypes.iter().any(|s| s == "Plains"))
            })
    }
}

impl CardBehavior for ClifftopRetreat {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Clifftop Retreat".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "This land enters tapped unless you control a Mountain or a Plains.\n{T}: Add {R} or {W}.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        if !Self::controller_has_matching_land(state, object_id) {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.tapped = true;
            }
            state.log(crate::state::LogLevel::Info, "Clifftop Retreat enters tapped".into());
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
                    description: "Add {R}".into(),
                    produced: vec![(ManaType::Red, 1)],
                    requires_tap: true,
                    has_side_effects: false,
                },
                ManaAbilityDef {
                    ability_index: 1,
                    description: "Add {W}".into(),
                    produced: vec![(ManaType::White, 1)],
                    requires_tap: true,
                    has_side_effects: false,
                },
            ]
        } else {
            vec![]
        }
    }
}

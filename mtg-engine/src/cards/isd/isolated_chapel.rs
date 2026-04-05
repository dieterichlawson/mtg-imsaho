use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Isolated Chapel — Land.
/// Isolated Chapel enters the battlefield tapped unless you control a Plains or a Swamp.
/// {T}: Add {W} or {B}.
pub struct IsolatedChapel;

impl IsolatedChapel {
    fn controller_has_matching_land(state: &GameState, object_id: ObjectId) -> bool {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return false,
        };
        state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .any(|o| {
                o.id != object_id
                    && (o.subtypes.iter().any(|s| s == "Plains")
                        || o.subtypes.iter().any(|s| s == "Swamp"))
            })
    }
}

impl CardBehavior for IsolatedChapel {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Isolated Chapel".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "This land enters tapped unless you control a Plains or a Swamp.\n{T}: Add {W} or {B}.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        if !Self::controller_has_matching_land(state, object_id) {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.tapped = true;
            }
            state.log(crate::state::LogLevel::Info, "Isolated Chapel enters tapped".into());
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
                    description: "Add {W}".into(),
                    produced: vec![(ManaType::White, 1)],
                    requires_tap: true,
                },
                ManaAbilityDef {
                    ability_index: 1,
                    description: "Add {B}".into(),
                    produced: vec![(ManaType::Black, 1)],
                    requires_tap: true,
                },
            ]
        } else {
            vec![]
        }
    }
}

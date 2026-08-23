use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{Zone, CardType, ManaType};

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
                let has_subtype = |name: &str| state.has_subtype(o.id, name, registry);
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
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            // "Enters tapped unless ..." is a replacement effect (CR 614.1d),
            // declared via `enters_tapped` — not a triggered ability.
            triggered_abilities: vec![],
        }
    }

    /// CR 614.1d: "HinterlandHarbor enters tapped unless you control a Forest or a Island."
    /// A replacement effect — no stack entry, no window in which the land is
    /// briefly untapped and could be tapped for mana in response, and the
    /// condition is read at the moment of entry rather than at resolution
    /// (so an opponent cannot bounce the Forest in response to change it).
    fn enters_tapped(&self, state: &GameState, self_id: ObjectId, _from_zone: Option<Zone>, registry: &CardRegistry) -> bool {
        !Self::controller_has_matching_land(state, self_id, registry)
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
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

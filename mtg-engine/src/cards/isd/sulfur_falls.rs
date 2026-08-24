use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, Zone, CardType, ManaType};

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
                let has_subtype = |name: &str| state.has_subtype(o.id, name, registry);
                has_subtype("Island") || has_subtype("Mountain")
            })
    }
}

impl CardBehavior for SulfurFalls {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sulfur Falls".into(),
            card_types: vec![CardType::Land],
            oracle_text: "This land enters tapped unless you control an Island or a Mountain.\n{T}: Add {U} or {R}.".into(),
            // "Enters tapped unless ..." is a replacement effect (CR 614.1d),
            // declared via `enters_tapped` — not a triggered ability.,
            ..Default::default()
        }
    }

    /// CR 614.1d: "SulfurFalls enters tapped unless you control a Island or a Mountain."
    /// A replacement effect — no stack entry, no window in which the land is
    /// briefly untapped and could be tapped for mana in response, and the
    /// condition is read at the moment of entry rather than at resolution
    /// (so an opponent cannot bounce the Island in response to change it).
    fn enters_tapped(&self, state: &GameState, self_id: ObjectId, _from_zone: Option<Zone>, registry: &CardRegistry) -> bool {
        !Self::controller_has_matching_land(state, self_id, registry)
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![
            ManaAbilityDef {
                ability_index: 0,
                description: "Add {U}".into(),
                produced: vec![(ManaType::Blue, 1)],
                requires_tap: true,
                cost: ManaCost::free(),
                has_side_effects: false,
            },
            ManaAbilityDef {
                ability_index: 1,
                description: "Add {R}".into(),
                produced: vec![(ManaType::Red, 1)],
                requires_tap: true,
                cost: ManaCost::free(),
                has_side_effects: false,
            },
        ]
    }
}

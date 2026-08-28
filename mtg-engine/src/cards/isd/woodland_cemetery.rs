use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, Zone, CardType, ManaType};

/// Woodland Cemetery — Land.
/// This land enters tapped unless you control a Swamp or a Forest.
/// {T}: Add {B} or {G}.
pub struct WoodlandCemetery;

impl WoodlandCemetery {
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
                has_subtype("Swamp") || has_subtype("Forest")
            })
    }
}

impl CardBehavior for WoodlandCemetery {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Woodland Cemetery".into(),
            card_types: vec![CardType::Land],
            oracle_text: "This land enters tapped unless you control a Swamp or a Forest.\n{T}: Add {B} or {G}.".into(),
            // "Enters tapped unless ..." is a replacement effect (CR 614.1d),
            // not a triggered ability. It is declared by `replace_event`
            // below, which hands the condition to
            // `helpers::enters_tapped_unless`. (This comment used to name a
            // `CardData` field that has never existed.)
            ..Default::default()
        }
    }

    fn replace_event(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        crate::cards::helpers::enters_tapped_unless(self_id, event, || {
            Self::controller_has_matching_land(state, self_id, registry)
        })
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![
            ManaAbilityDef {
                ability_index: 0,
                description: "Add {B}".into(),
                produced: vec![(ManaType::Black, 1)],
                requires_tap: true,
                cost: ManaCost::free(),
                has_side_effects: false,
            },
            ManaAbilityDef {
                ability_index: 1,
                description: "Add {G}".into(),
                produced: vec![(ManaType::Green, 1)],
                requires_tap: true,
                cost: ManaCost::free(),
                has_side_effects: false,
            },
        ]
    }
}

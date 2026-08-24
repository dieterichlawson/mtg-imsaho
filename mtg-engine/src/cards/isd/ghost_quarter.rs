use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost, TargetRequirement, TargetFilter};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, Zone, ManaType, ManaCost, Supertype};

/// Ghost Quarter — Land.
/// {T}: Add {C}.
/// {T}, Sacrifice Ghost Quarter: Destroy target land. Its controller may search
/// their library for a basic land card, put it onto the battlefield, then shuffle.
pub struct GhostQuarter;

impl CardBehavior for GhostQuarter {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ghost Quarter".into(),
            card_types: vec![CardType::Land],
            oracle_text: "{T}: Add {C}.\n{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.".into(),
            ..Default::default()
        }
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}".into(),
            produced: vec![(ManaType::Colorless, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }]
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ActivatedAbilityDef {
                ability_index: 1,
                description: "{T}, Sacrifice: Destroy target land, its controller may search for a basic land".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::SacrificeThis,
                target_requirement: Some(TargetRequirement::PermanentWithFilter(
                    TargetFilter::HasCardType(vec![CardType::Land]),
                )),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            let (target_controller, target_name) = match state.get_object(*target_id) {
                Some(o) if o.zone == Zone::Battlefield => (o.controller, o.name.clone()),
                _ => return,
            };

            // Destroy target land.
            crate::destruction::try_destroy(state, *target_id, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Ghost Quarter destroyed {target_name}"));

            // "Its controller may search their library for a basic land card,
            // put it onto the battlefield, then shuffle."
            // Find all basic lands in the controller's library.
            let basic_lands: Vec<ObjectId> = state.get_player(target_controller).library_order.iter()
                .filter(|&&lib_id| {
                    state.get_object(lib_id)
                        .and_then(|o| state.face_data(o.id, registry))
                        .is_some_and(|d| {
                            d.card_types.contains(&CardType::Land)
                                && d.supertypes.contains(&Supertype::Basic)
                        })
                })
                .copied()
                .collect();

            // A controller with no basic lands is still offered the "may
            // search" — declining is their choice to make, and a player who
            // does not search does not shuffle. `search_library` handles that.
            //
            // "...put it onto the battlefield, then shuffle." The land goes
            // straight to the battlefield untapped, which is why this could
            // not use the engine's search before it carried a destination.
            crate::cards::helpers::search_library(
                state, object_id, target_controller, basic_lands,
                Zone::Battlefield, false, true,
                "Ghost Quarter: you may search for a basic land card",
                registry,
            );
        }
    }

}

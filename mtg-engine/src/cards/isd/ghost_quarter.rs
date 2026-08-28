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

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // No zone-or-tapped guard here: `legal_actions` enumerates only
        // battlefield permanents its player controls and rejects a
        // `requires_tap` ability on a tapped one, and it also applies the
        // summoning-sickness rule this never did (CR 302.6 — irrelevant to a
        // land, but a card should not be the place that decides).
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
    }

    /// CR 602.2a: activating an ability puts it on the stack; the effect
    /// happens when it resolves. Its ruling is the plain statement of the
    /// CR 608.2b half: "If the targeted land is an illegal target by the time
    /// Ghost Quarter's ability resolves, it won't resolve and none of its
    /// effects will happen. The land's controller won't get to search for a
    /// basic land card." 
    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            let Some(target_controller) = state.get_object(*target_id)
                .filter(|o| o.zone == Zone::Battlefield)
                .map(|o| o.controller) else { return };

            // Destroy target land.
            // The search happens either way (ruling 2013-07-01: "even if that
            // land wasn't destroyed... because the land has indestructible or
            // because it was regenerated"), so the result is only used to say
            // truthfully what happened.
            crate::destruction::try_destroy_by(state, *target_id, "Ghost Quarter", registry);

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
            );
        }
    }

}

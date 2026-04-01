use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost, TargetRequirement, TargetFilter};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind, YesNoEffect};
use crate::types::*;

/// Ghost Quarter — Land.
/// {T}: Add {C}.
/// {T}, Sacrifice Ghost Quarter: Destroy target land. Its controller may search
/// their library for a basic land card, put it onto the battlefield, then shuffle.
pub struct GhostQuarter;

impl CardBehavior for GhostQuarter {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ghost Quarter".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {C}.\n{T}, Sacrifice Ghost Quarter: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ManaAbilityDef {
                ability_index: 0,
                description: "Add {C}".into(),
                produced: vec![(ManaType::Colorless, 1)],
                requires_tap: true,
            }]
        } else {
            vec![]
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
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
                format!("Ghost Quarter destroyed {}", target_name));

            // "Its controller may search for a basic land" — present choice.
            let has_basic = state.get_player(target_controller).library_order.iter().any(|&lib_id| {
                state.get_object(lib_id)
                    .and_then(|o| registry.card_data(o.card_id))
                    .map(|d| {
                        d.card_types.contains(&CardType::Land)
                            && d.supertypes.contains(&Supertype::Basic)
                    })
                    .unwrap_or(false)
            });

            if has_basic {
                // Store target_controller's player ID in context so on_yes_choice knows who searches.
                state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                    player: target_controller,
                    source: object_id,
                    choice: ResolutionChoiceKind::YesNo {
                        description: "Ghost Quarter: search your library for a basic land?".into(),
                        source_card: object_id,
                        effect: YesNoEffect::CardCallback {
                            context: vec![ObjectId(target_controller.0 as u64)],
                        },
                    },
                });
            }
        }
    }

    fn on_yes_choice(&self, state: &mut GameState, _self_id: ObjectId, context: &[ObjectId], registry: &CardRegistry) {
        let searcher = crate::ids::PlayerId(context.first().map(|id| id.0 as u8).unwrap_or(0));

        let basic_land_id = state.get_player(searcher).library_order.iter().find(|&&lib_id| {
            state.get_object(lib_id)
                .and_then(|o| registry.card_data(o.card_id))
                .map(|d| {
                    d.card_types.contains(&CardType::Land)
                        && d.supertypes.contains(&Supertype::Basic)
                })
                .unwrap_or(false)
        }).copied();

        if let Some(land_id) = basic_land_id {
            state.get_player_mut(searcher).library_order.retain(|&id| id != land_id);
            let name = state.get_object(land_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(land_id, Zone::Battlefield);
            if let Some(obj) = state.get_object_mut(land_id) {
                obj.summoning_sick = false;
            }
            state.log(crate::state::LogLevel::Event,
                format!("Ghost Quarter: p{} searched for {}", searcher.0, name));

            // Shuffle library.
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(searcher).library_order.shuffle(&mut rng);
        }
    }
}

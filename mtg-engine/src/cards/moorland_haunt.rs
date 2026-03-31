use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Moorland Haunt — Land.
/// {T}: Add {C}.
/// {W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white
/// Spirit creature token with flying.
pub struct MoorlandHaunt;

impl CardBehavior for MoorlandHaunt {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moorland Haunt".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {C}.\n{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.".into(),
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
        if obj.zone != Zone::Battlefield || obj.tapped {
            return vec![];
        }

        let controller = obj.controller;

        // Check if there's a creature card in the graveyard to exile.
        let has_creature_in_graveyard = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .any(|o| o.power.is_some() && !o.is_token);

        if has_creature_in_graveyard {
            vec![ActivatedAbilityDef {
                ability_index: 1,
                description: "{W}{U}, {T}, Exile a creature from graveyard: Create 1/1 white Spirit with flying".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Colored(Color::White),
                    ManaSymbol::Colored(Color::Blue),
                ]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Exile a creature card from graveyard (auto-pick the first one).
        let creature_in_gy = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| o.power.is_some() && !o.is_token)
            .map(|o| o.id)
            .next();

        if let Some(exile_id) = creature_in_gy {
            let name = state.get_object(exile_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(exile_id, Zone::Exile);
            state.log(crate::state::LogLevel::Event,
                format!("Moorland Haunt exiled {} from graveyard", name));
        }

        // Create a 1/1 white Spirit creature token with flying.
        state.create_token_with_subtypes(
            "Spirit Token",
            controller,
            1, 1,
            vec![Color::White],
            vec![CardType::Creature],
            vec![Keyword::Flying],
            vec!["Spirit".into()],
        );
        state.log(crate::state::LogLevel::Event,
            "Moorland Haunt created a 1/1 white Spirit token with flying".into());
    }
}

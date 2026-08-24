use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, Zone, ManaType, ManaCost, ManaSymbol, Color, Keyword};

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

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}".into(),
            produced: vec![(ManaType::Colorless, 1)],
            requires_tap: true,
            has_side_effects: false,
        }]
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
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

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Exile a creature card from graveyard — player chooses which one.
        let creatures_in_gy: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| o.power.is_some() && !o.is_token)
            .map(|o| o.id)
            .collect();

        if creatures_in_gy.len() == 1 {
            let exile_id = creatures_in_gy[0];
            let name = state.get_object(exile_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(exile_id, Zone::Exile, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Moorland Haunt exiled {name} from graveyard"));
            state.create_token_with_subtypes(
                "Spirit Token", controller, 1, 1,
                vec![Color::White], vec![CardType::Creature],
                vec![Keyword::Flying], vec!["Spirit".into()], registry,
            );
            state.log(crate::state::LogLevel::Event,
                "Moorland Haunt created a 1/1 white Spirit token with flying".into());
        } else if creatures_in_gy.len() > 1 {
            let targets: Vec<Target> = creatures_in_gy.iter().map(|&id| Target::Object(id)).collect();
            crate::cards::helpers::present_target_choice(
                state, object_id, controller, targets,
                crate::state::PendingEffect::CardEffect { source_id: object_id, key: String::new() },
                "Moorland Haunt: choose a creature card from your graveyard to exile",
                false,
            );
        }
    }

    /// "...exile a creature card from your graveyard: Create a 1/1 white
    /// Spirit creature token with flying." The token's characteristics are
    /// this card's text, not the engine's business.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let controller = crate::cards::helpers::controller_of(state, source_id);
        let name = state.obj_name(*id);
        state.move_object(*id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Moorland Haunt exiled {name} from graveyard"));
        state.create_token_with_subtypes(
            "Spirit Token", controller, 1, 1,
            vec![Color::White], vec![CardType::Creature],
            vec![Keyword::Flying], vec!["Spirit".into()], registry,
        );
        state.log(crate::state::LogLevel::Event,
            "Moorland Haunt created a 1/1 white Spirit token with flying".into());
    }
}

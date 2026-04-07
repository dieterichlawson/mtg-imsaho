use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Shimmering Grotto — Land.
/// {T}: Add {C}.
/// {1}, {T}: Add one mana of any color.
///
/// Simplified: the {1},{T} ability adds {G} (arbitrary choice since the engine
/// doesn't have a "choose a color" mechanism for mana abilities; the AI player
/// will treat this as color-fixing regardless).
pub struct ShimmeringGrotto;

impl CardBehavior for ShimmeringGrotto {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Shimmering Grotto".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {C}.\n{1}, {T}: Add one mana of any color.".into(),
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
            vec![
                ManaAbilityDef {
                    ability_index: 0,
                    description: "Add {C}".into(),
                    produced: vec![(ManaType::Colorless, 1)],
                    requires_tap: true,
                    has_side_effects: false,
                },
            ]
        } else {
            vec![]
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        // The {1},{T} filter ability: pay 1 mana, tap, get any color.
        // We present 5 options (one per color) so the AI can pick.
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![
                ActivatedAbilityDef {
                    ability_index: 1,
                    description: "{1}, {T}: Add {W}".into(),
                    cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                    requires_tap: true,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                },
                ActivatedAbilityDef {
                    ability_index: 2,
                    description: "{1}, {T}: Add {U}".into(),
                    cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                    requires_tap: true,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                },
                ActivatedAbilityDef {
                    ability_index: 3,
                    description: "{1}, {T}: Add {B}".into(),
                    cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                    requires_tap: true,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                },
                ActivatedAbilityDef {
                    ability_index: 4,
                    description: "{1}, {T}: Add {R}".into(),
                    cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                    requires_tap: true,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                },
                ActivatedAbilityDef {
                    ability_index: 5,
                    description: "{1}, {T}: Add {G}".into(),
                    cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                    requires_tap: true,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                },
            ]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(_object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let (mana_type, symbol) = match ability_index {
            1 => (ManaType::White, "W"),
            2 => (ManaType::Blue, "U"),
            3 => (ManaType::Black, "B"),
            4 => (ManaType::Red, "R"),
            5 => (ManaType::Green, "G"),
            _ => return,
        };
        state.get_player_mut(controller).mana_pool.add(mana_type, 1);
        state.log(crate::state::LogLevel::Event,
            format!("Shimmering Grotto adds {{{}}}", symbol));
    }
}

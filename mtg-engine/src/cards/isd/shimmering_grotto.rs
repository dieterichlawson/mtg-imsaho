use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, ManaType, ManaCost, ManaSymbol};

/// Shimmering Grotto — Land.
/// {T}: Add {C}.
/// {1}, {T}: Add one mana of any color.
///
/// "One mana of any color" is one mana ability per color rather than a choice
/// prompt — five ManaAbilityDef entries, each costing {1}. That keeps it in
/// `mana_abilities`, where it belongs: a filter is a mana ability under
/// CR 605.1a, and only mana abilities are visible to the auto-tap planner.
/// Exposing it through `activated_abilities` instead meant the planner never
/// knew the Grotto could make colored mana, so a hand that needed the Grotto
/// for its one green source generated no CastSpell action at all.
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

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let mut abilities = vec![
            ManaAbilityDef {
                ability_index: 0,
                description: "Add {C}".into(),
                produced: vec![(ManaType::Colorless, 1)],
                requires_tap: true,
                cost: ManaCost::free(),
                has_side_effects: false,
            },
        ];
        // "{1}, {T}: Add one mana of any color" — one entry per color, indexed
        // 1..=5 so the indices match the descriptions a player sees.
        for (i, mana_type) in [ManaType::White, ManaType::Blue, ManaType::Black,
                               ManaType::Red, ManaType::Green].into_iter().enumerate() {
            abilities.push(ManaAbilityDef {
                ability_index: i + 1,
                description: format!("{{1}}, {{T}}: Add {{{}}}", match mana_type {
                    ManaType::White => "W", ManaType::Blue => "U", ManaType::Black => "B",
                    ManaType::Red => "R", _ => "G",
                }),
                produced: vec![(mana_type, 1)],
                requires_tap: true,
                cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                has_side_effects: false,
            });
        }
        abilities
    }
}

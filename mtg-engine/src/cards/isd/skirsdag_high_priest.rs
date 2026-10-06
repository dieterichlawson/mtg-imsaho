use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TapCreaturesCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, Keyword};

/// Skirsdag High Priest — {1}{B} 1/2 Human Cleric.
/// Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon
/// creature token with flying. Activate only if a creature died this turn.
pub struct SkirsdagHighPriest;

impl CardBehavior for SkirsdagHighPriest {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Skirsdag High Priest".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        // The {T} part of the cost — untapped, and past summoning sickness
        // unless hasty (CR 302.6) — is the engine's to check, and so is the
        // "tap two untapped creatures you control" part: which two is asked
        // as one set when the ability is activated (issue #670). It used to
        // be one ability per pair, encoded in the ability index — 55 rows at
        // eleven creatures, 190 at twenty, on every surface.
        // What's particular to this ability: morbid.
        if obj.zone != Zone::Battlefield {
            return vec![];
        }
        if !state.creature_died_this_turn {
            return vec![];
        }
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying".into(),
            cost: ManaCost::new(vec![]),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
            tap_cost: Some(TapCreaturesCost { count: 2 }),
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        // CR 602.2a: an activated ability's controller is the player who
        // activated it, which the engine records; CR 608.2g falls back to the
        // source's last known controller. Reading `o.controller` here gave the
        // *current* controller, so an opponent taking the permanent in
        // response to the ability collected the effect — and `None => return`
        // threw the whole effect away if the source had left, against
        // CR 113.7a.
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        state.create_token_with_subtypes(
            "",
            controller,
            5, 5,
            vec![Color::Black],
            vec![CardType::Creature],
            vec![Keyword::Flying],
            vec!["Demon".into()],
            registry,
        );

        state.log(crate::state::LogLevel::Event,
            "Skirsdag High Priest creates a 5/5 black Demon token with flying".to_string());
    }
}

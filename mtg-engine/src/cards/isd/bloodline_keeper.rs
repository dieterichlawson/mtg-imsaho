use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{Zone, ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectScope, CreatureFilter};

/// Bloodline Keeper {2}{B}{B} 3/3 Vampire // Lord of Lineage 5/5 Vampire.
/// {T}: Create a 2/2 black Vampire creature token with flying.
/// {B}: Transform Bloodline Keeper. Activate only if you control five or more Vampires.
/// Lord of Lineage: Other Vampire creatures you control get +2/+2.
/// {T}: Create a 2/2 black Vampire creature token with flying.
pub struct BloodlineKeeper;

impl BloodlineKeeper {
    fn count_vampires(state: &GameState, controller: crate::ids::PlayerId, registry: &CardRegistry) -> usize {
        state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| state.has_subtype(o.id, "Vampire", registry))
            .count()
    }
}

impl CardBehavior for BloodlineKeeper {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bloodline Keeper".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Flying\n{T}: Create a 2/2 black Vampire creature token with flying.\n{B}: Transform this creature. Activate only if you control five or more Vampires.".into(),
            keywords: vec![Keyword::Flying],
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Lord of Lineage".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Vampire".into()],
            power: Some(5),
            toughness: Some(5),
            // CR 204.2: the back face has no mana cost, so its color is the
            // indicator printed beside its type line — black.
            color_indicator: vec![Color::Black],
            oracle_text: "Flying\nOther Vampire creatures you control get +2/+2.\n{T}: Create a 2/2 black Vampire creature token with flying.".into(),
            keywords: vec![Keyword::Flying],
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 2,
                    toughness: 2,
                    scope: EffectScope::GlobalOther(CreatureFilter::And(vec![
                        CreatureFilter::ControlledByYou,
                        CreatureFilter::HasSubtype("Vampire".into()),
                    ])),
                },
            ],
            ..Default::default()
        })
    }


    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        let is_transformed = obj.is_transformed;
        let controller = obj.controller;
        let mut abilities = vec![];

        // Both faces: {T}: Create a 2/2 Vampire token with flying.
        abilities.push(ActivatedAbilityDef {
            ability_index: 0,
            description: "{T}: Create a 2/2 Vampire token with flying".into(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        });

        // Front face only: {B}: Transform (requires 5+ Vampires).
        //
        // CR 111.7: a token that is a copy of a double-faced card is not itself
        // double-faced — it has only the copied face — so it cannot transform.
        // Cackling Counterpart makes exactly such a token of this card, and it
        // used to be offered a transform ability that could not mean anything.
        if !is_transformed && !obj.is_token {
            let vampire_count = Self::count_vampires(state, controller, registry);
            if vampire_count >= 5 {
                abilities.push(ActivatedAbilityDef {
                    ability_index: 1,
                    description: "{B}: Transform Bloodline Keeper (5+ Vampires)".into(),
                    cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Black)]),
                    requires_tap: false,
                    sacrifice_cost: SacrificeCost::None,
                    target_requirement: None,
                    once_per_turn: false,
                    sorcery_speed_only: false,
                    counter_cost: None,
                });
            }
        }

        abilities
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        // CR 602.2a: an activated ability's controller is the player who
        // activated it, which the engine records; CR 608.2g falls back to the
        // source's last known controller. Reading `o.controller` here gave the
        // *current* controller, so an opponent taking the permanent in
        // response to the ability collected the effect — and `None => return`
        // threw the whole effect away if the source had left, against
        // CR 113.7a.
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        match ability_index {
            0 => {
                // Create a 2/2 Vampire token with flying.
                state.create_token_with_subtypes(
                    "",
                    controller,
                    2, 2,
                    vec![Color::Black],
                    vec![CardType::Creature],
                    vec![Keyword::Flying],
                    vec!["Vampire".into()],
                    registry,
                );
                let face_name = state.get_object(object_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("{face_name}: created a 2/2 Vampire token with flying"));
            }
            1 => {
                // Through the shared helper rather than setting the flag and
                // the name by hand: `apply_transform` is where "what
                // transforming means" lives, including the CR 111.7 refusal for
                // a token copy of a double-faced card. Writing the two fields
                // directly bypassed that.
                crate::cards::helpers::apply_transform(state, object_id, registry);
            }
            _ => {}
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

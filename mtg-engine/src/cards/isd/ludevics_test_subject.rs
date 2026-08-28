use crate::actions::Target;
use crate::cards::helpers;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, CounterType, Keyword, Zone};

/// Ludevic's Test Subject {1}{U} 0/3 Lizard Egg // Ludevic's Abomination 13/13 Lizard Horror
/// with Trample.
/// {1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters
/// on it, remove all of them and transform it.
///
/// Implementation: hatchling counters live in the engine's counter pipeline
/// (`CounterType::Hatchling`) so proliferate / counter-removal effects can
/// interact with them per CR 122.
pub struct LudevicsTestSubject;

impl CardBehavior for LudevicsTestSubject {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ludevic's Test Subject".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Lizard".into(), "Egg".into()],
            power: Some(0),
            toughness: Some(3),
            oracle_text: "Defender\n{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.".into(),
            keywords: vec![Keyword::Defender],
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Ludevic's Abomination".into(),
            // CR 204.2: the back face has no mana cost, so its colour comes
            // from the printed colour indicator. Without one it resolved to
            // colourless and dodged every "blue creature" effect in the set.
            color_indicator: vec![Color::Blue],
            card_types: vec![CardType::Creature],
            subtypes: vec!["Lizard".into(), "Horror".into()],
            power: Some(13),
            toughness: Some(13),
            oracle_text: "Trample".into(),
            keywords: vec![Keyword::Trample],
            ..Default::default()
        })
    }


    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // The ability is printed on the front face only, so it can only be
        // activated while that face is up (CR 712.8a).
        let front_face_up = state.get_object(object_id)
            .is_some_and(|o| o.zone == Zone::Battlefield && !o.is_transformed);
        if !front_face_up {
            return vec![];
        }
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}{U}: Put a hatchling counter. At 5, transform.".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    /// "Put a hatchling counter on this creature. Then if there are five or
    /// more hatchling counters on it, remove all of them and transform it."
    ///
    /// Every part of that is unconditional, and none of it asks which face is
    /// up. This used to open with `if is_transformed { return }`, on the
    /// reasoning that "the back face has no activated abilities" — but which
    /// abilities the back face has says nothing about what an ability already
    /// on the stack does. The ability exists on the stack independently of its
    /// source (CR 113.7a), transforming is not a zone change so the permanent
    /// is the same object throughout (CR 400.7, CR 712.8), and "this creature"
    /// is a self-reference to that object rather than to a permanent with a
    /// particular name. So five surplus activations held on the
    /// stack really do stack hatchling counters onto Ludevic's Abomination and
    /// flip it back.
    ///
    /// Pre-errata the line read "Put a hatchling counter on Ludevic's Test
    /// Subject", which is where the old contrary reading came from; the name
    /// is gone from the current Oracle text.
    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        state.add_counters(object_id, CounterType::Hatchling, 1);
        let new_count = state.get_counter_count(object_id, CounterType::Hatchling);

        if new_count >= 5 {
            // "remove all of them" — all, not five, so a permanent pushed past
            // five by proliferate loses the surplus too.
            state.remove_counters(object_id, CounterType::Hatchling, new_count);
            helpers::apply_transform(state, object_id, registry);
        } else {
            state.log(crate::state::LogLevel::Event,
                format!("Ludevic's Test Subject: hatchling counter added ({new_count}/5)"));
        }
    }
}

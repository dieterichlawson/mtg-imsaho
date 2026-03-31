use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Parallel Lives — {3}{G} Enchantment.
/// If an effect would create one or more tokens under your control,
/// it creates twice that many of those tokens instead.
///
/// The doubling is handled in GameState::create_token_with_subtypes.
pub struct ParallelLives;

impl CardBehavior for ParallelLives {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Parallel Lives".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

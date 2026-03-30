use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Abattoir Ghoul — {3}{B} 3/2 Zombie. First strike.
/// Whenever a creature dealt damage by Abattoir Ghoul this turn dies,
/// you gain life equal to that creature's toughness.
///
/// TODO: The life gain trigger is not yet implemented. It requires a "damaged_by" tracking
/// system to know which creatures Abattoir Ghoul dealt damage to this turn, and then watch
/// for those creatures dying. For now, only the body (3/2 Zombie with First Strike) is implemented.
pub struct AbattoirGhoul;

impl CardBehavior for AbattoirGhoul {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Abattoir Ghoul".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "First strike\nWhenever a creature dealt damage by Abattoir Ghoul this turn dies, you gain life equal to that creature's toughness.".into(),
            keywords: vec![Keyword::FirstStrike],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![],
            // TODO: Add triggered ability when damaged_by tracking is available.
        }
    }
}

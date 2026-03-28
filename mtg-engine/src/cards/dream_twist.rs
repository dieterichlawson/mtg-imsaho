use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Dream Twist — {U} instant. Target player mills three cards.
pub struct DreamTwist;

impl CardBehavior for DreamTwist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Dream Twist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target player mills three cards.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue)])),
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Player(player_id)) = targets.first() {
            crate::engine::mill_cards(state, *player_id, 3);
        }
        state.move_spell_after_resolve(object_id);
    }
}

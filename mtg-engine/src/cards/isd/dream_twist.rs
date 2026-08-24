use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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
            oracle_text: "Target player mills three cards.\nFlashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue)])),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Player(player_id)) = targets.first() {
            crate::engine::mill_cards(state, *player_id, 3, registry);
        }
    }
}

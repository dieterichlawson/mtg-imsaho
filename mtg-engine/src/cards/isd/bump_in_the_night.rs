use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Bump in the Night — {B} sorcery. Target opponent loses 3 life.
pub struct BumpInTheNight;

impl CardBehavior for BumpInTheNight {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bump in the Night".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Target opponent loses 3 life.\nFlashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(5), ManaSymbol::Colored(Color::Red)])),
            ..Default::default()
        }
    }

    /// "Target opponent", not "target player" (CR 102.1). This used to be
    /// `PlayerOnly` plus an `is_valid_target` of `*pid != caster`; the
    /// requirement now says it, so the engine offers only opponents and
    /// re-checks the same restriction on resolution (CR 608.2b).
    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::OpponentOnly
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Player(player_id)) = targets.first() {
            // "loses 3 life", not "deals 3 damage": life loss bypasses
            // prevention, protection and damage triggers.
            state.lose_life(*player_id, 3);
        }
    }
}

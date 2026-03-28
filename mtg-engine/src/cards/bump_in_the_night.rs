use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target opponent loses 3 life.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(5), ManaSymbol::Colored(Color::Red)])),
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Player(player_id)) = targets.first() {
            let old_life = state.get_player(*player_id).life;
            let new_life = old_life - 3;
            state.get_player_mut(*player_id).life = new_life;
            state.events.push(GameEvent::LifeChanged {
                player: *player_id,
                old: old_life,
                new_life,
            });
        }
        state.move_spell_after_resolve(object_id);
    }
}

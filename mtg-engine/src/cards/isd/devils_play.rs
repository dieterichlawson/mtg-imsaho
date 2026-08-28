use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Devil's Play — {X}{R} Sorcery.
/// Devil's Play deals X damage to any target.
/// Flashback {X}{R}{R}{R}.
///
/// X is the player's, announced as the spell is cast (CR 601.2b): the engine
/// puts up a `ChooseXFunding` prompt, taps what the player names for it and
/// records the result as `x_value` on the spell object. This card reads that
/// number and nothing else — which is also why the flashback cost needs no
/// special case here, its {R}{R}{R} being paid by the same machinery.
///
/// The comment that stood here described X as "computed as the total mana the
/// player had minus the colored requirement", under a heading of "Simplified:
/// since the engine doesn't yet support choosing X at cast time". It does; the
/// note outlived the limitation it described.
pub struct DevilsPlay;

impl CardBehavior for DevilsPlay {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Devil's Play".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::X,
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Devil's Play deals X damage to any target.\nFlashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::X,
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::AnyTarget
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // No guard on X being zero: CR 120.8 says a source that would deal 0
        // damage deals none, and `damage::deal_damage` is where that lives.
        let x = state.get_object(object_id)
            .and_then(|o| o.x_value)
            .unwrap_or(0);
        crate::cards::helpers::resolve_damage(state, object_id, targets, x, registry);
    }
}

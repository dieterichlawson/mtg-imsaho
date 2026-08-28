use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Smite the Monstrous — {3}{W} instant. Destroy target creature with power 4 or greater.
pub struct SmiteTheMonstrous;

impl CardBehavior for SmiteTheMonstrous {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Smite the Monstrous".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Destroy target creature with power 4 or greater.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::PowerAtLeast(4))
    }

    /// No `is_valid_target`: "a creature on the battlefield with power 4 or
    /// greater" is exactly `CreatureWithFilter(PowerAtLeast(4))`, whose filter
    /// arm reads `state.effective_power(..)` — the same call the card's copy
    /// made. `legal_actions` applies it when offering targets, and
    /// `stack::is_target_legal` re-applies the zone check, the creature check
    /// and the filter on the way down (CR 608.2b), which is what makes a
    /// creature shrunk below 4 in response stop being a legal target.
    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, targets, registry);
    }
}

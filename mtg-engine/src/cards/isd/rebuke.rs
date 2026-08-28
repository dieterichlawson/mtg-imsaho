use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Rebuke — {2}{W} instant. Destroy target attacking creature.
pub struct Rebuke;

impl CardBehavior for Rebuke {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rebuke".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Destroy target attacking creature.".into(),
            ..Default::default()
        }
    }

    /// "target attacking creature" is one filter. The engine offers targets by
    /// it (CR 601.2c) and re-checks it on resolution (CR 608.2b), including the
    /// creature-ness half. This card also carried an `is_valid_target` whose
    /// whole body was `combat.attackers.contains_key(id)` behind a
    /// battlefield-and-is-creature preamble — the same `TargetFilter::Attacking`
    /// arm `matches_target_filter` runs, behind guards both callers already make.
    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::Attacking)
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, targets, registry);
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Prey Upon — {G} sorcery. Target creature you control fights target creature you don't control.
pub struct PreyUpon;

impl CardBehavior for PreyUpon {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Prey Upon".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::TwoTargets(
            Box::new(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
            Box::new(TargetRequirement::CreatureWithFilter(TargetFilter::YouDontControl)),
        )
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // Ruling: "If either target is an illegal target as Prey Upon
        // resolves, no creature will deal or be dealt damage." Two targets, so
        // one of them going illegal does not counter the spell (CR 608.2b) —
        // it resolves and the fight does not happen.
        //
        // Both halves of that are `combat::fight`'s: an illegal target arrives
        // here as `Target::Illegal` and fails the pattern, and CR 701.12b
        // ("if one or both creatures ... are no longer on the battlefield or
        // are no longer creatures, neither of them fights") is checked there.
        // This card used to re-check the battlefield half itself, and to sort
        // the two creatures into "mine" and "theirs" before handing them over
        // — a fight is symmetric (CR 701.12a), so the sort decided nothing.
        if let [Target::Object(a), Target::Object(b)] = targets {
            crate::combat::fight(state, *a, *b, registry);
        }
    }
}

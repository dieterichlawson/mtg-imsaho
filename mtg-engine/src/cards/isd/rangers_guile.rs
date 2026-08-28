use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{GameState, TemporaryEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, Keyword};

/// Ranger's Guile — {G} instant. Target creature you control gets +1/+1 and gains hexproof until end of turn.
pub struct RangersGuile;

impl CardBehavior for RangersGuile {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ranger's Guile".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Target creature you control gets +1/+1 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)
    }

    /// No `is_valid_target`: "target creature you control on the battlefield"
    /// is exactly `CreatureWithFilter(YouControl)`, which `legal_actions`
    /// applies when offering targets and `stack::is_target_legal` re-applies
    /// on the way down — creature-ness included, since that re-check now makes
    /// it (CR 608.2b).
    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                state.until_end_of_turn.push(
                    crate::state::TemporaryEffect::ModifyPT {
                        target: *target_id,
                        power_mod: 1,
                        toughness_mod: 1,
                    }
                );
                state.until_end_of_turn.push(
                    TemporaryEffect::GrantKeyword {
                        target: *target_id,
                        keyword: Keyword::Hexproof,
                    }
                );
            }
        }
    }
}

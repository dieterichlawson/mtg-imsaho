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

    fn is_valid_target(&self, state: &GameState, caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .is_some_and(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == caster)
            }
            Target::Player(_) => false,
        }
    }

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

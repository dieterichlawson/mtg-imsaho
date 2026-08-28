use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{GameState, TemporaryEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, Keyword};

/// Spidery Grasp — {2}{G} instant. Untap target creature. It gets +2/+4 and gains reach until end of turn.
pub struct SpideryGrasp;

impl CardBehavior for SpideryGrasp {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Spidery Grasp".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Untap target creature. It gets +2/+4 and gains reach until end of turn. (It can block creatures with flying.)".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                // Untap the target creature.
                state.untap(*target_id);
                state.until_end_of_turn.push(
                    crate::state::TemporaryEffect::ModifyPT {
                        target: *target_id,
                        power_mod: 2,
                        toughness_mod: 4,
                    }
                );
                state.until_end_of_turn.push(
                    TemporaryEffect::GrantKeyword {
                        target: *target_id,
                        keyword: Keyword::Reach,
                    }
                );
            }
        }
    }
}

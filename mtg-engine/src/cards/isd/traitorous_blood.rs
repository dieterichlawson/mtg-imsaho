use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{GameState, TemporaryEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, Keyword};

/// Traitorous Blood — {1}{R}{R} Sorcery.
/// Gain control of target creature until end of turn. Untap it.
/// It gains trample and haste until end of turn.
pub struct TraitorousBlood;

impl CardBehavior for TraitorousBlood {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Traitorous Blood".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if state.get_object(*creature_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                let controller = crate::cards::helpers::controller_of(state, object_id);
                // CR 613.7a: one more layer-2 effect, with the latest
                // timestamp, rather than a note about who had the creature a
                // moment ago. Where it goes when this ends is derived from
                // whatever else is still in force at that point — which is
                // how a creature stolen back from an Olivia Voldaren whose
                // Olivia then dies comes home instead of staying with the
                // thief (issue #285).
                let timestamp = state.next_control_timestamp();
                state.until_end_of_turn.push(TemporaryEffect::ChangeControl {
                    target: *creature_id,
                    controller,
                    timestamp,
                });
                // Gain control (summoning-sick for the new controller) and untap.
                // The haste grant below lets it attack this turn anyway.
                state.change_control(*creature_id, controller);
                state.untap(*creature_id);
                // Grant haste and trample.
                state.until_end_of_turn.push(TemporaryEffect::GrantKeyword {
                    target: *creature_id,
                    keyword: Keyword::Haste,
                });
                state.until_end_of_turn.push(TemporaryEffect::GrantKeyword {
                    target: *creature_id,
                    keyword: Keyword::Trample,
                });
                let name = state.get_object(*creature_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("Traitorous Blood steals {name}, untaps it, grants haste and trample"));
            }
        }
    }
}

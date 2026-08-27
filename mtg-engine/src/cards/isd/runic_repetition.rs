use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Runic Repetition — {2}{U} Sorcery.
/// Return target exiled card with flashback you own to your hand.
pub struct RunicRepetition;

impl CardBehavior for RunicRepetition {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Runic Repetition".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Return target exiled card with flashback you own to your hand.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::ExileCard
    }

    fn is_valid_target(&self, state: &GameState, caster: crate::ids::PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .is_some_and(|o| {
                        o.zone == Zone::Exile && o.owner == caster
                            && state.face_data(o.id, registry)
                                .is_some_and(|d| d.flashback_cost.is_some())
                    })
            }
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            let name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*target_id, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Runic Repetition returned {name} from exile to hand"));
        }
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Frightful Delusion — {2}{U} instant. Counter target spell unless its controller pays {1}.
/// That player discards a card.
pub struct FrightfulDelusion;

impl CardBehavior for FrightfulDelusion {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Frightful Delusion".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Counter target spell unless its controller pays {1}. That player discards a card.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Spell
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .is_some_and(|o| o.zone == Zone::Stack)
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Stack {
                    let controller = obj.controller;

                    // Always ask. Whether the {1} is payable — floating or by
                    // tapping — is the engine's call (CR 608.2g); this used to
                    // check only for mana already in the pool and silently
                    // counter the spell of anyone who had not pre-floated it.
                    let spell_name = state.obj_name(*target_id);
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: controller,
                        source: object_id,
                        choice: ResolutionChoiceKind::PayOrNot {
                            description: format!("Pay {{1}} to prevent {spell_name} from being countered?"),
                            spell_id: *target_id,
                            source_spell_id: object_id,
                            cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                        },
                    });
                    return; // Don't clean up yet
                }
            }
        }
    }
}

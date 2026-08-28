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
            oracle_text: "Counter target spell unless its controller pays {1}. That player discards a card.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Spell
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        crate::cards::helpers::spell_target_is_legal(state, target)
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

    /// "That player discards a card."
    ///
    /// A separate sentence from "counter target spell unless its controller
    /// pays {1}", and Scryfall's ruling of 2011-09-22 spells out that they are
    /// independent: "The player discards a card even if they pay {1}." So this
    /// ignores `paid`.
    ///
    /// It used to live in the engine's `PayOrNot` handler, which ran it for
    /// whatever card raised the choice. Nothing else in the set raises one, so
    /// nothing was wrong — but "counter unless they pay" is a template and the
    /// next card to use it would have inherited this discard on top of its own
    /// rider.
    fn on_pay_decision(&self, state: &mut GameState, self_id: ObjectId, payer: PlayerId, _paid: bool, registry: &CardRegistry) {
        let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, payer)
            .iter().map(|o| o.id).collect();
        if hand.len() == 1 {
            // No choice to make with one card, so do not ask for one.
            state.discard_card(hand[0], registry);
            state.log(crate::state::LogLevel::Event,
                format!("p{} discarded a card", payer.0));
        } else if !hand.is_empty() {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: payer,
                source: self_id,
                choice: ResolutionChoiceKind::ChooseCardFromHand {
                    description: format!("{}: choose a card to discard", state.obj_name(self_id)),
                    player: payer,
                    cards: hand,
                    discard_immediately: true,
                    remaining: 1,
                },
            });
        }
    }
}

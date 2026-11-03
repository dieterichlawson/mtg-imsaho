//! Answering a "choose a target" question asked while something resolves
//! (`actions::choices::resolve_choice`, CR 601.2c, 608.2): the answer is
//! one of the targets offered, or, where the question allows it, none;
//! anything else is no answer, and the question stands. What applying the
//! effect to the target chosen does is `engine::apply_pending_effect`'s,
//! recorded here (`pending_effects` proves it), and what declining does is
//! the card's (`on_declined_choice`, each card's proof).

use crate::actions::{ResolvedChoice, Target};
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_for, card_proof, Act};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::Zone;

/// The object whose card was told its question was declined.
static mut DECLINED: Option<ObjectId> = None;

/// A card that asks a question and is told when it is declined.
struct Asking;

impl CardBehavior for Asking {
    fn card_data(&self) -> CardData {
        CardData { name: "Asking".into(), ..CardData::default() }
    }
    fn on_declined_choice(&self, _: &mut GameState, id: ObjectId, _: &CardRegistry) {
        // SAFETY: a proof runs on one thread, and only this function writes DECLINED.
        unsafe { *std::ptr::addr_of_mut!(DECLINED) = Some(id) };
    }
}

/// A "choose a target" question answered (CR 601.2c, 608.2): with one of
/// the targets offered, the effect is applied to it, once; with none,
/// where the question is optional, the card is told it was declined;
/// either way the question is answered, the passes in a row start again,
/// and the ability that asked has finished resolving. With a target not
/// offered, or none where the question is not optional, the answer is
/// refused: the question stands exactly as it was asked, and nothing is
/// done. For the second player asked, and the reply (none, either target
/// offered, or another) and whether the question is optional constants of
/// each harness: with the answer any of the four, or the player or
/// whether it is optional the solver's, the answer was every path through
/// `resolve_choice` at once (as the card proofs of `choices` found), and
/// symbolic execution ran out of memory.
fn answered(reply: u8, optional: bool) {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Asking));
    let mut state = crate::proofs::cards::new_game();
    let chooser = PlayerId(1);
    let source = state.create_object(card, chooser, Zone::Battlefield, None, None);
    let offered = [Target::Object(ObjectId(7)), Target::Player(PlayerId(1))];
    let effect = PendingEffect::Destroy { source_name: String::new() };
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: chooser,
        source,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: String::new(),
            options: offered.to_vec(),
            optional,
            effect: effect.clone(),
        },
    });
    state.consecutive_passes = 2;
    state.resolving_ability_activator = Some(chooser);
    let answer = match reply {
        0 => None,
        1 => Some(offered[0].clone()),
        2 => Some(offered[1].clone()),
        _ => Some(Target::Object(ObjectId(8))),
    };

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenTarget(answer.clone()), &registry);

    // SAFETY: as in `Asking::on_declined_choice`.
    let declined = unsafe { *std::ptr::addr_of!(DECLINED) };
    let refused = answer == Some(Target::Object(ObjectId(8))) || (answer.is_none() && !optional);
    if refused {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source: s,
            choice: ResolutionChoiceKind::ChooseTarget { options, optional: o, effect: e, .. },
        }) if *player == chooser && *s == source && options.as_slice() == offered && *o == optional && *e == effect));
        assert!(acted() == 0 && declined.is_none() && state.consecutive_passes == 2);
    } else {
        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(state.awaiting_action.is_none() && state.consecutive_passes == 0);
        assert!(state.resolving_ability_activator.is_none() && state.resolving_ability.is_none());
        match answer {
            Some(t) => {
                assert!(asked_for(&[Act::Effect(t, effect.clone())]) && declined.is_none());
            }
            None => {
                assert!(acted() == 0 && declined == Some(source));
            }
        }
    }
    std::mem::forget((registry, state, effect));
}

/// The proofs of a target answered, one per reply.
macro_rules! each_reply {
    ($($name:ident: $reply:expr, $optional:expr;)*) => {$(
        card_proof! {
            /// [`answered`] for the reply of its name.
            fn $name() {
                answered($reply, $optional);
            }
        }
    )*};
}

each_reply! {
    declining_an_optional_target_is_taken: 0, true;
    declining_a_target_that_is_not_optional_is_refused: 0, false;
    the_first_target_offered_is_taken: 1, false;
    the_second_target_offered_is_taken: 2, true;
    a_target_not_offered_is_refused_and_the_question_stands: 3, true;
}

/// Stands in for `helpers::token_attack_options`: the defending player.
#[allow(dead_code)] // named only by `kani::stub`
fn attack_options(_: &GameState, _: PlayerId, _: &CardRegistry) -> Vec<Target> {
    vec![Target::Player(PlayerId(1))]
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::cards::helpers::token_attack_options, attack_options)]
#[kani::stub(crate::cards::helpers::present_target_choice, crate::proofs::cards::record_choose)]
/// What a token put onto the battlefield attacking attacks, answered
/// (`engine::apply_pending_effect` of `TokenAttacks`, CR 508.4, 506.2):
/// with a player, it is attacking that player; with a planeswalker, it is
/// attacking that planeswalker, the walker's controller its defending
/// player; either way it is an attacker with no blockers yet, and the
/// next token, if one is left, has its controller asked the same, of the
/// options as they are now, not optionally. For a token at either player
/// or at a planeswalker of either, and one token after it or none.
fn a_token_entering_attacking_attacks_what_its_controller_chose() {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Asking));
    let mut state = crate::proofs::cards::new_game();
    let source = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
    let token = state.create_object(card, PlayerId(0), Zone::Battlefield, Some(1), Some(1));
    let next = state.create_object(card, PlayerId(0), Zone::Battlefield, Some(1), Some(1));
    let walker_controller = PlayerId(u8::from(kani::any::<bool>()));
    let walker = state.create_object(card, walker_controller, Zone::Battlefield, None, None);
    state.combat = Some(crate::state::CombatState::default());
    let more: bool = kani::any();
    let remaining = if more { vec![next] } else { Vec::new() };
    let target = if kani::any() { Target::Object(walker) } else { Target::Player(PlayerId(u8::from(kani::any::<bool>()))) };

    crate::engine::apply_pending_effect(&mut state, &target,
        &PendingEffect::TokenAttacks { token_id: token, remaining, source_id: source }, &registry);

    let combat = state.combat.as_ref().expect("a combat");
    let defending = match target {
        Target::Player(p) => p,
        _ => walker_controller,
    };
    assert!(combat.attackers.get(&token) == Some(&defending));
    assert!(combat.blocker_assignments.get(&token).is_some_and(Vec::is_empty));
    assert!(combat.planeswalker_defenders.get(&token) == if target == Target::Object(walker) { Some(&walker) } else { None });
    if more {
        assert!(asked_for(&[Act::Choose {
            source,
            player: PlayerId(0),
            options: vec![Target::Player(PlayerId(1))],
            effect: PendingEffect::TokenAttacks { token_id: next, remaining: Vec::new(), source_id: source },
            optional: false,
        }]));
    } else {
        assert!(acted() == 0);
    }
    std::mem::forget((registry, state, target));
}

/// The answer recorded for the permanent entering.
static mut ENTRY: Option<(ObjectId, crate::state::EnterAsCopyChoice)> = None;

/// Stands in for `replacement::record_entry_choice` (what entering does
/// with the answer is `replacements`' proof): recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn entry_recorded(_: &mut GameState, object: ObjectId, choice: crate::state::EnterAsCopyChoice, _: &CardRegistry) {
    // SAFETY: as in `Asking::on_declined_choice`, for ENTRY.
    unsafe { *std::ptr::addr_of_mut!(ENTRY) = Some((object, choice)) };
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::replacement::record_entry_choice, entry_recorded)]
/// What a permanent entering as a copy copies, answered (`engine::
/// apply_pending_effect` of `EnterAsCopy`, CR 614.12b, 707.9): the creature
/// chosen, if it is still on the battlefield; otherwise, a choice no longer
/// legal, the permanent enters as itself, as if declined. For the creature
/// chosen on the battlefield or gone to a graveyard.
fn a_permanent_entering_as_a_copy_copies_the_creature_chosen_if_it_is_still_there() {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Asking));
    let mut state = crate::proofs::cards::new_game();
    let entering = state.create_object(card, PlayerId(0), Zone::Hand, None, None);
    let there: bool = kani::any();
    let chosen = state.create_object(card, PlayerId(1), if there { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));

    crate::engine::apply_pending_effect(&mut state, &Target::Object(chosen), &PendingEffect::EnterAsCopy { object: entering }, &registry);

    // SAFETY: as in `entry_recorded`.
    let recorded = unsafe { *std::ptr::addr_of!(ENTRY) };
    let expected = if there { crate::state::EnterAsCopyChoice::Copy(chosen) } else { crate::state::EnterAsCopyChoice::Declined };
    assert!(recorded == Some((entering, expected)));
    std::mem::forget((registry, state));
}

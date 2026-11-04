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
    // An effect of no fields: the handler only hands it on, and one holding
    // a string, read back off the restored question and compared, ran the
    // solver out of memory.
    let effect = PendingEffect::AttachTargetToPendingTrigger;
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
/// with the answer is `entering_as_copy`'s proof): recorded.
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

/// The activation `finish_activation` was handed: by whom, of what, and
/// the creatures its cost taps (up to two), once.
static mut FINISHED: Option<(PlayerId, ObjectId, [Option<ObjectId>; 2], usize)> = None;

/// Stands in for `actions::abilities::finish_activation`, which goes on as
/// an activation with nothing left to choose does (`activating`): recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn finish_activation(
    _: &mut GameState,
    a: &crate::engine::actions::abilities::Activation,
    cost: crate::state::DeferredActivationCost,
    _: &CardRegistry,
) -> crate::engine::Applied {
    let tapped = &cost.tapped_creatures;
    assert!(tapped.len() <= 2, "no more creatures than this proof's cost taps");
    // SAFETY: as in `Asking::on_declined_choice`, for FINISHED.
    let f = unsafe { &mut *std::ptr::addr_of_mut!(FINISHED) };
    assert!(f.is_none(), "finished once");
    *f = Some((a.player, a.object_id, [tapped.first().copied(), tapped.get(1).copied()], tapped.len()));
    std::mem::forget(cost);
    crate::engine::Applied::Continue
}

/// The creatures a "tap two untapped creatures you control" cost taps,
/// chosen (`resolve_choice` of `PayActivationTaps`, CR 602.2b via 601.2h,
/// 118.3): exactly two of the creatures offered, each once, each still an
/// untapped creature its chooser controls on the battlefield, and the
/// activation goes on with them as the creatures its cost taps, in the
/// order chosen, the passes in a row starting again; anything else (too
/// few, one twice, one not offered, one tapped since it was offered) is
/// refused: the question stands exactly as it was asked, the activation
/// still waits on it, and nothing is done. For the second player
/// activating, with three of their creatures offered, and each reply a
/// harness's: 0 and 1 the first two either way round, 2 only one, 3 one
/// twice, 4 one of the other player's, 5 the first two with the second
/// tapped since.
fn taps_chosen(reply: u8) {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Asking));
    let blank = registry.register(Box::new(crate::proofs::cards::Blank));
    let mut state = crate::proofs::cards::new_game();
    let chooser = PlayerId(1);
    let source = state.create_object(card, chooser, Zone::Battlefield, None, None);
    let mine = [
        state.create_object(blank, chooser, Zone::Battlefield, Some(1), Some(1)),
        state.create_object(blank, chooser, Zone::Battlefield, Some(1), Some(1)),
        state.create_object(blank, chooser, Zone::Battlefield, Some(1), Some(1)),
    ];
    let theirs = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(1), Some(1));
    let effect = PendingEffect::PayActivationTaps { source_id: source };
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: chooser,
        source,
        choice: ResolutionChoiceKind::ChooseObjectSet {
            description: String::new(),
            options: mine.to_vec(),
            min: 2,
            max: 2,
            effect: effect.clone(),
        },
    });
    state.pending_ability_effect = Some(crate::state::PendingAbilityEffect {
        source_id: source,
        ability_index: 0,
        behavior_card_id: card,
        targets: Vec::new(),
        description: String::new(),
        activator: chooser,
        target_requirement: None,
        unpaid: None,
        has_x: false,
    });
    state.consecutive_passes = 2;
    let chosen: [ObjectId; 2] = match reply {
        0 | 5 => [mine[0], mine[1]],
        1 => [mine[1], mine[0]],
        3 => [mine[0], mine[0]],
        _ => [mine[0], theirs],
    };
    let chosen = if reply == 2 { vec![mine[0]] } else { chosen.to_vec() };
    if reply == 5 {
        state.get_object_mut(mine[1]).expect("on the board").tapped = true;
    }

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenObjectSet(chosen), &registry);

    // SAFETY: as in `Asking::on_declined_choice`.
    let finished = unsafe { *std::ptr::addr_of!(FINISHED) };
    if reply <= 1 {
        let (first, second) = if reply == 0 { (mine[0], mine[1]) } else { (mine[1], mine[0]) };
        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(finished == Some((chooser, source, [Some(first), Some(second)], 2)));
        assert!(state.pending_ability_effect.is_none() && state.consecutive_passes == 0);
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source: s,
            choice: ResolutionChoiceKind::ChooseObjectSet { options, min: 2, max: 2, effect: e, .. },
        }) if *player == chooser && *s == source && options.as_slice() == mine && *e == effect));
        assert!(finished.is_none() && state.pending_ability_effect.is_some() && state.consecutive_passes == 2);
    }
    std::mem::forget((registry, state, effect));
}

/// Stands in for `GameState::is_creature`, proved in `predicates`: every
/// object of [`taps_chosen`]'s board is a creature (the layers that answer
/// it otherwise were a million steps of symbolic execution, out of memory).
#[allow(dead_code)] // named only by `kani::stub`
fn all_creatures(_: &GameState, _: ObjectId, _: &CardRegistry) -> bool {
    true
}

/// The proofs of a cost's creatures chosen, one per reply.
macro_rules! each_tap_reply {
    ($($name:ident: $reply:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::state::GameState::is_creature, all_creatures)]
            #[kani::stub(crate::engine::actions::abilities::finish_activation, finish_activation)]
            /// [`taps_chosen`] for the reply of its name.
            fn $name() {
                taps_chosen($reply);
            }
        }
    )*};
}

each_tap_reply! {
    two_creatures_offered_are_tapped_for_the_cost: 0;
    two_creatures_offered_are_tapped_in_the_order_chosen: 1;
    one_creature_for_a_cost_of_two_is_refused: 2;
    one_creature_chosen_twice_is_refused: 3;
    a_creature_not_offered_is_refused: 4;
    a_creature_tapped_since_it_was_offered_is_refused: 5;
}

/// What X-funding's answer was handed and did, in order: the mana the
/// funding paid (any), the costs paid (how many times, and whether the
/// cost paid was the one stashed), and the ability put on the stack (with
/// the X the game held then and how many times the costs had been paid).
static mut FUNDED: Option<u32> = None;
static mut COSTS_PAID: usize = 0;
static mut STASHED_COST: bool = false;
static mut PUSHED: Option<(ObjectId, usize, crate::ids::CardId, PlayerId, Option<u32>, usize)> = None;

/// Stands in for `funding::validate`: every response valid, as a seat
/// that answers from the prompt's own options gives.
#[allow(dead_code)] // each named only by `kani::stub`
fn validated(_: &crate::funding::FundingResponse, _: &crate::funding::FundingOptions) -> Result<(), crate::funding::FundingError> {
    Ok(())
}

/// Stands in for `funding::apply`, proved in the kernel (`mtg_kernel::
/// funding`, which taps and drains what it was told): any mana paid, up
/// to the prompt's most.
#[allow(dead_code)]
fn funded(_: &mut GameState, _: PlayerId, options: &crate::funding::FundingOptions, _: &crate::funding::FundingResponse, _: &CardRegistry) -> u32 {
    let paid: u32 = kani::any();
    kani::assume(paid <= options.max_x);
    // SAFETY: as in `Asking::on_declined_choice`, for FUNDED.
    let f = unsafe { &mut *std::ptr::addr_of_mut!(FUNDED) };
    assert!(f.is_none(), "funded once");
    *f = Some(paid);
    paid
}

/// Stands in for `actions::abilities::pay_activation_costs`, proved in
/// `activating`.
#[allow(dead_code)]
fn costs_paid(_: &mut GameState, _: PlayerId, _: ObjectId, _: usize, cost: &crate::state::DeferredActivationCost, _: &CardRegistry) {
    // SAFETY: as in `funded`, for COSTS_PAID and STASHED_COST.
    unsafe {
        *std::ptr::addr_of_mut!(COSTS_PAID) += 1;
        *std::ptr::addr_of_mut!(STASHED_COST) = cost.requires_tap && cost.once_per_turn;
    }
}

/// Stands in for `actions::abilities::put_ability_on_stack`, proved in
/// `activating`.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn pushed(
    state: &mut GameState, source: ObjectId, index: usize, card: crate::ids::CardId, targets: &[Target],
    activator: PlayerId, _: Option<crate::cards::TargetRequirement>, _: &CardRegistry,
) {
    assert!(targets.is_empty(), "the ability of this proof targets nothing");
    // SAFETY: as in `funded`, for PUSHED.
    let (p, paid) = unsafe { (&mut *std::ptr::addr_of_mut!(PUSHED), *std::ptr::addr_of!(COSTS_PAID)) };
    assert!(p.is_none(), "pushed once");
    *p = Some((source, index, card, activator, state.last_activated_x_value, paid));
}

card_proof! {
    #[kani::stub(crate::funding::validate, validated)]
    #[kani::stub(crate::funding::apply, funded)]
    #[kani::stub(crate::engine::actions::abilities::pay_activation_costs, costs_paid)]
    #[kani::stub(crate::engine::actions::abilities::put_ability_on_stack, pushed)]
    /// An ability's X funded (`resolve_choice` of `ChooseXFunding`, CR
    /// 601.2b, 601.2f-h via 602.2b, 107.3): X is the mana the funding paid
    /// plus the part of X a cost reduction pays for, and is what the
    /// ability resolves with (Kessig Wolf Run's "+X/+0"); only then are
    /// the ability's other costs paid, the ones stashed while X was asked,
    /// once, and then the ability goes on the stack, from its source, as
    /// that ability of the card granting it, under its activator; the
    /// question is answered, the activation no longer waits on anything,
    /// and the passes in a row start again. For the second player
    /// activating, a cost reduction of none to three, and any mana paid up
    /// to the most the prompt allows.
    fn an_abilitys_x_is_the_mana_funded_and_its_costs_are_paid_after() {
        let mut registry = CardRegistry::new();
        let card = registry.register(Box::new(Asking));
        let mut state = crate::proofs::cards::new_game();
        let activator = PlayerId(1);
        let source = state.create_object(card, activator, Zone::Battlefield, None, None);
        let discount = u32::from(kani::any::<u8>() % 4);
        let options = crate::funding::FundingOptions {
            pool: std::collections::BTreeMap::new(),
            groups: Vec::new(),
            max_x: 10,
            x_discount: discount,
            pool_order: Vec::new(),
            pool_keep: std::collections::BTreeMap::new(),
        };
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: activator,
            source,
            choice: ResolutionChoiceKind::ChooseXFunding { description: String::new(), options, source_id: source, is_ability: true },
        });
        state.pending_ability_effect = Some(crate::state::PendingAbilityEffect {
            source_id: source,
            ability_index: 1,
            behavior_card_id: card,
            targets: Vec::new(),
            description: String::new(),
            activator,
            target_requirement: None,
            unpaid: Some(crate::state::DeferredActivationCost {
                tap_plan: Vec::new(),
                non_x_mana_cost: crate::types::ManaCost::free(),
                requires_tap: true,
                counter_cost: None,
                sacrifice: None,
                sacrifice_cost: crate::cards::SacrificeCost::None,
                once_per_turn: true,
                tapped_creatures: Vec::new(),
            }),
            has_x: false,
        });
        state.priority_player = Some(activator);
        state.consecutive_passes = 2;

        let applied = crate::engine::actions::choices::resolve_choice(
            &mut state, &ResolvedChoice::XFunding(crate::funding::FundingResponse::default()), &registry);

        // SAFETY: as in `funded`.
        let (paid, costs, stashed, pushed) = unsafe { (
            *std::ptr::addr_of!(FUNDED), *std::ptr::addr_of!(COSTS_PAID),
            *std::ptr::addr_of!(STASHED_COST), *std::ptr::addr_of!(PUSHED),
        ) };
        let paid = paid.expect("funded");
        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(costs == 1 && stashed);
        assert!(pushed == Some((source, 1, card, activator, Some(paid + discount), 1)));
        assert!(state.pending_ability_effect.is_none() && state.awaiting_action.is_none() && state.consecutive_passes == 0);
        std::mem::forget((registry, state));
    }
}

/// How many times a spell's additional costs were paid (any it has).
static mut ADDITIONAL_PAID: usize = 0;

/// Stands in for `costs::pay_additional_cost`, proved in
/// `additional_costs`: counted.
#[allow(dead_code)] // each named only by `kani::stub`
#[allow(clippy::too_many_arguments)]
fn additional_paid(_: &mut GameState, _: &CardRegistry, _: crate::ids::CardId, _: ObjectId, _: PlayerId, _: Option<ObjectId>, _: Option<u32>, _: &[ObjectId]) {
    // SAFETY: as in `funded`, for ADDITIONAL_PAID.
    unsafe { *std::ptr::addr_of_mut!(ADDITIONAL_PAID) += 1 };
}

/// Stands in for `engine::demand_reserve`: none, as in `casting`; the
/// spell's cost but its X is none.
#[allow(dead_code)]
fn no_reserve(_: &GameState, _: &CardRegistry, _: PlayerId, _: Option<ObjectId>) -> crate::types::ManaCost {
    crate::types::ManaCost::free()
}

/// A spell's X funded (`resolve_choice` of `ChooseXFunding`, CR 601.2b,
/// 601.2f-i, 107.3): the spell, stashed in the zone it is cast from
/// while X was asked, has its additional costs paid once, moves to the
/// stack holding the targets it was cast with, the zone it was cast
/// from and whether it was cast with flashback, and its X is the mana
/// the funding paid plus the part of X a cost reduction pays for (what
/// Devil's Play deals); it goes on top of the stack, its casting is the
/// one event, it is counted to its caster, and the cast no longer waits
/// on anything. For the second player casting from their hand or, with
/// flashback, their graveyard (`flashback`, a constant of each harness:
/// left to the solver, the cast's log line built a list allocated on one
/// path and not the other, which Kani cannot follow), a cost reduction of
/// none to three, and any mana paid up to the most the prompt allows.
fn spell_x_funded(flashback: bool) {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Asking));
    let mut state = crate::proofs::cards::new_game();
    let caster = PlayerId(1);
    let from = if flashback { Zone::Graveyard } else { Zone::Hand };
    let spell = state.create_object(card, caster, from, None, None);
    let discount = u32::from(kani::any::<u8>() % 4);
    let options = crate::funding::FundingOptions {
        pool: std::collections::BTreeMap::new(),
        groups: Vec::new(),
        max_x: 10,
        x_discount: discount,
        pool_order: Vec::new(),
        pool_keep: std::collections::BTreeMap::new(),
    };
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: caster,
        source: spell,
        choice: ResolutionChoiceKind::ChooseXFunding { description: String::new(), options, source_id: spell, is_ability: false },
    });
    state.pending_spell_cast = Some(crate::state::PendingSpellCast {
        object_id: spell,
        player: caster,
        card_id: card,
        targets: vec![Target::Player(PlayerId(0))],
        sacrifice: None,
        exile_ids: Vec::new(),
        exile_count: None,
        tap_plan: Vec::new(),
        alternative_cost: None,
        non_x_mana_cost: crate::types::ManaCost::free(),
        is_flashback: flashback,
        cast_from_graveyard: flashback,
    });
    state.priority_player = Some(caster);
    let events = state.events.len();

    let applied = crate::engine::actions::choices::resolve_choice(
        &mut state, &ResolvedChoice::XFunding(crate::funding::FundingResponse::default()), &registry);

    // SAFETY: as in `funded`.
    let (paid, additional) = unsafe { (*std::ptr::addr_of!(FUNDED), *std::ptr::addr_of!(ADDITIONAL_PAID)) };
    let paid = paid.expect("funded");
    assert!(matches!(applied, crate::engine::Applied::Continue));
    assert!(additional == 1 && asked_for(&[Act::Move(spell, Zone::Stack)]));
    let obj = state.get_object(spell).expect("on the board");
    assert!(obj.x_value == Some(paid + discount) && obj.cast_from_zone == Some(from) && obj.cast_with_flashback == flashback);
    assert!(matches!(obj.targets.as_slice(), [Target::Player(PlayerId(0))]));
    assert!(matches!(state.stack.as_slice(), [crate::state::StackEntry::Spell(s)] if *s == spell));
    assert!(matches!(&state.events[events..], [crate::events::GameEvent::SpellCast { player, object }] if *player == caster && *object == spell));
    assert!(state.num_spells_cast_this_turn.get(&caster) == Some(&1));
    assert!(state.pending_spell_cast.is_none() && state.awaiting_action.is_none());
    std::mem::forget((registry, state));
}

/// Stands in for `engine::effects::cast_log_line`, the cast's line in the
/// log, which is for people: none. Built here, it is built for every count
/// of targets the proof cannot rule out (the cast's targets are read back
/// from the stash), and the words of names stood in for with nothing, joined
/// or pushed, failed the proof on a list of no room or one that grew.
#[allow(dead_code)] // named only by `kani::stub`
fn no_cast_line(
    _: &GameState,
    _: PlayerId,
    _: ObjectId,
    _: &crate::engine::effects::CastPayment,
    _: &[Target],
    _: &CardRegistry,
) -> String {
    String::new()
}

/// The proofs of a spell's X funded, one per zone it is cast from.
macro_rules! each_x_cast {
    ($($name:ident: $flashback:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::engine::effects::cast_log_line, no_cast_line)]
            #[kani::stub(crate::funding::validate, validated)]
            #[kani::stub(crate::funding::apply, funded)]
            #[kani::stub(crate::engine::costs::pay_additional_cost, additional_paid)]
            #[kani::stub(crate::engine::demand_reserve, no_reserve)]
            /// [`spell_x_funded`] for the zone of its name.
            fn $name() {
                spell_x_funded($flashback);
            }
        }
    )*};
}

each_x_cast! {
    a_spells_x_is_the_mana_funded_and_it_goes_on_the_stack_with_it: false;
    a_spells_x_cast_with_flashback_is_the_mana_funded_and_it_goes_on_the_stack_with_it: true;
}

/// A set of objects chosen (`resolve_choice` of `ChooseObjectSet`, CR
/// 601.2c's set read for a resolution's choice, as Curse of Oblivion asks
/// for two cards of a graveyard): as many as the question asks, each one
/// offered, each once, and the question's effect is applied to each, once,
/// in the order chosen (`apply_pending_effect`, recorded; what it does is
/// `pending_effects`'), the question answered and the passes in a row
/// starting again; anything else (too few, one twice, one not offered) is
/// refused, the question standing as it was asked and nothing done. For
/// the second player asked for two of three cards, and each reply a
/// harness's: 0 and 1 two offered, either order, 2 only one, 3 one twice,
/// 4 one not offered.
fn objects_chosen(reply: u8) {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Asking));
    let blank = registry.register(Box::new(crate::proofs::cards::Blank));
    let mut state = crate::proofs::cards::new_game();
    let chooser = PlayerId(1);
    let source = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
    let offered = [
        state.create_object(blank, chooser, Zone::Graveyard, None, None),
        state.create_object(blank, chooser, Zone::Graveyard, None, None),
        state.create_object(blank, chooser, Zone::Graveyard, None, None),
    ];
    let effect = PendingEffect::AttachTargetToPendingTrigger;
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: chooser,
        source,
        choice: ResolutionChoiceKind::ChooseObjectSet {
            description: String::new(),
            options: offered.to_vec(),
            min: 2,
            max: 2,
            effect: effect.clone(),
        },
    });
    state.consecutive_passes = 2;
    let chosen = match reply {
        0 => vec![offered[0], offered[1]],
        1 => vec![offered[2], offered[0]],
        2 => vec![offered[0]],
        3 => vec![offered[0], offered[0]],
        _ => vec![offered[0], source],
    };
    let expected = [chosen[0], *chosen.last().expect("one at least")];

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenObjectSet(chosen), &registry);

    if reply <= 1 {
        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(asked_for(&[
            Act::Effect(Target::Object(expected[0]), effect.clone()),
            Act::Effect(Target::Object(expected[1]), effect.clone()),
        ]));
        assert!(state.awaiting_action.is_none() && state.consecutive_passes == 0);
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source: s,
            choice: ResolutionChoiceKind::ChooseObjectSet { options, min: 2, max: 2, effect: e, .. },
        }) if *player == chooser && *s == source && options.as_slice() == offered && *e == effect));
        assert!(acted() == 0 && state.consecutive_passes == 2);
    }
    std::mem::forget((registry, state, effect));
}

/// The proofs of a set chosen, one per reply.
macro_rules! each_set_reply {
    ($($name:ident: $reply:expr;)*) => {$(
        card_proof! {
            /// [`objects_chosen`] for the reply of its name.
            fn $name() {
                objects_chosen($reply);
            }
        }
    )*};
}

each_set_reply! {
    two_objects_offered_have_the_effect_applied: 0;
    two_objects_offered_have_it_applied_in_the_order_chosen: 1;
    one_object_for_a_set_of_two_is_refused: 2;
    one_object_chosen_twice_is_refused: 3;
    an_object_not_offered_is_refused: 4;
}

/// A library search answered (`resolve_choice` of `ChooseFromLibrary`, CR
/// 701.19a-b, 701.20a): with one of the cards it offered, that card is
/// found, put where the search puts it, and the library shuffled
/// (`finish_library_search`, proved in `searching`, its moves recorded
/// here); with none, nothing is found and the library is shuffled all the
/// same; with a card it did not offer (one in the library without the
/// quality searched for), the answer is refused: the question stands as it
/// was asked and nothing is moved or shuffled (defect 21 below: the card
/// named was found). For the second player searching for one of two
/// cards offered, to their hand, and each reply a harness's: 0 and 1 a
/// card offered, 2 none, 3 a card not offered.
fn searched(reply: u8) {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Asking));
    let blank = registry.register(Box::new(crate::proofs::cards::Blank));
    let mut state = crate::proofs::cards::new_game();
    let searcher = PlayerId(1);
    let source = state.create_object(card, searcher, Zone::Battlefield, None, None);
    let offered = [
        state.create_object(blank, searcher, Zone::Library, None, None),
        state.create_object(blank, searcher, Zone::Library, None, None),
    ];
    let other = state.create_object(blank, searcher, Zone::Library, None, None);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: searcher,
        source,
        choice: ResolutionChoiceKind::ChooseFromLibrary {
            description: String::new(),
            options: offered.to_vec(),
            searcher,
            source_id: source,
            destination: Zone::Hand,
            tapped: false,
        },
    });
    state.consecutive_passes = 2;
    let answer = match reply {
        0 | 1 => ResolvedChoice::ChosenCard(offered[usize::from(reply)]),
        2 => ResolvedChoice::ChosenTarget(None),
        _ => ResolvedChoice::ChosenCard(other),
    };

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &answer, &registry);

    match reply {
        0 | 1 => {
            assert!(matches!(applied, crate::engine::Applied::Continue));
            assert!(asked_for(&[Act::Move(offered[usize::from(reply)], Zone::Hand), Act::Shuffle(searcher)]));
            assert!(state.awaiting_action.is_none() && state.consecutive_passes == 0);
        }
        2 => {
            assert!(matches!(applied, crate::engine::Applied::Continue));
            assert!(asked_for(&[Act::Shuffle(searcher)]) && state.awaiting_action.is_none());
        }
        _ => {
            assert!(matches!(applied, crate::engine::Applied::ReturnNow));
            assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source: s,
                choice: ResolutionChoiceKind::ChooseFromLibrary { options, searcher: who, destination: Zone::Hand, tapped: false, .. },
            }) if *player == searcher && *s == source && *who == searcher && options.as_slice() == offered));
            assert!(acted() == 0 && state.consecutive_passes == 2);
        }
    }
    std::mem::forget((registry, state, answer));
}

/// The proofs of a search answered, one per reply.
macro_rules! each_search_reply {
    ($($name:ident: $reply:expr;)*) => {$(
        card_proof! {
            /// [`searched`] for the reply of its name.
            fn $name() {
                searched($reply);
            }
        }
    )*};
}

each_search_reply! {
    the_first_card_a_search_offered_is_found: 0;
    the_second_card_a_search_offered_is_found: 1;
    a_search_that_finds_nothing_still_shuffles: 2;
    a_card_a_search_did_not_offer_is_refused: 3;
}

/// What the asking card's hooks were told: a yes or no, and the length of
/// the card type chosen (the types offered are of lengths one to three,
/// each its own, so the length says which was handed on without comparing
/// a string read back off the heap).
static mut TOLD: Option<(ObjectId, Option<bool>, Option<usize>)> = None;

/// A card that asks a yes-or-no question and to name a card type, and is
/// told the answers.
struct Told;

impl CardBehavior for Told {
    fn card_data(&self) -> CardData {
        CardData { name: "Told".into(), ..CardData::default() }
    }
    fn on_yes_no_choice(&self, _: &mut GameState, id: ObjectId, yes: bool, _: &CardRegistry) {
        // SAFETY: as in `Asking::on_declined_choice`, for TOLD.
        let t = unsafe { &mut *std::ptr::addr_of_mut!(TOLD) };
        assert!(t.is_none(), "told once");
        *t = Some((id, Some(yes), None));
    }
    fn on_card_type_choice(&self, _: &mut GameState, id: ObjectId, chosen: &str, _: &CardRegistry) {
        // SAFETY: as in `on_yes_no_choice`.
        let t = unsafe { &mut *std::ptr::addr_of_mut!(TOLD) };
        assert!(t.is_none(), "told once");
        *t = Some((id, None, Some(chosen.len())));
    }
}

/// A yes-or-no question and a "name a card type" question answered
/// (`resolve_choice` of `YesNo` and `ChooseCardType`): the card that
/// asked is told the answer, once, as given (yes or no; the type offered
/// at the index chosen), the question answered and the passes in a row
/// starting again; a type past those offered is refused, the question
/// standing as it was asked and the card told nothing; and a yes or no
/// handed to the card type's question, or an index to the yes-or-no
/// question, is no answer to it, and refused the same way. What each card
/// does with its answer is its own proof. For the second player asked,
/// and each reply a harness's: 0 and 1 no and yes, 2 to 4 each type
/// offered, 5 one past them, 6 a yes to the type's question, 7 an index
/// to the yes-or-no question.
fn told(reply: u8) {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Told));
    let mut state = crate::proofs::cards::new_game();
    let chooser = PlayerId(1);
    let source = state.create_object(card, chooser, Zone::Battlefield, None, None);
    let yes_no = matches!(reply, 0 | 1 | 7);
    let choice = if yes_no {
        ResolutionChoiceKind::YesNo { description: String::new(), source_card: source }
    } else {
        ResolutionChoiceKind::ChooseCardType {
            description: String::new(),
            options: vec!["A".into(), "BB".into(), "CCC".into()],
            controller: chooser,
        }
    };
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice { player: chooser, source, choice });
    state.consecutive_passes = 2;
    let answer = match reply {
        0 | 1 => ResolvedChoice::YesNoDecision(reply == 1),
        6 => ResolvedChoice::YesNoDecision(true),
        k => ResolvedChoice::ChosenIndex(if k == 7 { 0 } else { usize::from(k - 2) }, String::new()),
    };

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &answer, &registry);

    // SAFETY: as in `Asking::on_declined_choice`.
    let t = unsafe { *std::ptr::addr_of!(TOLD) };
    match reply {
        0 | 1 => {
            assert!(matches!(applied, crate::engine::Applied::Continue));
            assert!(t == Some((source, Some(reply == 1), None)) && state.consecutive_passes == 0);
            assert!(state.awaiting_action.is_none());
        }
        2..=4 => {
            assert!(matches!(applied, crate::engine::Applied::Continue));
            assert!(t == Some((source, None, Some(usize::from(reply - 1)))) && state.consecutive_passes == 0);
            assert!(state.awaiting_action.is_none());
        }
        _ => {
            assert!(matches!(applied, crate::engine::Applied::ReturnNow));
            assert!(t.is_none() && state.consecutive_passes == 2);
            assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source: s, choice: ResolutionChoiceKind::YesNo { source_card, .. },
            }) if yes_no && *player == chooser && *s == source && *source_card == source)
                || matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source: s, choice: ResolutionChoiceKind::ChooseCardType { options, controller, .. },
            }) if !yes_no && *player == chooser && *s == source && *controller == chooser && options.len() == 3));
        }
    }
    std::mem::forget((registry, state, answer));
}

/// The proofs of a card told its answer, one per reply.
macro_rules! each_told_reply {
    ($($name:ident: $reply:expr;)*) => {$(
        card_proof! {
            /// [`told`] for the reply of its name.
            fn $name() {
                told($reply);
            }
        }
    )*};
}

each_told_reply! {
    a_no_is_told_to_the_card_that_asked: 0;
    a_yes_is_told_to_the_card_that_asked: 1;
    the_first_card_type_offered_is_told_to_the_card: 2;
    the_second_card_type_offered_is_told_to_the_card: 3;
    the_third_card_type_offered_is_told_to_the_card: 4;
    a_card_type_past_those_offered_is_refused: 5;
    a_yes_to_a_card_type_question_is_refused: 6;
    an_index_to_a_yes_or_no_question_is_refused: 7;
}

/// What `damage::apply_chosen_effect` was handed: who chose, the damage
/// (its source, the object it is to, how much, of which kind), and which
/// effect (0 a doubling, 1 a prevention and removal, each by its object).
static mut APPLIED_FIRST: Option<(PlayerId, ObjectId, Option<ObjectId>, u32, crate::damage::DamageKind, u8, ObjectId)> = None;

/// Stands in for `damage::apply_chosen_effect`, whose effects are proved
/// in `effects` and `damage`: recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn applied_first(_: &mut GameState, chooser: PlayerId, event: &crate::damage::PendingDamage, effect: &crate::damage::DamageEffect, _: &CardRegistry) {
    use crate::damage::DamageEffect;
    let (which, by) = match effect {
        DamageEffect::Double { by } => (0, *by),
        DamageEffect::PreventAndRemoveCounter { by } => (1, *by),
        _ => panic!("an effect this proof does not offer"),
    };
    let to = match event.target {
        crate::events::DamageTarget::Object(id) => Some(id),
        crate::events::DamageTarget::Player(_) => None,
    };
    assert!(event.applied.is_empty() && !event.settled, "the damage as it was asked about");
    // SAFETY: as in `Asking::on_declined_choice`, for APPLIED_FIRST.
    let a = unsafe { &mut *std::ptr::addr_of_mut!(APPLIED_FIRST) };
    assert!(a.is_none(), "applied once");
    *a = Some((chooser, event.source, to, event.amount, event.kind, which, by));
}

/// Which of the effects that apply to one damage event applies first,
/// answered (`resolve_choice` of `ChooseDamageEffect`, CR 616.1): the one
/// chosen is applied to the damage the question was about, by the player
/// the damage affects (the controller of the creature it is to), and the
/// rest is asked again if it must be (`damage`); one past those offered is
/// refused, the question standing as it was asked and nothing applied. For
/// combat damage of three to the second player's creature, a doubling and
/// a prevention offered, and each reply a harness's: 0 and 1 each, 2 one
/// past them.
fn damage_effect_chosen(reply: u8) {
    use crate::damage::{DamageEffect, DamageKind};
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(crate::proofs::cards::Blank));
    let mut state = crate::proofs::cards::new_game();
    let attacker = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    let creature = state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(3), Some(3));
    let (flail, horde) = (
        state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None),
        state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(1), Some(1)),
    );
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: PlayerId(1),
        source: attacker,
        choice: ResolutionChoiceKind::ChooseDamageEffect {
            description: String::new(),
            effects: vec![DamageEffect::Double { by: flail }, DamageEffect::PreventAndRemoveCounter { by: horde }],
            options: vec![String::new(), String::new()],
            source: attacker,
            target: crate::events::DamageTarget::Object(creature),
            amount: 3,
            kind: DamageKind::Combat,
        },
    });
    state.consecutive_passes = 2;

    let applied = crate::engine::actions::choices::resolve_choice(
        &mut state, &ResolvedChoice::ChosenIndex(usize::from(reply), String::new()), &registry);

    // SAFETY: as in `Asking::on_declined_choice`.
    let a = unsafe { *std::ptr::addr_of!(APPLIED_FIRST) };
    if reply < 2 {
        let by = if reply == 0 { flail } else { horde };
        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(a == Some((PlayerId(1), attacker, Some(creature), 3, DamageKind::Combat, reply, by)));
        assert!(state.awaiting_action.is_none() && state.consecutive_passes == 0);
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(a.is_none() && state.consecutive_passes == 2);
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player: PlayerId(1), choice: ResolutionChoiceKind::ChooseDamageEffect { effects, amount: 3, .. }, ..
        }) if effects.len() == 2));
    }
    std::mem::forget((registry, state));
}

/// The proofs of a damage effect chosen, one per reply.
macro_rules! each_effect_reply {
    ($($name:ident: $reply:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::damage::apply_chosen_effect, applied_first)]
            /// [`damage_effect_chosen`] for the reply of its name.
            fn $name() {
                damage_effect_chosen($reply);
            }
        }
    )*};
}

each_effect_reply! {
    the_doubling_chosen_applies_to_the_damage_first: 0;
    the_prevention_chosen_applies_to_the_damage_first: 1;
    a_damage_effect_past_those_offered_is_refused: 2;
}

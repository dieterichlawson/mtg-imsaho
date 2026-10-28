//! `engine::apply_pending_effect`, the action a card asks for when it asks
//! the engine to do something to a target it holds or one chosen as it
//! resolves: deal damage, destroy it, sacrifice it, return it to its
//! owner's hand, give it -N/-N or stop it blocking this turn, finish a
//! search with it, or carry out the card's own effect on it.

use super::action_proof;
use crate::actions::Target;
use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::PlayerId;
use crate::proofs::cards::{acted, any_target, asked_for, board, Act, Snapshot};
use crate::state::PendingEffect;
use crate::types::Zone;

action_proof! {
    /// "Deals N damage to that target", a spell's or ability's, is N
    /// non-combat damage dealt by that source to that creature,
    /// planeswalker or player (CR 120.1, 120.2b), and to a target that has
    /// become illegal, none: an illegal target is neither changed nor acted
    /// on (608.2b). For every target, every amount up to 5 and either
    /// object as the source; nothing else changes.
    fn damage_to_a_target_is_noncombat_damage_from_its_source() {
        let (registry, mut state, blank, _) = board(Box::new(crate::cards::grizzly_bears::GrizzlyBears));
        let creature = state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
        let spell = state.create_object(blank, PlayerId(0), Zone::Stack, None, None);
        let target = any_target([creature, spell]);
        let amount: u32 = kani::any();
        kani::assume(amount <= 5);
        let source = if kani::any() { spell } else { creature };
        let before = Snapshot::of(&state);

        crate::engine::apply_pending_effect(&mut state, &target, &PendingEffect::DealDamage { amount, source_id: source }, &registry);

        let dealt_to = match target {
            Target::Object(id) => Some(DamageTarget::Object(id)),
            Target::Player(p) => Some(DamageTarget::Player(p)),
            Target::Illegal => None,
        };
        match dealt_to {
            Some(t) => {
                assert!(asked_for(&[Act::Damage { source, target: t, amount, kind: DamageKind::NonCombat }]));
            }
            None => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&state) == before);
        kani::cover!(dealt_to.is_none(), "an illegal target");
        std::mem::forget((registry, state));
    }
}

/// Any object of the blank card's, of either player's, on the battlefield
/// or in a graveyard; and a target that is it, a player, or one gone
/// illegal.
fn any_object_or_target(state: &mut crate::state::GameState, blank: crate::ids::CardId) -> (crate::ids::ObjectId, bool, Target) {
    let on: bool = kani::any();
    let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), if on { Zone::Battlefield } else { Zone::Graveyard }, Some(2), Some(2));
    let target = match kani::any::<u8>() % 3 {
        0 => Target::Object(id),
        1 => Target::Player(PlayerId(u8::from(kani::any::<bool>()))),
        _ => Target::Illegal,
    };
    (id, on, target)
}

/// What each effect a card proof records `Act::Effect` for does: the
/// effect is the action named, of its target object, and nothing at all
/// for a target that is not an object. Proved one effect at a time.
macro_rules! effect_proof {
    ($(#[$meta:meta])* fn $name:ident($effect:expr) => |$state:ident, $id:ident, $on:ident| $expected:block) => {
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
        #[kani::stub(crate::destruction::try_destroy_by, crate::proofs::cards::record_destroy_by)]
        #[kani::stub(crate::destruction::sacrifice_by, crate::proofs::cards::record_sacrifice)]
        $(#[$meta])*
        fn $name() {
            let (registry, mut $state, blank) = crate::proofs::cards::blank_board();
            let (id, on, target) = any_object_or_target(&mut $state, blank);
            let before = Snapshot::of(&$state);

            crate::engine::apply_pending_effect(&mut $state, &target, &$effect, &registry);

            match target {
                Target::Object(_) => {
                    let ($id, $on) = (id, on);
                    // The actions asked for, as the effect says, and how many
                    // until-end-of-turn effects it added.
                    let expected: (bool, usize) = $expected;
                    assert!(expected.0);
                    assert!(Snapshot::of(&$state).without_until_end_of_turn() == before.without_until_end_of_turn());
                    assert!($state.until_end_of_turn.len() == expected.1);
                }
                _ => {
                    assert!(acted() == 0 && Snapshot::of(&$state) == before);
                }
            }
            std::mem::forget((registry, $state));
        }
    };
}

effect_proof! {
    /// "Destroy target creature" and "destroy target permanent" (CR
    /// 701.7a): destroying it, whose outcome is `try_destroy`'s.
    fn a_pending_destruction_destroys_its_target(PendingEffect::Destroy { source_name: "a source".into() })
        => |state, id, _on| { (asked_for(&[Act::Destroy(id)]), 0) }
}

effect_proof! {
    /// The same, for the creature-only form.
    fn a_pending_creature_destruction_destroys_its_target(PendingEffect::DestroyCreature { source_name: "a source".into() })
        => |state, id, _on| { (asked_for(&[Act::Destroy(id)]), 0) }
}

effect_proof! {
    /// "Sacrifice a creature" (CR 701.17a), the one chosen: sacrificing it,
    /// which only a permanent can be (`sacrifice_by`'s proof).
    fn a_pending_sacrifice_sacrifices_its_target(PendingEffect::SacrificeCreature { source_name: "a source".into() })
        => |state, id, _on| { (asked_for(&[Act::Sacrifice(id)]), 0) }
}

effect_proof! {
    /// "Return it to its owner's hand": the move, from wherever it is.
    fn a_pending_return_returns_its_target_to_hand(PendingEffect::ReturnToHand { source_name: "a source".into() })
        => |state, id, _on| { (asked_for(&[Act::Move(id, Zone::Hand)]), 0) }
}

effect_proof! {
    /// "It gets -N/-N until end of turn" (CR 611.2c): it gets it. One on a
    /// card that has left the battlefield applies to nothing, the card a
    /// new object (400.7): the engine ends it as the action ends, before
    /// anyone can act (`prune_effects_on_departed_objects`, proved below).
    fn a_pending_debuff_lasts_the_turn(PendingEffect::DebuffUntilEOT { power: -1, toughness: -2, source_name: "a source".into() })
        => |state, id, _on| {
            let added = state.until_end_of_turn.as_slice();
            (acted() == 0 && matches!(added, [crate::state::TemporaryEffect::ModifyPT { target, power_mod: -1, toughness_mod: -2 }] if *target == id), 1)
        }
}

effect_proof! {
    /// "It can't block this turn" (CR 611.2c): it can't; and the same of a
    /// card off the battlefield.
    fn a_pending_cant_block_lasts_the_turn(PendingEffect::CantBlockThisTurn { source_name: "a source".into() })
        => |state, id, _on| {
            let added = state.until_end_of_turn.as_slice();
            (acted() == 0 && matches!(added, [crate::state::TemporaryEffect::CantBlock { target }] if *target == id), 1)
        }
}

/// An until-end-of-turn effect of kind `kind` (one of nine) about
/// `target`, and whether it is about that permanent: it gets +N/+N, gains
/// or loses a keyword, can't block, has protection, or is controlled by
/// another player (CR 611.2c), or gets +N/+N while a source stays; not a
/// card given flashback in a graveyard (702.34), and not combat damage
/// prevented this turn, which is about no object.
fn an_effect(kind: u8, target: crate::ids::ObjectId) -> (crate::state::TemporaryEffect, bool) {
    use crate::state::TemporaryEffect as T;
    use crate::types::{CreatureFilter, Keyword, ManaCost};
    match kind % 9 {
        0 => (T::ModifyPT { target, power_mod: 1, toughness_mod: -1 }, true),
        1 => (T::GrantKeyword { target, keyword: Keyword::Flying }, true),
        2 => (T::RemoveKeyword { target, keyword: Keyword::Defender }, true),
        3 => (T::CantBlock { target }, true),
        4 => (T::GrantProtection { target, filter: CreatureFilter::ControlledByOpponent }, true),
        5 => (T::ChangeControl { target, controller: PlayerId(1), timestamp: 3 }, true),
        6 => (T::ModifyPTWhileSourceInPlay { target, source: target, power_mod: 1, toughness_mod: 0 }, true),
        7 => (T::GrantFlashback { target, cost: ManaCost::free() }, false),
        _ => (T::PreventCombatDamageExcept { filter: CreatureFilter::ControlledByYou, source_name: String::new() }, false),
    }
}

/// `prune_effects_on_departed_objects` over effects of the kinds given, in
/// that order, each about either of two objects (each on the battlefield
/// or in a graveyard) or one no longer in the game. The kinds are
/// constants: an effect of a kind the solver left open is dropped, when
/// pruned, as every kind at once, a creature filter's every variant
/// included, and the proof ran past eight gigabytes.
fn pruning<const N: usize>(kinds: [u8; N]) {
    let (_registry, mut state, blank) = crate::proofs::cards::blank_board();
    let mut on = [false; 2];
    let mut ids = [crate::ids::ObjectId(0); 3];
    for (i, o) in on.iter_mut().enumerate() {
        *o = kani::any();
        ids[i] = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), if *o { Zone::Battlefield } else { Zone::Graveyard }, Some(2), Some(2));
    }
    ids[2] = crate::ids::ObjectId(ids[1].0 + 1);
    let about: [u8; N] = kani::any();
    let target = |j: usize| ids[usize::from(about[j] % 3)];
    for j in 0..N {
        state.until_end_of_turn.push(an_effect(kinds[j], target(j)).0);
    }
    let before = Snapshot::of(&state);

    state.prune_effects_on_departed_objects();

    // Kept, in order: each effect about no permanent, and each about one
    // still on the battlefield.
    let mut n = 0;
    for j in 0..N {
        let (effect, about_a_permanent) = an_effect(kinds[j], target(j));
        let on_battlefield = about[j] % 3 < 2 && on[usize::from(about[j] % 3)];
        if !about_a_permanent || on_battlefield {
            assert!(state.until_end_of_turn.get(n) == Some(&effect));
            n += 1;
        }
        std::mem::forget(effect);
    }
    assert!(state.until_end_of_turn.len() == n);
    assert!(Snapshot::of(&state).without_until_end_of_turn() == before.without_until_end_of_turn());
    std::mem::forget(state);
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
/// An effect that lasts the turn on a permanent that has left the
/// battlefield applies to nothing, and the card it became is a new object
/// that must not carry it back (CR 400.7): as each action ends, the engine
/// ends it. One about no permanent stays. For an effect of each of the
/// nine kinds, about either object or one gone.
fn an_effect_on_a_departed_permanent_ends() {
    match kani::any::<u8>() % 9 {
        0 => pruning([0]),
        1 => pruning([1]),
        2 => pruning([2]),
        3 => pruning([3]),
        4 => pruning([4]),
        5 => pruning([5]),
        6 => pruning([6]),
        7 => pruning([7]),
        _ => pruning([8]),
    }
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
/// Of several effects, those that end end and the rest stay, in the order
/// they were: up to three, a card's flashback among two about permanents,
/// each about either object or one gone.
fn effects_on_departed_permanents_end_and_the_rest_stay_in_order() {
    match kani::any::<u8>() % 4 {
        0 => pruning([]),
        1 => pruning([0, 3]),
        2 => pruning([7, 0]),
        _ => pruning([0, 7, 3]),
    }
}

/// What a card's own effect was asked to do: its source, whether its key
/// was the one given, and its target.
static mut ASKED: Option<(crate::ids::ObjectId, bool, Target)> = None;

/// A card whose own effect records that it was asked.
struct OwnEffect;

impl crate::cards::CardBehavior for OwnEffect {
    fn card_data(&self) -> crate::cards::CardData {
        crate::cards::CardData { name: "Own Effect".into(), ..crate::cards::CardData::default() }
    }

    fn resolve_card_effect(&self, _: &mut crate::state::GameState, source_id: crate::ids::ObjectId, key: &str, target: &Target, _: &crate::cards::CardRegistry) {
        // SAFETY: a proof runs on one thread, and only this module names it.
        unsafe {
            *std::ptr::addr_of_mut!(ASKED) = Some((source_id, key == "the key", target.clone()));
        }
    }
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// A card's own effect, chosen for a target as it resolves, is that
/// card's to carry out: the source's card is asked to resolve it, by the
/// key it was given and for the target chosen, whatever that is (a target
/// gone illegal included, which the card's own proof says it ignores); a
/// source no longer in the game asks nothing.
fn a_card_effect_is_the_cards_own_to_carry_out() {
    let (registry, mut state, blank, card) = board(Box::new(OwnEffect));
    let source = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())), if kani::any() { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    let other = state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    let gone: bool = kani::any();
    let source_id = if gone { crate::ids::ObjectId(other.0 + 1) } else { source };
    let target = any_target([source, other]);
    let before = Snapshot::of(&state);

    crate::engine::apply_pending_effect(&mut state, &target, &PendingEffect::CardEffect { source_id, key: "the key".into() }, &registry);

    // SAFETY: as in `OwnEffect`.
    let asked = unsafe { (*std::ptr::addr_of!(ASKED)).clone() };
    if gone {
        assert!(asked.is_none());
    } else {
        assert!(asked == Some((source, true, target)));
    }
    assert!(acted() == 0 && Snapshot::of(&state) == before);
    std::mem::forget((registry, state, asked));
}

/// What finishing a search was asked: who searched, the card found, where
/// it goes, and whether tapped.
static mut FINISHED: Option<(PlayerId, crate::ids::ObjectId, Zone, bool)> = None;

/// Stands in for `helpers::finish_library_search`, whose own proof says
/// what it does.
#[allow(dead_code)] // named only by `kani::stub`
fn record_finish(_: &mut crate::state::GameState, searcher: PlayerId, found: crate::ids::ObjectId, destination: Zone, tapped: bool, _: &crate::cards::CardRegistry) {
    // SAFETY: as in `OwnEffect`.
    unsafe {
        *std::ptr::addr_of_mut!(FINISHED) = Some((searcher, found, destination, tapped));
    }
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::cards::helpers::finish_library_search, record_finish)]
/// A search's answer (CR 701.19): the card chosen is found, and the search
/// finished as it said, by the searcher, to its zone, tapped or not; an
/// answer that is no card (the search declined) finishes nothing.
fn a_searchs_answer_finishes_the_search_with_that_card() {
    let (registry, mut state, blank) = crate::proofs::cards::blank_board();
    let found = state.create_object(blank, PlayerId(0), Zone::Library, None, None);
    let target = if kani::any() { Target::Object(found) } else { Target::Illegal };
    let searcher = PlayerId(u8::from(kani::any::<bool>()));
    let destination = if kani::any() { Zone::Hand } else { Zone::Battlefield };
    let tapped: bool = kani::any();
    let before = Snapshot::of(&state);

    crate::engine::apply_pending_effect(&mut state, &target, &PendingEffect::FinishLibrarySearch { searcher, destination, tapped }, &registry);

    // SAFETY: as in `OwnEffect`.
    let finished = unsafe { *std::ptr::addr_of!(FINISHED) };
    match target {
        Target::Object(_) => {
            assert!(finished == Some((searcher, found, destination, tapped)));
        }
        _ => {
            assert!(finished.is_none());
        }
    }
    assert!(acted() == 0 && Snapshot::of(&state) == before);
    std::mem::forget((registry, state));
}

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
    /// "It gets -N/-N until end of turn" (CR 611.2c): the permanent gets
    /// it, and a card off the battlefield, a new object (400.7), nothing.
    fn a_pending_debuff_lasts_the_turn_on_a_permanent(PendingEffect::DebuffUntilEOT { power: -1, toughness: -2, source_name: "a source".into() })
        => |state, id, on| {
            let added = state.until_end_of_turn.as_slice();
            (acted() == 0 && (!on || matches!(added, [crate::state::TemporaryEffect::ModifyPT { target, power_mod: -1, toughness_mod: -2 }] if *target == id)),
                usize::from(on))
        }
}

effect_proof! {
    /// "It can't block this turn" (CR 611.2c): the permanent can't, and a
    /// card off the battlefield is untouched.
    fn a_pending_cant_block_lasts_the_turn_on_a_permanent(PendingEffect::CantBlockThisTurn { source_name: "a source".into() })
        => |state, id, on| {
            let added = state.until_end_of_turn.as_slice();
            (acted() == 0 && (!on || matches!(added, [crate::state::TemporaryEffect::CantBlock { target }] if *target == id)),
                usize::from(on))
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

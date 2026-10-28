//! Dealing damage, which a card proof's `record_damage`, `record_queue`
//! and `record_deal_queued` stand in for: `damage::deal_damage`, and
//! `damage::queue_damage` with `damage::process_pending_damage`, which a
//! spell dealing damage to several at once (CR 120.1) uses.
//!
//! Here, damage that no prevention or replacement effect applies to: which
//! ones apply (`applicable_effects`) is answered with none. Whether the
//! source has deathtouch or lifelink is any answer (`proofs::decide`), the
//! same each time it is asked; which objects have them is `has_keyword`'s
//! own proof.

use crate::cards::CardRegistry;
use crate::damage::{self, DamageEffect, DamageKind, PendingDamage};
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{blank_board, Snapshot};
use crate::proofs::decide::{decided, Question};
use crate::state::GameState;
use crate::types::{CardType, CounterType, Keyword, Zone};

/// Stands in for `damage::applicable_effects`: none applies.
#[allow(dead_code)] // named only by `kani::stub`
fn none_applies(_: &GameState, _: &PendingDamage, _: &CardRegistry) -> Vec<DamageEffect> {
    Vec::new()
}

/// Stands in for `GameState::has_keyword`: dealing damage asks only about
/// deathtouch and lifelink.
#[allow(dead_code)] // named only by `kani::stub`
fn has_keyword(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    match keyword {
        Keyword::Deathtouch => decided(Question::HasDeathtouch, id),
        Keyword::Lifelink => decided(Question::HasLifelink, id),
        _ => panic!("dealing damage asks only about deathtouch and lifelink"),
    }
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::damage::applicable_effects, none_applies)]
#[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
/// "N damage to any target", combat or not, that nothing prevents or
/// replaces (CR 120.3): damage to a player is that much life lost
/// (120.3a); to a planeswalker, that many loyalty counters removed, as many
/// as it has if fewer (120.3c); to any other permanent, that much damage
/// marked on it (120.3e), and marked as deathtouch damage if the source
/// has deathtouch (702.2b); the source is then among what has damaged it.
/// The damage is announced as combat damage or not, once, and if the
/// source has lifelink its controller gains that much life (702.15b). No
/// damage is dealt at all by a source dealing 0 (120.8), or to an object
/// no longer on the battlefield. Through `deal_damage`, and through
/// `queue_damage` then `process_pending_damage`, after which nothing is
/// waiting; for a source of either player's on the battlefield or not, a
/// creature or planeswalker of either player's with damage or loyalty on
/// it, either player at any life an `i16` holds, and up to 20 damage.
/// Nothing else changes.
fn damage_nothing_prevents_or_replaces_is_dealt() {
    let (registry, mut state, blank) = blank_board();
    let totals: [i16; 2] = [kani::any(), kani::any()];
    state.players[0].life = i32::from(totals[0]);
    state.players[1].life = i32::from(totals[1]);
    let sc = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(blank, sc, if kani::any() { Zone::Battlefield } else { Zone::Graveyard }, Some(2), Some(2));
    let tc = PlayerId(u8::from(kani::any::<bool>()));
    let on: bool = kani::any();
    let walker: bool = kani::any();
    let t = state.create_object(blank, tc, if on { Zone::Battlefield } else { Zone::Graveyard }, Some(3), Some(3));
    let (marked, loyalty): (u32, u32) = (kani::any(), kani::any());
    kani::assume(marked <= 20 && loyalty <= 3);
    let damaged_before: bool = kani::any();
    let o = state.get_object_mut(t).expect("just created");
    if walker {
        o.card_types = vec![CardType::Planeswalker];
        o.power = None;
        o.toughness = None;
        if loyalty > 0 {
            o.counters.insert(CounterType::Loyalty, loyalty);
        }
    } else {
        o.damage_marked = marked;
    }
    o.damaged_by = Vec::with_capacity(2);
    if damaged_before {
        o.damaged_by.push(source);
    }
    let target = if kani::any() { DamageTarget::Object(t) } else { DamageTarget::Player(PlayerId(u8::from(kani::any::<bool>()))) };
    let amount: u32 = kani::any();
    kani::assume(amount <= 20);
    let kind = if kani::any() { DamageKind::Combat } else { DamageKind::NonCombat };
    let before = Snapshot::of(&state);

    if kani::any() {
        damage::deal_damage(&mut state, source, target, amount, kind, &registry);
    } else {
        damage::queue_damage(&mut state, source, target, amount, kind);
        damage::process_pending_damage(&mut state, &registry);
    }

    assert!(state.pending_damage.is_empty());
    let dealt = amount > 0 && match target {
        DamageTarget::Object(_) => on,
        DamageTarget::Player(_) => true,
    };
    if !dealt {
        assert!(state.events.is_empty() && Snapshot::of(&state) == before);
        let o = state.get_object(t).expect("on the board");
        assert!(o.damaged_by.len() == usize::from(damaged_before) && !o.dealt_deathtouch_damage);
        std::mem::forget((registry, state));
        return;
    }
    let n = i32::try_from(amount).expect("small");
    let mut expected = before.without_events();
    let mut events = 0;
    match target {
        DamageTarget::Player(p) => {
            let old = i32::from(totals[usize::from(p.0)]);
            assert!(matches!(state.events[0], GameEvent::LifeChanged { player, old: o, new_life } if player == p && o == old && new_life == old - n));
            expected = expected.with_life(p, old - n);
            events += 1;
        }
        DamageTarget::Object(_) => {
            let o = state.get_object(t).expect("on the board");
            if walker {
                expected = expected.with_counters(t, CounterType::Loyalty, loyalty.saturating_sub(amount));
                assert!(loyalty > amount || !o.counters.contains_key(&CounterType::Loyalty));
                assert!(!o.dealt_deathtouch_damage);
            } else {
                expected = expected.with_damage(t, marked + amount);
                assert!(o.dealt_deathtouch_damage == decided(Question::HasDeathtouch, source));
            }
            assert!(o.damaged_by.len() == 1 && o.damaged_by[0] == source);
        }
    }
    assert!(match (kind, &state.events[events]) {
        (DamageKind::Combat, GameEvent::CombatDamageDealt { source: s, target: d, amount: a })
        | (DamageKind::NonCombat, GameEvent::NonCombatDamageDealt { source: s, target: d, amount: a }) =>
            *s == source && *d == target && *a == amount,
        _ => false,
    });
    events += 1;
    if decided(Question::HasLifelink, source) {
        let old = match target {
            DamageTarget::Player(p) if p == sc => i32::from(totals[usize::from(sc.0)]) - n,
            _ => i32::from(totals[usize::from(sc.0)]),
        };
        assert!(matches!(state.events[events], GameEvent::LifeChanged { player, old: o, new_life } if player == sc && o == old && new_life == old + n));
        expected = expected.with_life(sc, old + n);
        events += 1;
    }
    assert!(state.events.len() == events && Snapshot::of(&state).without_events() == expected);
    kani::cover!(walker && on && loyalty > 0 && amount >= loyalty, "a planeswalker loses its last loyalty");
    kani::cover!(matches!(target, DamageTarget::Player(p) if p == sc) && decided(Question::HasLifelink, source), "lifelink on its own controller");
    std::mem::forget((registry, state));
}

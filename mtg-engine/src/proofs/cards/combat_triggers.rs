//! The creatures whose abilities trigger on the combat damage they deal
//! (CR 510.3a, 603.2), and the statics printed beside them.
//!
//! - Balefire Dragon, 6/6 Flying: "Whenever this creature deals combat
//!   damage to a player, it deals that much damage to each creature that
//!   player controls."
//! - Bloodcrazed Neonate, 2/1: "This creature attacks each combat if able.
//!   Whenever this creature deals combat damage to a player, put a +1/+1
//!   counter on it."
//! - Falkenrath Marauders, 2/2 Flying, haste: "Whenever this creature deals
//!   combat damage to a player, put two +1/+1 counters on it."
//! - Stromkirk Noble, 1/1: "This creature can't be blocked by Humans.
//!   Whenever this creature deals combat damage to a player, put a +1/+1
//!   counter on it."
//! - Stromkirk Patrol, 4/3: "Whenever this creature deals combat damage to
//!   a player, put a +1/+1 counter on it."
//! - Creepy Doll, 1/1 Indestructible: "Whenever this creature deals combat
//!   damage to a creature, flip a coin. If you win the flip, destroy that
//!   creature."
//!
//! Each proof pins the trigger the card declares, which is what the engine
//! reads to put it on the stack, and then runs the card's code for that
//! trigger resolving: over the creature on the battlefield or gone from it
//! by then (the ability resolves all the same, CR 113.7a), either player
//! damaged, any amount, and a board of a card and a token of any
//! characteristics. That the engine puts the trigger on the stack exactly
//! when the event happens, with that player and amount, is
//! `proofs::actions`'.

use super::{act, acted, any_object, asked_for, board, card_proof, has_subtype, prints_effects, prints_where, triggers, were, Act, Form, Kinds, Snapshot};
use crate::cards::{CardBehavior, CardRegistry, TriggerKind};
use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{CardType, ContinuousEffect as E, CounterType, CreatureFilter as F, EffectScope, Zone};

use crate::cards::isd;

/// The card's creature, controlled by player 0, on the battlefield or in
/// its owner's graveyard, beside a card and a token of any characteristics
/// `kinds` allows: the board, the creature, and the other two.
fn creature_and_board(card: Box<dyn CardBehavior>, kinds: &Kinds) -> (CardRegistry, GameState, ObjectId, [ObjectId; 2]) {
    let (registry, mut state, blank, card) = board(card);
    let on: bool = kani::any();
    let me = state.create_object(card, PlayerId(0), if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    if !on {
        state.players[0].graveyard_order.push(me);
    }
    let others = [any_object(&mut state, blank, kinds, Form::Card), any_object(&mut state, blank, kinds, Form::Token)];
    (registry, state, me, others)
}

const ANY_CREATURE: Kinds = Kinds { types: &[CardType::Creature, CardType::Artifact], subtypes: &[], keywords: &[] };

/// `card`'s "deals combat damage to a player" trigger resolving, for either
/// player and any amount up to 10: the board after, the creature, the
/// other objects, the player and amount, and the board before.
fn deals_combat_damage_to_a_player(card: Box<dyn CardBehavior>) -> (CardRegistry, GameState, ObjectId, [ObjectId; 2], PlayerId, u32, Snapshot) {
    assert!(triggers(card.as_ref()) == [(TriggerKind::CombatDamageToPlayer, None)]);
    let (registry, mut state, me, others) = creature_and_board(card, &ANY_CREATURE);
    let player = PlayerId(u8::from(kani::any::<bool>()));
    let amount: u32 = kani::any();
    kani::assume((1..=10).contains(&amount));
    let before = Snapshot::of(&state);
    let card = state.get_object(me).expect("on the board").card_id;
    registry.get(card).expect("registered").on_combat_damage_to_player(&mut state, me, player, amount, &registry);
    (registry, state, me, others, player, amount, before)
}

/// The trigger puts `n` +1/+1 counters on the creature (CR 122.1), and
/// does nothing else; a creature gone from the battlefield takes none,
/// which `add_counters` decides.
fn grows_by(card: Box<dyn CardBehavior>, n: u32) {
    let (registry, state, me, _, _, _, before) = deals_combat_damage_to_a_player(card);
    assert!(asked_for(&[Act::Counters(me, CounterType::PlusOnePlusOne, n)]) && Snapshot::of(&state) == before);
    std::mem::forget((registry, state));
}

card_proof! {
    /// Bloodcrazed Neonate: attacks each combat if able (CR 508.1d), and a
    /// +1/+1 counter when it deals combat damage to a player.
    fn bloodcrazed_neonate_attacks_and_grows() {
        assert!(prints_effects(&isd::bloodcrazed_neonate::BloodcrazedNeonate, &[E::ForceAttack { scope: EffectScope::OnSelf }]));
        grows_by(Box::new(isd::bloodcrazed_neonate::BloodcrazedNeonate), 1);
    }
}

card_proof! {
    /// Falkenrath Marauders: two +1/+1 counters when it deals combat damage
    /// to a player.
    fn falkenrath_marauders_grows_by_two() {
        assert!(prints_effects(&isd::falkenrath_marauders::FalkenrathMarauders, &[]));
        grows_by(Box::new(isd::falkenrath_marauders::FalkenrathMarauders), 2);
    }
}

card_proof! {
    /// Stromkirk Noble: can't be blocked by Humans (CR 509.1b), and a +1/+1
    /// counter when it deals combat damage to a player.
    fn stromkirk_noble_evades_humans_and_grows() {
        assert!(prints_where(&isd::stromkirk_noble::StromkirkNoble, |e| matches!(e, [E::CanOnlyBeBlockedBy {
            allowed_blockers: F::Not(not_human), scope: EffectScope::OnSelf,
        }] if has_subtype(not_human, "Human"))));
        grows_by(Box::new(isd::stromkirk_noble::StromkirkNoble), 1);
    }
}

card_proof! {
    /// Stromkirk Patrol: a +1/+1 counter when it deals combat damage to a
    /// player.
    fn stromkirk_patrol_grows() {
        assert!(prints_effects(&isd::stromkirk_patrol::StromkirkPatrol, &[]));
        grows_by(Box::new(isd::stromkirk_patrol::StromkirkPatrol), 1);
    }
}

card_proof! {
    /// Balefire Dragon: as much damage as it dealt the player, from the
    /// Dragon, to each creature that player controls, all at once; and
    /// nothing else. Whether the Dragon is still there does not matter
    /// (CR 113.7a, 608.2h: it deals the damage as it last existed).
    fn balefire_dragon_deals_that_much_to_each_of_their_creatures() {
        assert!(prints_effects(&isd::balefire_dragon::BalefireDragon, &[]));
        let (registry, state, me, others, player, amount, before) =
            deals_combat_damage_to_a_player(Box::new(isd::balefire_dragon::BalefireDragon));
        let theirs: Vec<ObjectId> = others.iter().copied()
            .filter(|id| state.get_object(*id).is_some_and(|o| {
                o.zone == Zone::Battlefield && o.controller == player && o.card_types.contains(&CardType::Creature)
            }))
            .collect();
        let n = theirs.len();
        let queued = |i: usize| Act::Queue { source: me, target: DamageTarget::Object(theirs[i]), amount, kind: DamageKind::NonCombat };
        assert!(acted() == n + 1 && were(0, n, queued) && were(n, 1, |_| Act::DealQueued));
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state, theirs));
    }
}

card_proof! {
    /// Creepy Doll: when it deals combat damage to a creature, a coin
    /// flipped, and that creature destroyed if the flip is won (whether it
    /// is still there to destroy is the action's); and nothing else.
    fn creepy_doll_destroys_on_a_won_flip() {
        assert!(triggers(&isd::creepy_doll::CreepyDoll) == [(TriggerKind::DealsCombatDamageToCreature, None)]);
        assert!(prints_effects(&isd::creepy_doll::CreepyDoll, &[]));
        let (registry, mut state, me, others) = creature_and_board(Box::new(isd::creepy_doll::CreepyDoll), &ANY_CREATURE);
        let damaged = if kani::any() { others[0] } else { others[1] };
        let amount: u32 = kani::any();
        kani::assume((1..=10).contains(&amount));
        let before = Snapshot::of(&state);
        let card = state.get_object(me).expect("on the board").card_id;
        registry.get(card).expect("registered").on_deals_combat_damage_to_creature(&mut state, me, damaged, amount, &registry);
        match act(0) {
            Some(Act::FlipCoin(true)) => {
                assert!(acted() == 2 && Some(&Act::Destroy(damaged)) == act(1));
            }
            Some(Act::FlipCoin(false)) => {
                assert!(acted() == 1);
            }
            _ => panic!("flips a coin first"),
        }
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

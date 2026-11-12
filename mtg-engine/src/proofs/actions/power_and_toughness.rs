//! A creature's power and toughness as the engine works them out
//! (`GameState::effective_power` and `effective_toughness`, CR 613.4), from
//! what a card proof sees a card leave on the board: a setting (Tree of
//! Redemption's toughness, which `set_base_pt` keeps with the latest
//! timestamp), +1/+1 and -1/-1 counters, and changes until end of turn,
//! all at once. Each alone is a proof of its own as well: the changes'
//! in `until_end_of_turn`, an Aura's or Equipment's in `effects`.

use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::Blank;
use crate::state::{SetPtEffect, TemporaryEffect};
use crate::types::{CardType, CounterType, Zone};

/// Any amount from 0 to `max`.
fn up_to(max: u8) -> u8 {
    let n: u8 = kani::any();
    kani::assume(n <= max);
    n
}

/// Its power and toughness of a permanent (CR 613.4): its own, or for each
/// of the two what the latest setting of it says (layer 7b; 613.7a: the
/// later applies after, and so stands), and then plus each +1/+1 counter
/// on it, less each -1/-1 counter, plus each change until end of turn on
/// it (7c). Nothing reaches the other creature but what is on it, and a
/// creature off the battlefield has its own (613.1). Over two creatures of
/// the blank card (either player's, power 0 to 3 and toughness 1 to 3),
/// each on the battlefield or in a graveyard; `n` settings in the order
/// made, each on either creature, of its power, its toughness or both,
/// each to 0 to 5; up to two of each kind of counter on each; and a change
/// of -2 to 2 to each of power and toughness on either.
fn working_out(n: usize) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    state.set_pt_effects = Vec::with_capacity(n);
    let mut creatures = [ObjectId(0); 2];
    let mut base = [(0, 0); 2];
    let mut on = [false; 2];
    let mut counted = [0; 2];
    for i in 0..2 {
        let (p, t) = (up_to(3), up_to(2) + 1);
        on[i] = kani::any();
        creatures[i] = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())),
            if on[i] { Zone::Battlefield } else { Zone::Graveyard }, Some(i32::from(p)), Some(i32::from(t)));
        base[i] = (i32::from(p), i32::from(t));
        let (plus, minus) = (up_to(2), up_to(2));
        let o = state.get_object_mut(creatures[i]).expect("just created");
        o.card_types = vec![CardType::Creature];
        o.counters.insert(CounterType::PlusOnePlusOne, u32::from(plus));
        o.counters.insert(CounterType::MinusOneMinusOne, u32::from(minus));
        counted[i] = i32::from(plus) - i32::from(minus);
    }
    let mut set: [(Option<i32>, Option<i32>); 2] = [(None, None); 2];
    for k in 0..n {
        let i = usize::from(kani::any::<bool>());
        let half = |sets: bool| if sets { Some(i32::from(up_to(5))) } else { None };
        let (power, toughness) = match kani::any::<u8>() % 3 {
            0 => (half(true), None),
            1 => (None, half(true)),
            _ => (half(true), half(true)),
        };
        state.set_pt_effects.push(SetPtEffect { object: creatures[i], power, toughness, timestamp: 1 + k as u64 });
        set[i] = (power.or(set[i].0), toughness.or(set[i].1));
    }
    let changed = usize::from(kani::any::<bool>());
    let (dp, dt): (i8, i8) = (kani::any(), kani::any());
    kani::assume((-2..=2).contains(&dp) && (-2..=2).contains(&dt));
    state.until_end_of_turn.push(TemporaryEffect::ModifyPT { target: creatures[changed], power_mod: i32::from(dp), toughness_mod: i32::from(dt) });
    for i in 0..2 {
        let (bp, bt) = base[i];
        let (p, t) = if on[i] {
            let (cp, ct) = if i == changed { (i32::from(dp), i32::from(dt)) } else { (0, 0) };
            (set[i].0.unwrap_or(bp) + counted[i] + cp, set[i].1.unwrap_or(bt) + counted[i] + ct)
        } else {
            (bp, bt)
        };
        assert!(state.effective_power(creatures[i], &registry) == Some(p));
        assert!(state.effective_toughness(creatures[i], &registry) == Some(t));
    }
    std::mem::forget((registry, state));
}

/// [`working_out`] for each number of settings.
macro_rules! each_setting {
    ($($name:ident: $n:literal),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        /// The proof above it, for one number of settings.
        fn $name() {
            working_out($n);
        }
    )*};
}

each_setting! {
    power_and_toughness_with_no_setting_are_their_own_changed: 0,
    power_and_toughness_with_one_setting_are_what_it_sets_changed: 1,
    power_and_toughness_with_two_settings_are_what_the_later_sets_changed: 2,
}

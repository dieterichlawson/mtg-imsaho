//! Paying a mana cost out of a mana pool: `mana::can_pay`, `auto_pay` and
//! `auto_pay_reserving`.
//!
//! The specification. A cost is a multiset of pips: a coloured pip is paid
//! only with mana of its colour, a `{C}` pip only with colorless mana, and a
//! generic pip with mana of any type (CR 107.4, 601.2h); `{X}` is zero when
//! it is not being announced (CR 107.3e). Every pip but the generic ones
//! accepts exactly one type, so a pool can pay a cost exactly when it holds
//! enough of each type for that type's own pips, and enough in all for every
//! pip together — [`spec_payable`]. That is Hall's condition for this
//! matching, and it is the whole of what "can pay" means.
//!
//! The public functions are each one line over three pieces:
//!
//! - `pips_of` turns a cost into per-type and generic counts, proved by
//!   [`counting_a_cost_counts_every_pip`];
//! - `pay_fixed` and `pay_around` pay those counts out of six per-type
//!   amounts, proved by the rest of this file;
//! - `amounts_of` and `write_amounts` read the pool into six amounts and
//!   write the result back. These are not proved: the pool is a `BTreeMap`,
//!   and CBMC does not finish on six `BTreeMap` inserts in five minutes.
//!   They are six lines each, and `mana::tests` covers them.
//!
//! Bounds. The payment proofs quantify over counts, not over symbol lists,
//! so a cost or a reserve of any length is covered as long as it asks at
//! most [`MAX_MANA`] of each type and at most [`MAX_MANA`] generic, out of a
//! pool holding at most [`MAX_MANA`] of each type. That is every one of
//! about 10^12 combinations of pool, cost and reserve, and each defect fixed
//! alongside these proofs shows with two of a type. It is not every cost in
//! the card pool: Army of the Damned's flashback asks `{7}` generic. The
//! bound is the checker's, not the code's: CBMC reasons about sums bit by
//! bit, and at 3 the slowest payment proof takes under two minutes, while
//! at 5 the reserve proof did not finish in nine. Proving them for every
//! amount is a job for a verifier that reasons about integers rather than
//! bits (`docs/plans/formal-verification.md`).
//!
//! The counting lemma is the one proof over symbol lists, and takes costs
//! of up to [`MAX_SYMBOLS`] symbols: every printed cost, but not a reserve
//! built from a whole hand's pips, which can be longer.

use crate::mana::{pay_around, pay_fixed, pips_of, slot_of, Amounts, Pips};
use crate::types::{Color, ManaCost, ManaSymbol, ManaType};

/// The most of one type of mana a pool, a cost or a reserve holds in these
/// proofs, and the most generic mana a cost or a reserve asks for. The
/// module doc says why it is this small.
const MAX_MANA: u32 = 3;
/// The longest printed cost in the card pool is four symbols
/// (Army of the Damned's `{5}{B}{B}{B}`).
const MAX_SYMBOLS: usize = 4;

const TYPES: [ManaType; 6] = [
    ManaType::White, ManaType::Blue, ManaType::Black,
    ManaType::Red, ManaType::Green, ManaType::Colorless,
];

fn total(a: &Amounts) -> u32 {
    a.iter().sum()
}

/// The specification: enough of each type for its own pips, and enough in
/// all for every pip.
fn spec_payable(have: &Amounts, cost: &Pips) -> bool {
    (0..6).all(|i| have[i] >= cost.specific[i])
        && total(have) >= total(&cost.specific) + cost.generic
}

/// Two costs paid together: every pip of both.
fn both(a: &Pips, b: &Pips) -> Pips {
    Pips {
        specific: std::array::from_fn(|i| a.specific[i] + b.specific[i]),
        generic: a.generic + b.generic,
    }
}

/// What a successful payment owes the rules: it took exactly the cost's
/// pips, took nothing that was not there, and took each type's own pips
/// from that type.
fn assert_paid_exactly(before: &Amounts, after: &Amounts, cost: &Pips) {
    for i in 0..6 {
        assert!(after[i] <= before[i], "a payment never adds mana");
        assert!(before[i] - after[i] >= cost.specific[i], "each pip is paid with its own type");
    }
    assert_eq!(
        total(before) - total(after),
        total(&cost.specific) + cost.generic,
        "a payment spends every pip, no more and no fewer",
    );
}

fn any_amount() -> u32 {
    let n: u32 = kani::any();
    kani::assume(n <= MAX_MANA);
    n
}

fn any_amounts() -> Amounts {
    std::array::from_fn(|_| any_amount())
}

fn any_pips() -> Pips {
    Pips { specific: any_amounts(), generic: any_amount() }
}

fn any_color() -> Color {
    match kani::any::<u8>() % 5 {
        0 => Color::White,
        1 => Color::Blue,
        2 => Color::Black,
        3 => Color::Red,
        _ => Color::Green,
    }
}

fn any_symbol() -> ManaSymbol {
    match kani::any::<u8>() % 4 {
        0 => ManaSymbol::Colored(any_color()),
        1 => ManaSymbol::Generic(any_amount()),
        2 => ManaSymbol::Colorless(any_amount()),
        _ => ManaSymbol::X,
    }
}

fn any_cost() -> ManaCost {
    let len: usize = kani::any();
    kani::assume(len <= MAX_SYMBOLS);
    let mut symbols = Vec::with_capacity(MAX_SYMBOLS);
    for i in 0..MAX_SYMBOLS {
        if i < len {
            symbols.push(any_symbol());
        }
    }
    ManaCost::new(symbols)
}

/// The six slots are the six types, each once.
#[kani::proof]
#[kani::solver(cadical)]
fn each_type_has_its_own_slot() {
    let a = TYPES[usize::from(kani::any::<u8>() % 6)];
    let b = TYPES[usize::from(kani::any::<u8>() % 6)];
    assert!(slot_of(a) < 6);
    assert_eq!(slot_of(a) == slot_of(b), a == b);
}

/// `pips_of` counts, for each type, the pips only that type can pay, and
/// the generic mana; and together they are the cost's mana value, so
/// spending every pip is spending the mana value.
#[kani::proof]
#[kani::solver(cadical)]
#[kani::unwind(6)]
fn counting_a_cost_counts_every_pip() {
    let cost = any_cost();
    let pips = pips_of(&cost);
    let t = TYPES[usize::from(kani::any::<u8>() % 6)];
    let only_t: u32 = cost.symbols.iter().map(|s| match s {
        ManaSymbol::Colored(c) if ManaType::from(*c) == t => 1,
        ManaSymbol::Colorless(n) if t == ManaType::Colorless => *n,
        _ => 0,
    }).sum();
    assert_eq!(pips.specific[slot_of(t)], only_t);
    assert_eq!(pips.generic, cost.generic_amount());
    assert_eq!(total(&pips.specific) + pips.generic, cost.mana_value());
}

/// `can_pay` and `auto_pay` (which are `pay_fixed`) succeed exactly when the
/// rules say the pool can pay, and a success spends the cost and nothing
/// else.
#[kani::proof]
#[kani::solver(cadical)]
fn paying_succeeds_exactly_when_the_rules_allow_and_spends_the_cost() {
    let have = any_amounts();
    let cost = any_pips();
    let left = pay_fixed(have, &cost);
    assert_eq!(left.is_some(), spec_payable(&have, &cost));
    if let Some(left) = left {
        assert_paid_exactly(&have, &left, &cost);
    }
}

/// The same for `auto_pay_reserving` (`pay_around`): reserving changes which
/// mana pays, never whether the cost is paid or how much it takes.
#[kani::proof]
#[kani::solver(cadical)]
fn reserving_succeeds_exactly_when_the_rules_allow_and_spends_the_cost() {
    let have = any_amounts();
    let cost = any_pips();
    let keep = any_amounts();
    let left = pay_around(have, &cost, &keep);
    assert_eq!(left.is_some(), spec_payable(&have, &cost));
    if let Some(left) = left {
        assert_paid_exactly(&have, &left, &cost);
    }
}

/// What `auto_pay_reserving` is for (issues #252, #678, #840): when the pool
/// can pay the cost and the reserve together, paying the cost leaves the
/// reserve payable. The reserve's generic mana plays no part in which mana
/// is spent, but it is part of what has to stay payable.
#[kani::proof]
#[kani::solver(cadical)]
fn reserving_leaves_the_reserve_payable() {
    let have = any_amounts();
    let cost = any_pips();
    let reserve = any_pips();
    kani::assume(spec_payable(&have, &both(&cost, &reserve)));
    let left = pay_around(have, &cost, &reserve.specific).expect("the cost alone is payable");
    assert!(spec_payable(&left, &reserve), "the reserve is still payable");
}

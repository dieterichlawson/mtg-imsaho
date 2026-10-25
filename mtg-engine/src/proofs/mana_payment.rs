//! Turning a mana cost into what payment works on: `mana::slot_of` and
//! `mana::pips_of`.
//!
//! Paying is proved in `mtg-kernel` with Verus, for every pool, cost and
//! reserve, over six per-type counts and a cost as [`Pips`]. Verus sees only
//! that crate, so the step from the engine's `ManaCost` to `Pips` is proved
//! here: each type has its own slot, and `pips_of` counts, for each type,
//! the pips only that type can pay (a coloured pip, `{C}`), the generic
//! mana on top, and nothing else; `{X}` is zero when it is not being
//! announced (CR 107.3e). Together the counts are the mana value, so
//! spending every pip is spending the mana value.
//!
//! Bounds: costs of up to [`MAX_SYMBOLS`] symbols, which is every printed
//! cost, and numbers in a symbol of up to [`MAX_AMOUNT`]. A reserve built
//! from a whole hand's pips can be longer; it is the same fold over more
//! symbols, but it is outside this proof. Reading the pool into six counts
//! and writing the result back goes through `ManaPool`'s `BTreeMap`, which
//! CBMC does not finish on (six inserts ran past five minutes); the unit
//! tests in `mana.rs` cover those few lines.

use crate::mana::{pips_of, slot_of, Amounts, Pips};
use crate::types::{Color, ManaCost, ManaSymbol, ManaType};

/// The longest printed cost in the card pool is four symbols (Army of the
/// Damned's `{5}{B}{B}{B}`).
const MAX_SYMBOLS: usize = 4;
/// The most a `{N}` or `{C}` symbol says in this proof.
const MAX_AMOUNT: u32 = 3;

const TYPES: [ManaType; 6] = [
    ManaType::White, ManaType::Blue, ManaType::Black,
    ManaType::Red, ManaType::Green, ManaType::Colorless,
];

fn total(a: &Amounts) -> u32 {
    a.iter().sum()
}

fn any_amount() -> u32 {
    let n: u32 = kani::any();
    kani::assume(n <= MAX_AMOUNT);
    n
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
/// the generic mana; and together they are the cost's mana value.
#[kani::proof]
#[kani::solver(cadical)]
#[kani::unwind(6)]
fn counting_a_cost_counts_every_pip() {
    let cost = any_cost();
    let pips: Pips = pips_of(&cost);
    let t = TYPES[usize::from(kani::any::<u8>() % 6)];
    let only_t: u32 = cost.symbols.iter().map(|s| match s {
        ManaSymbol::Colored(c) if ManaType::from(*c) == t => 1,
        ManaSymbol::Colorless(n) if t == ManaType::Colorless => *n,
        _ => 0,
    }).sum();
    kani::cover!(only_t > 1, "a type asked for more than once");
    kani::cover!(pips.generic > 0 && only_t > 0, "generic beside a type's own pips");
    assert_eq!(pips.specific[slot_of(t)], only_t);
    assert_eq!(pips.generic, cost.generic_amount());
    assert_eq!(total(&pips.specific) + pips.generic, cost.mana_value());
}

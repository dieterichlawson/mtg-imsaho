//! A mana cost as it is built (`types::ManaCost::new`), every cost a card
//! prints or a reduction leaves passing through it.

use crate::types::{Color, ManaCost, ManaSymbol};

/// Any symbol of the kinds a cost holds: generic of 0 to 2, coloured
/// (white or red), colourless of 0 or 1, or X.
fn any_symbol() -> ManaSymbol {
    match kani::any::<u8>() % 5 {
        0 => ManaSymbol::Generic(u32::from(kani::any::<u8>() % 3)),
        1 => ManaSymbol::Colored(Color::White),
        2 => ManaSymbol::Colored(Color::Red),
        3 => ManaSymbol::Colorless(u32::from(kani::any::<bool>())),
        _ => ManaSymbol::X,
    }
}

/// A cost of `N` symbols, each any: a `{0}` beside any other symbol is
/// dropped, and every other symbol kept, in order; a cost of nothing but
/// `{0}`s is kept as it is, which is how a card of no cost prints (#978,
/// a fully reduced Blasphemous Act read "{0}{R}"). What it asks for is
/// the same either way: its mana value, and each symbol but a `{0}`. Of a
/// length fixed in each harness: of one the solver chose, the list was
/// copied and filtered at every length, and the solver ran out of memory.
fn building<const N: usize>() {
    let symbols: [ManaSymbol; N] = std::array::from_fn(|_| any_symbol());
    let zero = |s: &ManaSymbol| matches!(s, ManaSymbol::Generic(0));
    let kept = symbols.iter().filter(|s| !zero(s)).count();
    let value: u32 = symbols.iter().map(|s| match s {
        ManaSymbol::Colored(_) => 1,
        ManaSymbol::Generic(k) | ManaSymbol::Colorless(k) => *k,
        ManaSymbol::X => 0,
    }).sum();

    let cost = ManaCost::new(symbols.to_vec());

    if kept == 0 {
        assert!(cost.symbols.len() == N && (0..N).all(|i| i >= cost.symbols.len() || cost.symbols[i] == symbols[i]));
    } else {
        assert!(cost.symbols.len() == kept);
        // The kth symbol kept is the kth of the given that is not a {0}.
        let mut k = 0;
        for s in &symbols {
            if !zero(s) {
                assert!(k < cost.symbols.len() && cost.symbols[k] == *s);
                k += 1;
            }
        }
    }
    assert!(cost.mana_value() == value);
}

/// The proofs of a cost built, one per length.
macro_rules! each_length_built {
    ($($name:ident: $n:expr;)*) => {$(
        #[kani::proof]
        #[kani::unwind(5)]
        /// [`building`] for a cost of the length of its name.
        fn $name() {
            building::<$n>();
        }
    )*};
}

each_length_built! {
    a_cost_of_no_symbols_is_built_as_none: 0;
    a_cost_of_one_symbol_is_built_as_it: 1;
    a_cost_of_two_symbols_drops_a_zero_beside_the_other: 2;
    a_cost_of_three_symbols_drops_each_zero_beside_another: 3;
}

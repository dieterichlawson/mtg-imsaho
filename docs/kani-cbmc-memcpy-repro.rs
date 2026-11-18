//! A clone that Kani 0.68 (CBMC 6.11) reads back wrong: the reproduction
//! behind "a known imprecision in CBMC 6.11's memcpy" in
//! docs/plans/formal-verification.md ("What is trusted").
//!
//! As `src/lib.rs` of a new crate (`cargo new --lib`, edition 2021), run
//! `cargo kani`. Expected: `cloned_list_reads_back` and
//! `the_clone_holds_four` verify. Seen: `cloned_list_reads_back` fails
//! and `the_clone_holds_four` fails as it must, so the clone's second
//! element is left open rather than wrong (an over-approximation).
//! `built_by_a_helper` is the same clone with the list built in another
//! function, and verifies.
//!
//! What happens: the clone's `memcpy` reaches the list's buffer through a
//! pointer CBMC's symbolic execution cannot pin to one object, and for
//! the heap buffer among its candidates it copies
//! `byte_extract(buffer[0], 0, char[n])`, n bytes out of the first one.

#[derive(Clone, Debug)]
pub enum Inner {
    Wide { description: String, a: u64, b: u64, c: u64, d: u64, e: u64, f: u64, g: u64, h: u64, i: u64, j: u64, k: u64, l: u64, m: u64 },
    Narrow { description: String, looked_at: Vec<u64> },
    Small { x: u8 },
}

#[derive(Clone, Debug)]
pub enum Outer {
    Blockers { p: u8 },
    Choice { player: u8, source: u64, choice: Inner },
}

#[cfg(kani)]
mod proofs {
    use super::*;

    fn three_four() -> Vec<u64> {
        let mut v = Vec::with_capacity(4);
        v.push(3);
        v.push(4);
        v
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn cloned_list_reads_back() {
        let mut v = Vec::with_capacity(4);
        v.push(3u64);
        v.push(4u64);
        let a = Outer::Choice { player: 0, source: 1, choice: Inner::Narrow { description: "d".into(), looked_at: v } };
        let c = a.clone();
        if let Outer::Choice { choice: Inner::Narrow { looked_at, .. }, .. } = c {
            assert!(looked_at.len() == 2 && looked_at[1] == 4);
        }
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn built_by_a_helper() {
        let a = Outer::Choice { player: 0, source: 1, choice: Inner::Narrow { description: "d".into(), looked_at: three_four() } };
        let c = a.clone();
        if let Outer::Choice { choice: Inner::Narrow { looked_at, .. }, .. } = c {
            assert!(looked_at.len() == 2 && looked_at[1] == 4);
        }
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn the_clone_holds_four() {
        // Must fail: the clone's second element is 4.
        let mut v = Vec::with_capacity(4);
        v.push(3u64);
        v.push(4u64);
        let a = Outer::Choice { player: 0, source: 1, choice: Inner::Narrow { description: "d".into(), looked_at: v } };
        let c = a.clone();
        if let Outer::Choice { choice: Inner::Narrow { looked_at, .. }, .. } = &c {
            if looked_at.len() == 2 {
                assert!(looked_at[1] != 4);
            }
        }
    }
}

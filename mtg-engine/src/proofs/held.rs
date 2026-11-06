//! A proof's own state, kept where no constant is read from it.
//!
//! A harness and the stubs it sets talk through `static mut`s: what each
//! stub was asked and what it answered. Kani 0.68 lays out a static's
//! initial value through the same table it shares equal constants through:
//! `codegen_static` calls `codegen_alloc_in_memory`, which enters the
//! allocation under the static's own symbol, and a constant laid out after
//! it with the same bytes, alignment and provenance is then read from the
//! static. The standard library's `ZERO_CAP`, the capacity of every
//! `Vec::new()`, is eight zero bytes, as was the combat damage proofs'
//! counter `MARKED_N: usize = 0`; once a stub had counted one creature
//! dealing first-strike damage, every list made after had a capacity of 1
//! and no buffer, and the first push wrote through a dangling pointer. A
//! constant read so is whatever a proof last stored, which is no longer the
//! program Rust compiled, in either direction.
//!
//! So every such static is a [`Held`]: its value beside its own address.
//! An allocation's provenance is part of what Kani compares, and no
//! constant holds a pointer to a static of the proofs, so no constant has a
//! `Held`'s bytes. `scripts/static-constants.py`, which
//! `scripts/kani-bounded.py` runs on every harness's goto program before
//! CBMC, checks that no constant is read through any static.

/// A static of the proofs: declared `static mut NAME: Held<T> =
/// Held::new(value, (&raw const NAME).cast())`, and read and written
/// through `addr_of!(NAME.value)` and `addr_of_mut!(NAME.value)`.
///
/// The value first (`repr(C)`), where it was before it was held. Laid out
/// as Rust lays it out, the address first and the value eight bytes in, an
/// `Option<bool>` written into the offer proofs' answers read back as none
/// the instruction after (`get_or_insert_with`'s `unwrap_unchecked`
/// reached), and both proofs of who may attack and block failed.
#[repr(C)]
pub(crate) struct Held<T> {
    pub(crate) value: T,
    /// The static's own address, read by nothing: what makes its bytes its
    /// own.
    _own: *const u8,
}

impl<T> Held<T> {
    pub(crate) const fn new(value: T, own: *const u8) -> Self {
        Held { value, _own: own }
    }
}

#!/usr/bin/env python3
"""static-constants.py BINARY: list every constant a Kani goto binary reads
through a static's memory, and every pointer outside the proofs' own code
into a static of the proofs. Exit status 1 if there is any.

Kani 0.68 keeps one table of the allocations it has laid out, keyed by
their bytes, alignment and provenance, so that two equal constants share
one global (`codegen_const_allocation`). A static's initial value is laid
out through the same function and enters the same table under the
static's own symbol (`codegen_static` calls `codegen_alloc_in_memory`).
A constant laid out after it with the same bytes is then read from the
static: the standard library's `ZERO_CAP`, the capacity of every
`Vec::new()`, is eight zero bytes, as is a counter `static mut N: usize =
0`. Once the proof set the counter to 1, every new list in it had a
capacity of 1 and no buffer, and the first push wrote through a dangling
pointer. That is a different program from the one Rust compiled, in
either direction: a constant may read as whatever a proof stored.

A constant laid out in memory is read as `*(T *)(unsigned char *)&g`,
with `g` a Kani global named `<crate>::global::<n>::`; through a static
the same expression names the static. A pointer constant into such an
allocation is `(T *)((unsigned char *)&g + k)`, the same expression a
program's own `&raw const S` makes, so it is told from one only by where
it is: a static of the proofs is named by the proofs' code alone (their
harnesses and the stubs standing in for engine functions, whose code is
in `mtg-engine/src/proofs/`), and a pointer to one anywhere else is a
constant laid out over it.

`mtg-engine/src/proofs/held.rs` makes every static of the proofs one no
constant can share; this is the check that none does, and that no other
static is read so either.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import goto_binary as gb  # noqa: E402


def scan(strings, symbols, functions):
    rev = {v: k for k, v in strings.items()}

    def sid(s):
        return rev.get(s.encode())

    DEREF, TYPECAST, ADDRESS_OF, SYMBOL = sid('dereference'), sid('typecast'), sid('address_of'), sid('symbol')
    K_IDENT, K_TYPE, K_WIDTH, K_FILE = sid('identifier'), sid('type'), sid('width'), sid('file')
    POINTER, UNSIGNEDBV = sid('pointer'), sid('unsignedbv')

    def named(n, k):
        for kk, v in n.named:
            if kk == k:
                return v
        return None

    def text(n):
        return strings[n.id].decode(errors='replace') if n is not None else ''

    def ident(n):
        return text(named(n, K_IDENT)) if n.id == SYMBOL else None

    def is_byte_pointer(t):
        if t is None or t.id != POINTER or not t.subs:
            return False
        w = named(t.subs[0], K_WIDTH)
        return t.subs[0].id == UNSIGNEDBV and w is not None and text(w) == '8'

    # Statics and Kani's constant globals: lvalues of static lifetime
    # (`write_goto_binary`'s flags, bits 5 and 4). Kani's are named
    # `<crate>::global::<n>::`.
    pretty, statics = {}, set()
    for t, v, loc, names, ordering, flags in symbols:
        name = strings[names[0]].decode(errors='replace')
        pretty[name] = strings[names[4]].decode(errors='replace')
        if flags & 0b110000 == 0b110000 and '::global::' not in name:
            statics.add(name)

    reads, pointers = [], []
    for fname, instrs in functions:
        fn = fname.decode(errors='replace')
        # The proofs' own code: a function any of whose instructions is
        # written in `mtg-engine/src/proofs/` (a stub keeps the name of the
        # function it stands in for, and `addr_of!` is placed where the
        # macro is defined, in `core::ptr`).
        files = {text(named(loc, K_FILE)) for _, loc, *_ in instrs if loc is not None}
        in_proofs = any('mtg-engine/src/proofs/' in f for f in files)
        for code, loc, *_ in instrs:
            stack, seen = [code], set()
            while stack:
                n = stack.pop()
                if id(n) in seen:
                    continue
                seen.add(id(n))
                # *(T *)(unsigned char *)&S
                if n.id == DEREF and n.subs and n.subs[0].id == TYPECAST and n.subs[0].subs:
                    inner = n.subs[0].subs[0]
                    if (inner.id == TYPECAST and is_byte_pointer(named(inner, K_TYPE)) and inner.subs
                            and inner.subs[0].id == ADDRESS_OF and inner.subs[0].subs
                            and inner.subs[0].subs[0].id == SYMBOL):
                        s = ident(inner.subs[0].subs[0])
                        if s in statics:
                            reads.append((fn, s))
                if n.id == ADDRESS_OF and n.subs and n.subs[0].id == SYMBOL and not in_proofs:
                    s = ident(n.subs[0])
                    if s in statics and '6proofs' in s:
                        pointers.append((fn, s))
                stack.extend(n.subs)
                stack.extend(v for _, v in n.named)
    return reads, pointers, pretty


def main(path):
    version, strings, symbols, functions = gb.read(path)
    reads, pointers, pretty = scan(strings, symbols, functions)
    for kind, found in (('constant read through', reads), ('pointer outside the proofs into', pointers)):
        for fn, s in sorted(set(found)):
            print(f'{kind} {pretty.get(s) or s} in {pretty.get(fn) or fn}')
    print(f'{len(set(reads))} constants read through a static, '
          f'{len(set(pointers))} pointers outside the proofs into a static of theirs')
    return 1 if reads or pointers else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1]))

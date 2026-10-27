#!/usr/bin/env python3
"""precise-enums.py IN OUT: rewrite a Kani goto binary so that CBMC knows
the value of every enum the program builds from known parts.

Kani encodes an enum as `{ case, cases }`, the tag and a union of one
struct per variant, and builds a variant in two ways CBMC cannot follow:

- the variant's struct fills its leading padding (the bytes the tag sits
  in) with a nondeterministic value, and
- it is assigned as one member of the union, `x.cases.V := v`, which CBMC
  applies to the union's bytes as they were: unknown, wherever V is
  narrower than the union.

CBMC propagates a value only when it knows all of it, so neither value is
propagated. A variant copied to the heap through a parameter
(`Box::new(x)`, `Vec::push(x)`) then reaches memory as bytes CBMC cannot
trace back to the pointers inside it, and a pointer read back from there
dereferences to an unconstrained object. The answer is still right (the
formula is exact wherever CBMC knew the pointer's target), but every
branch on what the unconstrained object holds is explored, and a recursive
function over a type that holds itself (the engine's filters, effects and
target requirements, and Rust's clone, drop and comparison of them)
explores every variant at every level: a clone of a three-level filter ran
past five minutes and ten gigabytes, and runs in a second rewritten.

The rewrite, which changes nothing a Rust program can observe:

- every struct expression filling a padding component with `nondet`
  fills it with zero instead (safe Rust never reads padding: derived `==`
  and `Hash` go field by field, and the engine has no `unsafe`);
- every assignment `x.cases.V := v` of an enum with its own tag field
  becomes `x.cases := bytes of {v, 0...}`, so the bytes of other variants
  are zero rather than whatever they were. No program reads a variant other
  than the one its tag names. A niche-encoded enum keeps its tag inside a
  variant's bytes and is left alone, as is a Rust `union`, whose other
  bytes are kept by a write.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import goto_binary as gb  # noqa: E402

ASSIGN, DECL = 13, 14


def rewrite(version, strings, symbols, functions):
    rev = {v: k for k, v in strings.items()}

    def sid(s):
        b = s.encode()
        if b not in rev:
            i = max(strings) + 1
            strings[i] = b
            rev[b] = i
        return rev[b]

    def leaf(s):
        return gb.Irep(sid(s), [], [])

    K_TYPE, K_STATEMENT, K_COMPONENTS, K_IDENT = sid('type'), sid('statement'), sid('components'), sid('identifier')
    K_PADDING, K_VALUE, K_WIDTH, K_SIZE = sid('#is_padding'), sid('value'), sid('width'), sid('size')
    K_NAME, K_COMPONENT_NAME, K_BITS_PER_BYTE = sid('name'), sid('component_name'), sid('bits_per_byte')
    STRUCT, STRUCT_TAG, UNION, UNION_TAG = sid('struct'), sid('struct_tag'), sid('union'), sid('union_tag')
    SIDE_EFFECT, NONDET, CONSTANT, MEMBER, EMPTY = sid('side_effect'), sid('nondet'), sid('constant'), sid('member'), sid('')
    ARRAY, POINTER, BOOL, UNSIGNEDBV = sid('array'), sid('pointer'), sid('bool'), sid('unsignedbv')
    BYTE_EXTRACT = sid('byte_extract_little_endian')
    BVS = {sid(t) for t in ('signedbv', 'unsignedbv', 'c_bool', 'floatbv', 'bv')}

    def named(n, k):
        for key, v in n.named:
            if key == k:
                return v
        return None

    def zero(t):
        return gb.Irep(CONSTANT, [], [(K_TYPE, t), (K_VALUE, leaf('0'))])

    types = {sym[3][0]: sym[0] for sym in symbols if sym[5] & (1 << 15)}

    def resolve(t):
        if t is not None and t.id in (STRUCT_TAG, UNION_TAG):
            ident = named(t, K_IDENT)
            return types.get(ident.id) if ident is not None else None
        return t

    widths = {}

    def width(t):
        """The width of `t` in bits, or None."""
        if id(t) in widths:
            return widths[id(t)]
        widths[id(t)] = None
        r, w = resolve(t), None
        if r is None:
            pass
        elif r.id in BVS or r.id == POINTER:
            n = named(r, K_WIDTH)
            w = int(strings[n.id]) if n is not None else None
        elif r.id == BOOL:
            w = 1
        elif r.id in (STRUCT, UNION):
            comps = named(r, K_COMPONENTS)
            ws = [width(named(c, K_TYPE)) for c in comps.subs] if comps is not None else [None]
            if all(x is not None for x in ws):
                w = sum(ws) if r.id == STRUCT else max(ws, default=0)
        elif r.id == ARRAY and r.subs:
            n = named(r, K_SIZE)
            v = named(n, K_VALUE) if n is not None and n.id == CONSTANT else None
            e = width(r.subs[0])
            if v is not None and e is not None:
                w = int(strings[v.id], 16) * e
        widths[id(t)] = w
        return w

    # Padding filled with zero rather than nondet.
    padding, seen = 0, set()
    stack = [x for sym in symbols for x in sym[:3]] + [x for _, ins in functions for i in ins for x in (i[0], i[3])]
    while stack:
        n = stack.pop()
        if id(n) in seen:
            continue
        seen.add(id(n))
        if n.id == STRUCT and n.subs:
            t = resolve(named(n, K_TYPE))
            comps = named(t, K_COMPONENTS) if t is not None and t.id == STRUCT else None
            if comps is not None and len(comps.subs) == len(n.subs):
                for i, (op, c) in enumerate(zip(n.subs, comps.subs)):
                    pad, st = named(c, K_PADDING), named(op, K_STATEMENT)
                    if (pad is not None and strings[pad.id] == b'1' and op.id == SIDE_EFFECT
                            and st is not None and st.id == NONDET):
                        n.subs[i] = zero(named(c, K_TYPE))
                        padding += 1
        stack.extend(n.subs)
        stack.extend(v for _, v in n.named)

    # Variant writes made writes of the whole union.
    offset0 = zero(gb.Irep(sid('signedbv'), [], [(K_WIDTH, leaf('64'))]))
    variants = 0
    for _, instrs in functions:
        for ins in instrs:
            code = ins[0]
            if ins[2] != ASSIGN or len(code.subs) != 2:
                continue
            lhs, rhs = code.subs
            if lhs.id != MEMBER or not lhs.subs:
                continue
            cases = lhs.subs[0]
            ut = named(cases, K_TYPE)
            if ut is None or ut.id != UNION_TAG or cases.id != MEMBER or not cases.subs:
                continue
            cn = named(cases, K_COMPONENT_NAME)
            outer = resolve(named(cases.subs[0], K_TYPE))
            ocomps = named(outer, K_COMPONENTS) if outer is not None and outer.id == STRUCT else None
            if (cn is None or strings[cn.id] != b'cases' or ocomps is None
                    or not any(strings[named(c, K_NAME).id] == b'case' for c in ocomps.subs)):
                continue
            vt = named(rhs, K_TYPE)
            uw, vw = width(ut), (width(vt) if vt is not None else None)
            if uw is None or vw is None or vw > uw:
                continue
            comps, ops = [gb.Irep(EMPTY, [], [(K_NAME, leaf('v')), (K_TYPE, vt)])], [rhs]
            if uw > vw:
                tail = gb.Irep(UNSIGNEDBV, [], [(K_WIDTH, leaf(str(uw - vw)))])
                comps.append(gb.Irep(EMPTY, [], [(K_NAME, leaf('tail')), (K_TYPE, tail)]))
                ops.append(zero(tail))
            bytes_of = gb.Irep(STRUCT, ops, [(K_TYPE, gb.Irep(STRUCT, [], [(K_COMPONENTS, gb.Irep(EMPTY, comps, []))]))])
            whole = gb.Irep(BYTE_EXTRACT, [bytes_of, offset0], [(K_TYPE, ut), (K_BITS_PER_BYTE, leaf('8'))])
            ins[0] = gb.Irep(code.id, [cases, whole], list(code.named))
            variants += 1
    return padding, variants


def main(src, dst):
    model = gb.read(src)
    padding, variants = rewrite(*model)
    gb.write(dst, *model)
    print(f'{padding} padding values made zero, {variants} variant writes made whole')


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])

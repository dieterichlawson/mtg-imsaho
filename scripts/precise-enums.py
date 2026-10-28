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
  becomes `x.cases := the union of member V, v`, a write of the whole
  union rather than of one member over its bytes as they were (which CBMC
  keeps as an update of those bytes, and does not simplify a read of), and
  one that a read of member V gives back exactly, where the union as bytes
  would be kept as its widest member. No program reads a variant other
  than the one its tag names. A fieldless variant, only its tag written,
  has its variants' bytes made zero, so that CBMC knows all of it and
  propagates it. A niche-encoded enum keeps its tag inside a
  variant's bytes and is left alone, as is a Rust `union`, whose other
  bytes are kept by a write. A niche-encoded enum's variant (its tag kept
  in a field of another variant's bytes) is written as its fields, if it
  has any, and then its tag into the niche: both become one write of the
  whole value, the bytes outside the variant zero, the tag in the
  variant's leading padding (where the niche is), when the variable was
  fresh (declared just before) or the write of its fields came just
  before; and the variant that holds the niche, written with no tag
  after it, is written as the whole value too;
- each enum's widest variant is put first in its union. CBMC keeps a
  union's value as its widest member and reads one through a pointer
  through the first member with a field where the bytes are, and a read
  through a member other than the one kept is simplified only when the
  whole value converts to bits, which a value holding a pointer does not:
  a niche-encoded enum's tag, read so, was unknown, and every `match` on
  it followed every arm. A union's members all start at its first byte,
  so their order is not the program's. A variant other than the widest
  is then still read through another member than the one kept, so the
  engine's own enums that would be niche-encoded and hold pointers
  (`PendingEffect`, `TargetFilter`, `GameEvent` and the rest marked
  `#[cfg_attr(kani, repr(u8))]`) keep their tag in a field of its own in
  a Kani build, and each variant is written as the union of its own
  member (below), read back by that member's name;
- every heap allocation (CBMC's `malloc`, which Kani's `__rust_alloc`
  calls) is zeroed, as `calloc`'s is. Rust's allocator hands out memory of
  no particular value, and safe Rust never reads memory before writing it
  (that is undefined behaviour), so zero is a value it may have and no
  program tells. CBMC's own value is one it leaves open, and a list whose
  length the solver leaves open is read past its end on paths it cannot
  rule out: an enum read from there is of every variant, and dropping one
  of the stack's entries went into every variant of a target requirement,
  three levels deep. Zeroed, what is read there is one known variant.
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
            # The union as the member written: read by that member's name,
            # as code reads the variant its tag names, it is the value
            # written. As bytes, CBMC keeps it as its widest member, and a
            # read through any other member is simplified only when the
            # whole value converts to bits, which it does not when it holds
            # a pointer or bytes never written (a fieldless variant's).
            whole = gb.Irep(UNION, [rhs], [(K_TYPE, ut), (K_COMPONENT_NAME, named(lhs, K_COMPONENT_NAME))])
            ins[0] = gb.Irep(code.id, [cases, whole], list(code.named))
            variants += 1

    # A niche-encoded enum's variant: `x.V := v` over a union of no known
    # bytes, then its tag written into the niche, `*(T *)&x := tag`. Made a
    # write of the whole union, the bytes outside the variant zero.
    DEREF, TYPECAST, ADDRESS_OF, SYMBOL = sid('dereference'), sid('typecast'), sid('address_of'), sid('symbol')

    # A tagged enum's fieldless variant: a fresh local, `DECL x`, then only
    # its tag written, `x.case := tag`, its variants' bytes never. Made a
    # write of the whole value, those bytes zero: CBMC propagates a value
    # only when it knows all of it, and one holding this one (a
    # `ContinuousEffect` over `EffectScope::Attached`) was not propagated
    # into a `Box`, where cloning it then went into every scope's filter.
    fieldless = 0
    for _, instrs in functions:
        for i, ins in enumerate(instrs[1:], 1):
            code, prev = ins[0], instrs[i - 1]
            if ins[2] != ASSIGN or len(code.subs) != 2 or prev[2] != DECL or not prev[0].subs:
                continue
            lhs, tag = code.subs
            if lhs.id != MEMBER or not lhs.subs or lhs.subs[0].id != SYMBOL:
                continue
            x, cn = lhs.subs[0], named(lhs, K_COMPONENT_NAME)
            declared, xid = prev[0].subs[0], named(lhs.subs[0], K_IDENT)
            if (cn is None or strings[cn.id] != b'case' or declared.id != SYMBOL or xid is None
                    or named(declared, K_IDENT) is None or named(declared, K_IDENT).id != xid.id):
                continue
            st = resolve(named(x, K_TYPE))
            comps = named(st, K_COMPONENTS) if st is not None and st.id == STRUCT else None
            if comps is None:
                continue
            ops = []
            for c in comps.subs:
                name, ct = strings[named(c, K_NAME).id], named(c, K_TYPE)
                w = width(ct)
                if name == b'case':
                    ops.append(tag)
                elif w is None:
                    break
                elif ct.id == UNION_TAG:
                    bits = gb.Irep(UNSIGNEDBV, [], [(K_WIDTH, leaf(str(w)))])
                    ops.append(gb.Irep(BYTE_EXTRACT, [zero(bits), offset0], [(K_TYPE, ct), (K_BITS_PER_BYTE, leaf('8'))]))
                else:
                    ops.append(zero(ct))
            if len(ops) != len(comps.subs):
                continue
            ins[0] = gb.Irep(code.id, [x, gb.Irep(STRUCT, ops, [(K_TYPE, named(x, K_TYPE))])], list(code.named))
            fieldless += 1


    def niche_write_to(code, var):
        if len(code.subs) != 2:
            return False
        lhs = code.subs[0]
        while lhs.id in (DEREF, TYPECAST) and lhs.subs:
            lhs = lhs.subs[0]
        return lhs.id == ADDRESS_OF and lhs.subs and lhs.subs[0].id == SYMBOL and named(lhs.subs[0], K_IDENT) is not None \
            and named(lhs.subs[0], K_IDENT).id == var

    def is_enum_union(ut):
        """Whether union type `ut` is an enum's (each member a variant, a
        struct named for the enum): a niche-encoded enum's, or the union
        of a tagged enum's variants beside its tag. Not a Rust `union`."""
        u, ident = resolve(ut), named(ut, K_IDENT)
        comps = named(u, K_COMPONENTS) if u is not None and u.id == UNION else None
        if ident is None or comps is None or not comps.subs:
            return False
        name = strings[ident.id]
        prefix = (name[:-len(b'-union')] if name.endswith(b'-union') else name) + b'::'
        for c in comps.subs:
            ct = named(c, K_TYPE)
            ci = named(ct, K_IDENT) if ct is not None and ct.id == STRUCT_TAG else None
            if ci is None or not strings[ci.id].startswith(prefix):
                return False
        return True

    # Each enum's widest variant first. Symex splits a union into its
    # members and keeps its value as the widest member (the first of the
    # widest: CBMC turns the bytes of a whole write into that member), but
    # reads through a pointer, or by bytes, through the first member with a
    # field where the bytes are; a member read other than the one kept is
    # simplified only by turning the whole value into bits, which CBMC
    # cannot do when it holds a pointer. The tag of a niche-encoded enum
    # was read through another member than the one kept, and every `match`
    # on it followed every arm. Members of a union all start at its first
    # byte, so their order is not the program's.
    reordered = 0
    for ident, ty in types.items():
        comps = named(ty, K_COMPONENTS) if ty.id == UNION else None
        if comps is None or len(comps.subs) < 2 or not is_enum_union(gb.Irep(UNION_TAG, [], [(K_IDENT, gb.Irep(ident, [], []))])):
            continue
        ws = [width(named(c, K_TYPE)) for c in comps.subs]
        if any(w is None for w in ws):
            continue
        widest = ws.index(max(ws))
        if widest:
            comps.subs.insert(0, comps.subs.pop(widest))
            reordered += 1

    niches, tags, dataful = 0, 0, 0
    for _, instrs in functions:
        for i, ins in enumerate(instrs[:-1]):
            code = ins[0]
            if ins[2] != ASSIGN or len(code.subs) != 2:
                continue
            lhs, rhs = code.subs
            if lhs.id != MEMBER or not lhs.subs or lhs.subs[0].id != SYMBOL:
                continue
            union = lhs.subs[0]
            ut, var = named(union, K_TYPE), named(union, K_IDENT)
            if ut is None or ut.id != UNION_TAG or var is None:
                continue
            tagged = instrs[i + 1][2] == ASSIGN and niche_write_to(instrs[i + 1][0], var.id)
            if not tagged and not is_enum_union(ut):
                continue
            vt = named(rhs, K_TYPE)
            uw, vw = width(ut), (width(vt) if vt is not None else None)
            if uw is None or vw is None or vw > uw:
                continue
            comps, ops = [gb.Irep(EMPTY, [], [(K_NAME, leaf('v')), (K_TYPE, vt)])], [rhs]
            if tagged:
                # The tag goes where the niche is, the variant's leading
                # padding, and its write of its own goes: the value is then
                # one whose tag symex reads back (with the widest variant
                # first, above).
                nxt = instrs[i + 1][0]
                tag = nxt.subs[1]
                vr = resolve(vt)
                vcomps = named(vr, K_COMPONENTS) if vr is not None and vr.id == STRUCT else None
                if rhs.id == STRUCT and rhs.subs and vcomps is not None and len(vcomps.subs) == len(rhs.subs) and tag.id == CONSTANT:
                    c0, tt = vcomps.subs[0], named(tag, K_TYPE)
                    pad, ct = named(c0, K_PADDING), named(c0, K_TYPE)
                    if pad is not None and strings[pad.id] == b'1' and tt is not None and width(ct) is not None and width(ct) == width(tt):
                        first = tag if tt.id == ct.id else gb.Irep(TYPECAST, [tag], [(K_TYPE, ct)])
                        ops = [gb.Irep(rhs.id, [first] + rhs.subs[1:], list(rhs.named))]
                        instrs[i + 1][0] = gb.Irep(nxt.id, [union, union], list(nxt.named))
                        tags += 1
            else:
                # The variant that holds the niche, whose own bytes are its tag.
                dataful += 1
            if uw > vw:
                tail = gb.Irep(UNSIGNEDBV, [], [(K_WIDTH, leaf(str(uw - vw)))])
                comps.append(gb.Irep(EMPTY, [], [(K_NAME, leaf('tail')), (K_TYPE, tail)]))
                ops.append(zero(tail))
            bytes_of = gb.Irep(STRUCT, ops, [(K_TYPE, gb.Irep(STRUCT, [], [(K_COMPONENTS, gb.Irep(EMPTY, comps, []))]))])
            whole = gb.Irep(BYTE_EXTRACT, [bytes_of, offset0], [(K_TYPE, ut), (K_BITS_PER_BYTE, leaf('8'))])
            ins[0] = gb.Irep(code.id, [union, whole], list(code.named))
            niches += 1

    # A niche-encoded enum's variant of no fields (`None`): a fresh local,
    # `DECL x`, then only its tag written into the niche, `*(T *)&x := tag`.
    # Made a write of the whole value, the bytes after the tag zero.
    def symbol_of_niche_write(code):
        lhs = code.subs[0]
        if lhs.id != DEREF or not lhs.subs:
            return None
        p = lhs.subs[0]
        while p.id == TYPECAST and p.subs:
            p = p.subs[0]
        if p.id != ADDRESS_OF or not p.subs or p.subs[0].id != SYMBOL:
            return None
        return p.subs[0]

    for _, instrs in functions:
        for i, ins in enumerate(instrs[1:], 1):
            code, prev = ins[0], instrs[i - 1]
            if ins[2] != ASSIGN or len(code.subs) != 2 or prev[2] != DECL or not prev[0].subs:
                continue
            x = symbol_of_niche_write(code)
            if x is None:
                continue
            declared, xid = prev[0].subs[0], named(x, K_IDENT)
            if declared.id != SYMBOL or xid is None or named(declared, K_IDENT) is None or named(declared, K_IDENT).id != xid.id:
                continue
            xt, tag = named(x, K_TYPE), code.subs[1]
            tt = named(tag, K_TYPE)
            xw, tw = width(xt), (width(tt) if tt is not None else None)
            if xw is None or tw is None or tw > xw:
                continue
            comps, ops = [gb.Irep(EMPTY, [], [(K_NAME, leaf('tag')), (K_TYPE, tt)])], [tag]
            if xw > tw:
                tail = gb.Irep(UNSIGNEDBV, [], [(K_WIDTH, leaf(str(xw - tw)))])
                comps.append(gb.Irep(EMPTY, [], [(K_NAME, leaf('tail')), (K_TYPE, tail)]))
                ops.append(zero(tail))
            bytes_of = gb.Irep(STRUCT, ops, [(K_TYPE, gb.Irep(STRUCT, [], [(K_COMPONENTS, gb.Irep(EMPTY, comps, []))]))])
            whole = gb.Irep(BYTE_EXTRACT, [bytes_of, offset0], [(K_TYPE, xt), (K_BITS_PER_BYTE, leaf('8'))])
            ins[0] = gb.Irep(code.id, [x, whole], list(code.named))
            niches += 1

    # Every allocation zeroed, as `calloc`'s is.
    ALLOCATE = sid('allocate')

    def allocations(name):
        for fname, instrs in functions:
            if fname in (name, name.encode()):
                for ins in instrs:
                    code = ins[0]
                    if ins[2] == ASSIGN and len(code.subs) == 2:
                        rhs = code.subs[1]
                        st = named(rhs, K_STATEMENT)
                        if rhs.id == SIDE_EFFECT and st is not None and st.id == ALLOCATE and len(rhs.subs) == 2:
                            yield ins, code, rhs
    zeroed = next((rhs.subs[1] for _, _, rhs in allocations('calloc')), None)
    if zeroed is None:
        zeroed = gb.Irep(CONSTANT, [], [(K_TYPE, gb.Irep(BOOL, [], [])), (K_VALUE, leaf('true'))])
    allocs = 0
    for ins, code, rhs in list(allocations('malloc')):
        ins[0] = gb.Irep(code.id, [code.subs[0], gb.Irep(rhs.id, [rhs.subs[0], zeroed], list(rhs.named))], list(code.named))
        allocs += 1
    return padding, variants, fieldless, niches, tags, dataful, reordered, allocs


def main(src, dst):
    model = gb.read(src)
    padding, variants, fieldless, niches, tags, dataful, reordered, allocs = rewrite(*model)
    gb.write(dst, *model)
    print(f'{padding} padding values made zero, {variants} variant writes made whole, '
          f'{fieldless} fieldless variants made whole, '
          f'{niches} niche variant writes made whole ({tags} with their tags, {dataful} holding the niche), '
          f'{reordered} enums with their widest variant put first, {allocs} allocations zeroed')


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])

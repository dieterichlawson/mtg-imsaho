#!/usr/bin/env python3
"""goto_binary.py: read and write CBMC goto binaries (format version 6), as
util/irep_serialization.cpp and goto-programs/{read_bin_goto_object,
write_goto_binary}.cpp define them.

An irep is a node [id, subs, named], with `id` and the keys of `named`
string ids into `strings`; nodes keep the sharing they were read with.
"""
import sys


class Irep:
    __slots__ = ('id', 'subs', 'named')

    def __init__(self, id, subs, named):
        self.id, self.subs, self.named = id, subs, named

    def get(self, strings_rev, key):
        k = strings_rev.get(key)
        for n, v in self.named:
            if n == k:
                return v
        return None


class Reader:
    def __init__(self, data):
        self.d, self.p = data, 0
        self.strings = {}
        self.ireps = {}

    def byte(self):
        b = self.d[self.p]
        self.p += 1
        return b

    def word(self):
        res, shift = 0, 0
        while True:
            ch = self.byte()
            res |= (ch & 0x7f) << shift
            shift += 7
            if not ch & 0x80:
                return res

    def string(self):
        out = bytearray()
        while True:
            c = self.byte()
            if c == 0:
                return bytes(out)
            if c == 0x5c:  # backslash escapes the next byte
                c = self.byte()
            out.append(c)

    def string_ref(self):
        i = self.word()
        if i not in self.strings:
            self.strings[i] = self.string()
        return i

    def ref(self):
        i = self.word()
        if i not in self.ireps:
            self.ireps[i] = self.irep()
        return self.ireps[i]

    def irep(self):
        id = self.string_ref()
        subs, named = [], []
        while self.d[self.p] == 0x53:  # 'S'
            self.p += 1
            subs.append(self.ref())
        while self.d[self.p] in (0x4e, 0x43):  # 'N', 'C'
            self.p += 1
            k = self.string_ref()
            named.append((k, self.ref()))
        if self.byte() != 0:
            raise ValueError(f'irep not terminated at {self.p}')
        return Irep(id, subs, named)


def read(path):
    data = open(path, 'rb').read()
    if data[:4] != b'\x7fGBF':
        raise ValueError('not a goto binary')
    r = Reader(data)
    r.p = 4
    version = r.word()
    symbols = []
    for _ in range(r.word()):
        t, v, loc = r.ref(), r.ref(), r.ref()
        names = [r.string_ref() for _ in range(5)]  # name, module, base_name, mode, pretty_name
        ordering, flags = r.word(), r.word()
        symbols.append([t, v, loc, names, ordering, flags])
    functions = []
    for _ in range(r.word()):
        fname = r.string()
        instrs = []
        for _ in range(r.word()):
            code, loc = r.ref(), r.ref()
            itype = r.word()
            guard = r.ref()
            target_number = r.word()
            targets = [r.word() for _ in range(r.word())]
            labels = [r.string_ref() for _ in range(r.word())]
            instrs.append([code, loc, itype, guard, target_number, targets, labels])
        functions.append((fname, instrs))
    if r.p != len(data):
        raise ValueError(f'{len(data) - r.p} trailing bytes')
    return version, r.strings, symbols, functions


class Writer:
    def __init__(self, strings):
        self.out = bytearray()
        self.strings = strings
        self.written_strings = set()
        self.numbers = {}

    def word(self, u):
        while True:
            v = u & 0x7f
            u >>= 7
            if u == 0:
                self.out.append(v)
                return
            self.out.append(v | 0x80)

    def string(self, s):
        for c in s:
            if c == 0 or c == 0x5c:
                self.out.append(0x5c)
            self.out.append(c)
        self.out.append(0)

    def string_ref(self, i):
        self.word(i)
        if i not in self.written_strings:
            self.written_strings.add(i)
            self.string(self.strings[i])

    def ref(self, node):
        n = self.numbers.get(id(node))
        if n is not None:
            self.word(n)
            return
        n = len(self.numbers)
        self.numbers[id(node)] = n
        self.word(n)
        self.string_ref(node.id)
        for s in node.subs:
            self.out.append(0x53)
            self.ref(s)
        for k, v in node.named:
            self.out.append(0x4e)
            self.string_ref(k)
            self.ref(v)
        self.out.append(0)


def write(path, version, strings, symbols, functions):
    w = Writer(strings)
    w.out += b'\x7fGBF'
    w.word(version)
    w.word(len(symbols))
    for t, v, loc, names, ordering, flags in symbols:
        w.ref(t); w.ref(v); w.ref(loc)
        for n in names:
            w.string_ref(n)
        w.word(ordering); w.word(flags)
    w.word(len(functions))
    for fname, instrs in functions:
        w.string(fname)
        w.word(len(instrs))
        for code, loc, itype, guard, target_number, targets, labels in instrs:
            w.ref(code); w.ref(loc)
            w.word(itype)
            w.ref(guard)
            w.word(target_number)
            w.word(len(targets))
            for t in targets:
                w.word(t)
            w.word(len(labels))
            for l in labels:
                w.string_ref(l)
    open(path, 'wb').write(w.out)


if __name__ == '__main__':
    # Round trip: goto_binary.py IN OUT
    write(sys.argv[2], *read(sys.argv[1]))

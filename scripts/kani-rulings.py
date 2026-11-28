#!/usr/bin/env python3
"""kani-rulings.py [--check | --count]: every official ruling the Oracle
cache holds, and the proofs through the game that account for each.

`scripts/kani/rulings.tsv` has a row for each ruling in
`data/oracle_cache.json` (from Scryfall, which `scripts/oracle_lookup.py`
keeps): the module of the card it is of, the card's name, the ruling's
date, its text, and what accounts for it. That is one of:

- the names of harnesses of `proofs::through_the_stack`, the card's own or
  rows of the rules (`proofs::through_the_stack::rules`): proofs that play
  the situation the ruling describes through the engine and hold what
  comes of it to what the ruling says;
- `-`: nothing accounts for it yet;
- `not in the pool`: the ruling is of a card the engine does not have. The
  cache holds cards looked up for other reasons, and their rulings are
  listed so that every ruling in it is on this list and none is counted
  that the engine could not be held to.

Written by hand, by whoever writes the proof, and kept when the file is
written again. A ruling counts as accounted for when it names a proof,
and as proved when every proof it names last ended SUCCESSFUL
(`scripts/kani/times.txt`).

What this cannot say is whether the cache has every ruling there is for a
card of the pool: it has what was fetched when the card was.

With no argument, writes the file again from the cache and from what it
has. With --check, exits 1 if the file is not what that would write, or a
row names a harness that is not there or is another card's. With --count,
prints the numbers.
"""
import importlib.util, json, os, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LEDGER = f'{ROOT}/scripts/kani/rulings.tsv'
HEAD = ('# Written by scripts/kani-rulings.py from the Oracle cache\'s rulings; the fifth column is kept.\n'
        '# card\tname\tdate\truling\taccounted for by\n')
NOT_OURS = 'not in the pool'


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), f'{ROOT}/scripts/{name}.py')
    module = importlib.util.module_from_spec(spec)
    saved, sys.argv = sys.argv, [f'{name}.py']
    spec.loader.exec_module(module)
    sys.argv = saved
    return module


def rulings(claims):
    """(module or `-`, card name, date, text) for every ruling of the
    cache: the pool's cards in the pool's order, then the others by name."""
    cache = json.load(open(f'{ROOT}/data/oracle_cache.json'))['rulings']
    modules = {name: module for module, _, name, _ in claims.load('kani-printed-rows').pool()}
    for name in [n for n in modules if n in cache] + sorted(n for n in cache if n not in modules):
        for ruling in cache[name]:
            text = ' '.join(ruling['text'].split())
            yield modules.get(name, '-'), name, ruling.get('date') or '-', text


def kept():
    found = {}
    if os.path.exists(LEDGER):
        for row in open(LEDGER):
            if not row.startswith('#') and row.strip():
                module, name, date, text, by = row.rstrip('\n').split('\t')
                found[name, date, text] = by.split() if by != NOT_OURS else [NOT_OURS]
    return found


def render(claims):
    old, rows, seen = kept(), [], set()
    for module, name, date, text in rulings(claims):
        key = (name, date, text)
        if key in seen:
            continue    # the cache has a ruling twice
        seen.add(key)
        by = [NOT_OURS] if module == '-' else [c for c in old.get(key, []) if c not in ('-', NOT_OURS)] or ['-']
        rows.append((module, name, date, text, by))
    return HEAD + ''.join(f'{m}\t{n}\t{d}\t{t}\t{" ".join(b)}\n' for m, n, d, t, b in rows), rows


def twice():
    """How many rulings the cache has more than once, word for word."""
    cache = json.load(open(f'{ROOT}/data/oracle_cache.json'))['rulings']
    every = [(name, r.get('date') or '-', ' '.join(r['text'].split())) for name, of_it in cache.items() for r in of_it]
    return len(every) - len(set(every))


def numbers(claims=None, rows=None, of=None):
    """Rulings in the cache, those of cards not in the pool, those of the
    pool's, and of those the ones accounted for and the ones proved."""
    claims = claims or load('kani-claims')
    of = of if of is not None else claims.harnesses()
    rows = rows if rows is not None else render(claims)[1]
    last = claims.results()
    ours = [(m, b) for m, _, _, _, b in rows if b != [NOT_OURS]]
    accounted = [(m, b) for m, b in ours if b != ['-']]
    path = lambda m, h: of.get(m, {}).get(h) or of.get(claims.OF_THE_RULES, {}).get(h)
    proved = [b for m, b in accounted if all(last.get(path(m, h)) == 'SUCCESSFUL' for h in b)]
    return len(rows), len(rows) - len(ours), len(ours), len(accounted), len(proved)


def main():
    claims = load('kani-claims')
    of = claims.harnesses()
    text, rows = render(claims)
    said = '%d rulings in the cache (and %d it has twice): %d of cards not in the pool, %d of the pool\'s; %d of those accounted for by a proof, %d proved' % (
        (len(rows), twice()) + numbers(claims, rows, of)[1:])
    if sys.argv[1:] == ['--count']:
        print(said)
        return
    if sys.argv[1:] == ['--check']:
        if not os.path.exists(LEDGER) or open(LEDGER).read() != text:
            sys.exit('scripts/kani/rulings.tsv is not what scripts/kani-rulings.py writes: run it')
        bad = [f'{module}: {h} is to account for a ruling of {name} and is not a harness of that card or of the rules'
               for module, name, _, _, by in rows for h in by
               if h not in ('-', NOT_OURS) and h not in of.get(module, {}) and h not in of.get(claims.OF_THE_RULES, {})]
        if bad:
            sys.exit('\n'.join(bad))
        return
    with open(LEDGER, 'w') as f:
        f.write(text)
    print(said)


if __name__ == '__main__':
    main()

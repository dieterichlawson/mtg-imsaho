#!/usr/bin/env python3
"""kani-card-ledger.py RESULTS COMMIT: the card proofs' results, card by card.

Reads a results file written by `kani-bounded.py` and the harnesses the
proofs define at the working tree, and prints a Markdown ledger: for every
card in `mtg-engine/src/cards`, the harnesses that name it and how each
ended. A harness is a card's when its name starts with the card's module
name or with the name of the card's back face (`homicidal_brute_...` is
Civilized Scholar's); the harnesses that walk a table of cards
(`the_static_curses_print_what_their_text_says`) are listed on their own.
A card with no harness of its own is one whose code is its printed data
alone, which `tests/card_characteristics.rs` holds to its Oracle record.
"""
import collections, glob, importlib.util, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CARDS = f'{ROOT}/mtg-engine/src/cards'


def harnesses():
    spec = importlib.util.spec_from_file_location('kb', f'{ROOT}/scripts/kani-bounded.py')
    kb = importlib.util.module_from_spec(spec)
    saved = sys.argv
    sys.argv = ['kani-bounded.py']
    spec.loader.exec_module(kb)
    sys.argv = saved
    return [h for h in kb.list_harnesses(f'{ROOT}/mtg-engine/src') if h.startswith('proofs::cards::')]


def snake(name):
    return re.sub(r'[^a-z0-9]+', '_', name.lower().replace("'", '')).strip('_')


def cards():
    """Each card's module, and the snake-case names it is known by: its
    own, its back face's, and the first word of its name when no other
    card's starts with it (Grimgrin, Garruk)."""
    known, first = {}, collections.Counter()
    for f in sorted(glob.glob(f'{CARDS}/*.rs') + glob.glob(f'{CARDS}/isd/*.rs')):
        m = os.path.basename(f)[:-3]
        if m in ('mod', 'helpers'):
            continue
        src = open(f).read()
        names = {m}
        back = src.find('fn back_face_data')
        if back >= 0:
            b = re.search(r'name:\s*"([^"]+)"', src[back:])
            if b:
                names.add(snake(b.group(1)))
        known[m] = names
        first[m.split('_')[0]] += 1
    for m, names in known.items():
        word = m.split('_')[0]
        if first[word] == 1 and word != m:
            names.add(word)
    return known


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    results, commit = sys.argv[1], sys.argv[2]
    last = {}
    for line in open(results, errors='replace'):
        p = line.split()
        if len(p) > 2:
            last[p[2]] = (p[0], p[1])
    known = cards()
    by_name = sorted(((n, m) for m, ns in known.items() for n in ns), key=lambda x: -len(x[0]))
    per = collections.defaultdict(list)
    tables = []
    for h in harnesses():
        leaf = h.split('::', 3)[3]
        card = next((m for n, m in by_name if leaf == n or leaf.startswith(n + '_') or leaf.startswith(n + '::')), None)
        (per[card] if card else tables).append(h)

    def status(h):
        return last.get(h, ('NOT_RUN', '-'))

    every = [h for hs in per.values() for h in hs] + tables
    count = collections.Counter(status(h)[0] for h in every)
    out = [f'# Card proofs at {commit}', '',
           f'{len(every)} harnesses over {len(per)} cards and {len(tables)} tables of cards; '
           f'{len(known) - len(per)} cards are their printed data alone.', '']
    out += [f'- {k}: {v}' for k, v in sorted(count.items())] + ['']
    out += ['| card | harnesses | proved | not proved |', '|---|---|---|---|']
    for m in sorted(per):
        hs = per[m]
        bad = [f'`{h.split("::")[-1]}` ({status(h)[0]})' for h in hs if status(h)[0] != 'SUCCESSFUL']
        ok = sum(1 for h in hs if status(h)[0] == 'SUCCESSFUL')
        out.append(f'| {m} | {len(hs)} | {ok} | {", ".join(bad)} |')
    out += ['', '## Tables of cards', '', '| harness | result | time |', '|---|---|---|']
    for h in tables:
        r, t = status(h)
        out.append(f'| `{h.split("::", 2)[2]}` | {r} | {t} |')
    out += ['', '## Cards that are their printed data alone', '',
            ', '.join(sorted(m for m in known if m not in per)), '']
    print('\n'.join(out))


if __name__ == '__main__':
    main()

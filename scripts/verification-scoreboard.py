#!/usr/bin/env python3
"""verification-scoreboard.py [--check FILE]: the numbers of
`docs/plans/verification-goals.md`, counted from the sources and from the
results recorded in `scripts/kani/times.txt`, as Markdown.

A goal there is done when its number here says so. With `--check FILE`
(the scoreboard as last committed), exits 1 if a number has gone the wrong
way since: fewer cards proved through the stack, more stand-ins, more
proofs open. A proof that was waiting to be run and has now been run and
did not finish is not one gone the wrong way: the open proofs may grow by
as many as the proofs not yet run have shrunk.

What is counted:

- goal 1: for each card of `mtg-engine/src/cards`, its harnesses in
  `proofs::through_the_stack` (a harness is a card's when its name starts
  with the card's module name, as in `kani-card-ledger.py`), and how many
  of them last ended SUCCESSFUL; and the harnesses of
  `proofs::engine_under_card`, which call a card's hook directly and are
  to go;
- goal 4: the engine functions a proof stands in for with `kani::stub`,
  other than the five outside the rules (`format!`, an object's name for
  the log, the log, the allocator's growing and freeing);
- goal 6: the harnesses whose last recorded result is neither SUCCESSFUL
  nor NOT_RUN (written, and not yet run to a result).

Goals 2, 3, 5 and 7 are counted from ledgers that do not exist yet
(`reports/card-mutants.tsv`, `reports/rulings-ledger.tsv`); each is said
to be uncounted until its ledger is there.
"""
import collections, glob, importlib.util, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
THROUGH, HOOK, DECLARED = 'proofs::through_the_stack::', 'proofs::engine_under_card::', 'proofs::cards::'
OUTSIDE = ('fmt::format', 'obj_name', 'GameState::log', 'grow_impl', 'deallocate')


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), f'{ROOT}/scripts/{name}.py')
    module = importlib.util.module_from_spec(spec)
    saved, sys.argv = sys.argv, [f'{name}.py']
    spec.loader.exec_module(module)
    sys.argv = saved
    return module


def stand_ins():
    """The engine functions some proof replaces, by name."""
    found = set()
    for f in glob.glob(f'{ROOT}/mtg-engine/src/proofs/**/*.rs', recursive=True):
        for m in re.finditer(r'kani::stub\(\s*\$?(?:crate::)?([A-Za-z0-9_:$]+)\s*,', open(f).read()):
            if not any(o in m.group(1) for o in OUTSIDE):
                found.add(m.group(1))
    return found


def numbers():
    ledger, kb = load('kani-card-ledger'), load('kani-bounded')
    last = {}
    for line in open(f'{ROOT}/scripts/kani/times.txt', errors='replace'):
        p = line.split()
        if len(p) > 2 and not line.startswith('#'):
            last[p[2]] = p[0]
    known = ledger.cards()
    by_name = sorted(((n, m) for m, ns in known.items() for n in ns), key=lambda x: -len(x[0]))

    def card_of(h):
        leaf = h.rsplit('::', 1)[1]
        return next((m for n, m in by_name if leaf == n or leaf.startswith(n + '_')), None)

    per = {m: collections.Counter() for m in known}
    unplaced = collections.Counter()
    for h in kb.list_harnesses(f'{ROOT}/mtg-engine/src'):
        kind = 'through' if h.startswith(THROUGH) else 'hook' if h.startswith(HOOK) else 'declared' if h.startswith(DECLARED) else None
        if kind is None:
            continue
        card = card_of(h)
        if card is None:
            unplaced[kind] += 1
            continue
        per[card][kind] += 1
        if kind == 'through' and last.get(h) == 'SUCCESSFUL':
            per[card]['proved'] += 1
    # A harness that has a result and did not prove is open; one recorded
    # as NOT_RUN has been written and not yet run to a result.
    open_proofs = sorted(h for h, r in last.items() if r not in ('SUCCESSFUL', 'NOT_RUN'))
    not_run = sorted(h for h, r in last.items() if r == 'NOT_RUN')
    return per, unplaced, sorted(stand_ins()), open_proofs, not_run


def render():
    per, unplaced, stubs, open_proofs, not_run = numbers()
    through = sum(1 for c in per.values() if c['through'] and c['proved'] == c['through'])
    partly = sum(1 for c in per.values() if c['through'] and c['proved'] != c['through'])
    hooks = sum(c['hook'] for c in per.values()) + unplaced['hook']
    cases = sum(c['through'] for c in per.values())
    proved = sum(c['proved'] for c in per.values())
    out = ['# Verification scoreboard', '',
           'Counted by `scripts/verification-scoreboard.py`; the goals are in',
           '`docs/plans/verification-goals.md`.', '',
           '| Goal | Now | Done at |', '|---|---|---|',
           f'| 1. cards with every proof written for them through the engine\'s cast, stack and resolution proved | {through} of {len(per)}'
           f'{f" (and {partly} with some not yet proved)" if partly else ""} | {len(per)} |',
           f'| 1. those proofs: proved of written | {proved} of {cases} ({len(not_run)} not yet run) | all |',
           f'| 1. harnesses that call a card\'s hook directly | {hooks} | 0 |',
           '| 2. cards whose proofs cover every case that applies | not counted yet | all |',
           '| 3. changes to a card\'s code its proofs catch | not counted yet | every one |',
           f'| 4. engine functions a proof stands in for, unchecked | {len(stubs)} | 0 |',
           '| 5. official rulings accounted for | not counted yet | 578 |',
           f'| 6. proofs that do not finish | {len(open_proofs)} | 0 |',
           '| 7. permanents that can be pieces of the small boards | 129 of 199 (counted by hand, 2026-11-27) | 199 |',
           '', '## Cards through the stack', '',
           '| card | through the stack, proved of written | by its hook alone | declared |', '|---|---|---|---|']
    for m in sorted(per):
        c = per[m]
        mark = f'{c["proved"]} of {c["through"]}' if c['through'] else '-'
        out.append(f'| {m} | {mark} | {c["hook"]} | {c["declared"]} |')
    out += ['', '## Engine functions stood in for', '', ', '.join(f'`{s}`' for s in stubs), '']
    return '\n'.join(out), (through, -hooks, -len(stubs), -len(open_proofs)), len(not_run)


def main():
    text, now, waiting = render()
    if len(sys.argv) == 3 and sys.argv[1] == '--check':
        was = open(sys.argv[2]).read()
        m = (re.search(r'resolution proved \| (\d+) of', was), re.search(r'hook directly \| (\d+)', was),
             re.search(r'unchecked \| (\d+)', was), re.search(r'do not finish \| (\d+)', was))
        if all(m):
            # Proofs that were waiting and have since been run to a result.
            waited = re.search(r'\((\d+) not yet run\)', was)
            run_since = max(0, int(waited.group(1)) - waiting) if waited else 0
            before = (int(m[0].group(1)), -int(m[1].group(1)), -int(m[2].group(1)), -int(m[3].group(1)) - run_since)
            worse = [name for name, a, b in zip(('cards through the stack', 'hook harnesses', 'stand-ins', 'open proofs'), before, now) if b < a]
            if worse:
                sys.exit(f'the scoreboard went the wrong way: {", ".join(worse)} (was {before}, now {now})')
    print(text)


if __name__ == '__main__':
    main()

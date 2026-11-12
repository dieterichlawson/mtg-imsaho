#!/usr/bin/env python3
"""scripts/kani-tiers.py: the harness lists the Kani tiers in CI run.

    kani-tiers.py write RESULTS [--budget SECONDS]
        From RESULTS (a results file of a run of every harness, as
        kani-bounded.py writes it), write scripts/kani/:
          times.txt          each harness's last result and seconds, which
                             --shard shares the scheduled run out by;
          expected-open.txt  the harnesses whose last result is not
                             SUCCESSFUL, which the tiers do not fail on;
          pr.list            the harnesses a pull request runs: the
                             fastest proved harness of each module of the
                             proofs, fastest first, while their seconds sum
                             to under BUDGET (default 600).
    kani-tiers.py pr [--base REF] [--extra SECONDS]
        The harnesses a pull request runs, printed: pr.list, and for each
        proof module the change edits (git diff REF...HEAD, default
        origin/master), its own proved harnesses, fastest first, while they
        sum to under EXTRA seconds (default 600) more.

The scheduled run of every harness (`.github/workflows/proofs-full.yml`)
is what refreshes these: its merged results are the RESULTS to write
from. `docs/plans/formal-verification.md`, "Running the proofs", says what
each tier is for.
"""
import argparse, importlib.util, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = os.path.join(ROOT, 'scripts', 'kani')


def last_results(path):
    """Each harness's last result and the seconds of every attempt summed."""
    last, took = {}, {}
    with open(path, errors='replace') as f:
        for l in f:
            p = l.split()
            if len(p) < 3:
                continue
            last[p[2]] = p[0]
            m = re.match(r'(\d+)s$', p[1])
            took[p[2]] = took.get(p[2], 0) + (int(m.group(1)) if m else 0)
    return last, took


def harnesses():
    spec = importlib.util.spec_from_file_location('kb', os.path.join(ROOT, 'scripts', 'kani-bounded.py'))
    kb = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(kb)
    return kb.list_harnesses(os.path.join(ROOT, 'mtg-engine', 'src'))


def module(h):
    return h.rsplit('::', 1)[0]


def write(results, budget):
    last, took = last_results(results)
    every = harnesses()
    missing = [h for h in every if h not in last]
    if missing:
        sys.exit(f'{len(missing)} harnesses have no result in {results}, the first {missing[0]}')
    os.makedirs(DIR, exist_ok=True)
    with open(os.path.join(DIR, 'times.txt'), 'w') as f:
        f.write('# Each harness\'s last result and its seconds, every attempt summed, from the\n'
                '# last run of every harness (scripts/kani-tiers.py write). --shard reads it.\n')
        for h in every:
            f.write(f'{last[h]:10s} {took[h]:>7d}s  {h}\n')
    open_ = [h for h in every if last[h] != 'SUCCESSFUL']
    with open(os.path.join(DIR, 'expected-open.txt'), 'w') as f:
        f.write('# The harnesses known not to prove, each with its last result in the last\n'
                '# run of every harness (scripts/kani-tiers.py write). The tiers in CI fail\n'
                '# on any other harness that does not prove; one of these that proves is\n'
                '# said so, to be taken off.\n')
        for h in open_:
            f.write(f'{h}  # {last[h]}\n')
    fastest = {}
    for h in every:
        if last[h] == 'SUCCESSFUL' and (module(h) not in fastest or took[h] < took[fastest[module(h)]]):
            fastest[module(h)] = h
    chosen, total = [], 0
    for h in sorted(fastest.values(), key=lambda h: (took[h], h)):
        if total + took[h] > budget:
            break
        chosen.append(h)
        total += took[h]
    with open(os.path.join(DIR, 'pr.list'), 'w') as f:
        f.write(f'# What a pull request proves: the fastest proved harness of each module of\n'
                f'# the proofs, fastest first, {len(chosen)} of the {len(fastest)} modules within {budget} s\n'
                f'# ({total} s in the last run). Written by scripts/kani-tiers.py write.\n')
        for h in chosen:
            f.write(h + '\n')
    print(f'{len(every)} harnesses: {len(open_)} expected open; pr.list {len(chosen)} of {len(fastest)} modules, {total} s')


def pr(base, extra):
    names = [n for n in (l.split('#', 1)[0].strip() for l in open(os.path.join(DIR, 'pr.list'))) if n]
    last, took = last_results(os.path.join(DIR, 'times.txt'))
    changed = subprocess.run(['git', 'diff', '--name-only', f'{base}...HEAD'], cwd=ROOT,
                             capture_output=True, text=True, check=True).stdout.split()
    prefix = 'mtg-engine/src/'
    modules = set()
    for path in changed:
        if path.startswith(prefix + 'proofs/') and path.endswith('.rs'):
            m = path[len(prefix):-len('.rs')]
            modules.add(m[:-len('/mod')] if m.endswith('/mod') else m)
    modules = {m.replace('/', '::') for m in modules}
    total = 0
    for h in sorted((h for h in harnesses() if any(h.startswith(m + '::') for m in modules) and h not in names),
                    key=lambda h: (took.get(h, 120), h)):
        if last.get(h, 'SUCCESSFUL') != 'SUCCESSFUL':
            continue
        if total + took.get(h, 120) > extra:
            break
        names.append(h)
        total += took.get(h, 120)
    print('\n'.join(names))


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='what', required=True)
    w = sub.add_parser('write')
    w.add_argument('results')
    w.add_argument('--budget', type=int, default=600)
    p = sub.add_parser('pr')
    p.add_argument('--base', default='origin/master')
    p.add_argument('--extra', type=int, default=600)
    a = ap.parse_args()
    if a.what == 'write':
        write(a.results, a.budget)
    else:
        pr(a.base, a.extra)


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""scripts/kani-bounded.py [NAME...] [--results FILE] [--batch N] [--timeout S] [--bound B] [--as-built]
                          [--start-unwind U] [--loops FILE] [--jobs J --mem-cap GB] [--memory-checks]

Run the Kani proofs in mtg-engine/src/proofs/ the way `cargo kani` does,
with one addition: every function in a harness's final goto program that
recurses over the engine's self-referring types (filters, effects,
target requirements, and the drop, clone and comparison glue Rust
generates for them) gets a recursion bound of B (default 3) through
CBMC's --unwindset. CBMC keeps its recursion unwinding assertions, so a
bound too small for the data fails the proof rather than passing it.
`docs/plans/formal-verification.md`, "Cards proved with Kani", says why:
without the bound, dropping one filter read from the heap explored every
variant of every filter it might hold, to the unwinding depth.

And one rewrite of the goto program before cbmc reads it,
`scripts/precise-enums.py`, which says what it changes and why nothing a
Rust program can observe does: with Kani's encoding of enums as it is,
CBMC loses every pointer stored in an enum written to the heap, and
explores every variant of what it might point to. `--as-built` leaves the
program as Kani built it, to compare.

Loops are unwound to U (default 4) first, not to the harness's bound: a
board of three objects needs no more, and a loop whose count the solver
leaves open (collecting the creatures a player controls) is otherwise
unwound to the harness's bound, each pass scanning the board again. A
loop whose unwinding assertion fails at U is given the harness's bound
and the harness is run again; one that fails at the harness's bound is
a failure, as with Kani. The loops so raised are kept in FILE (default
logs/kani/loop-bounds.txt), by function name and loop number, for the
harnesses after. The unwinding assertions stay on throughout, so a
harness passes only with every loop unwound as far as it goes.

Per batch: `cargo kani --only-codegen` for the batch's harnesses; per
harness: Kani's goto-cc link and goto-instrument passes, the rewrite,
then cbmc with Kani's flags, the bounds and --json-ui. A result is read as Kani reads
it: reachability checks are not failures, and covers are counted.

CBMC's pointer and bounds checks are off (`NO_MEMORY_CHECKS` says why);
--memory-checks keeps them.

With --jobs J, J harnesses of a batch are verified at once, each cbmc
held to GB gigabytes of address space; one that runs out of it is run
again alone, once the rest are done, held to the --solo-cap (12): with
none, one that ran away took the machine, this driver with it. With one
at a time, each is held to the --solo-cap. Kept to one at a time
by default: two solvers sharing memory the kernel runs out of are both
at risk, not only the one that needed it.

A NAME is a harness's path or last part, or a module (`vec_map`,
`cards::auras`) for every harness in it; with none, every harness the
proofs define. One line per harness
goes to RESULTS (default logs/kani/results.txt):
    RESULT  SECONDS  NAME  [bounds, covers, size, or what failed]
"""
import argparse, concurrent.futures, glob, json, os, re, subprocess, sys, threading, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
K = os.path.expanduser('~/.kani/kani-0.68.0')
KANI_LIB = f'{K}/library/kani/kani_lib.c'

# Every harness the proofs define, read off their source: a #[kani::proof]
# fn, a card_proof! or action_proof!, a for_each_form! (one harness per
# form), and the tables and forms the proof modules declare whole sets of
# harnesses with (abilities!, statics!, equipment!, ...). `cargo kani list`
# says the same at the price of building every harness, gigabytes of goto
# files.
TABLES = {'abilities', 'declares', 'werewolves', 'mana', 'statics', 'check_land', 'choosing_at_random', 'for_each_size', 'from_each_zone', 'for_each_key', 'each_printed', 'by_each_way', 'each_target', 'each_board', 'each_cost', 'each_pruning', 'each_path', 'each_set', 'over_each_library', 'over_each_hand', 'case_by_case'}
FORMS = {'reaches_what_it_describes', 'holds_exactly_then'}
def block(text, i):
    # text[i] is an opening bracket; return the index of its match
    open_, close = text[i], {'{': '}', '(': ')', '[': ']'}[text[i]]
    depth = 0
    for j in range(i, len(text)):
        if text[j] == open_: depth += 1
        elif text[j] == close:
            depth -= 1
            if depth == 0: return j
    raise ValueError
def strip(text):
    # drop comments and string literals' content enough not to confuse matching
    text = re.sub(r'//[^\n]*', '', text)
    return text
# Loops a proof module bounds below the start, by its comment
# `// loop bound: <function> <n> [<part of a harness name>]`: every loop of
# that function (its pretty name, as `pretty_names` has it) is unwound to n
# in each harness of the module (whose name holds the part, if one is
# given), the unwinding assertions on as everywhere, so a bound too small
# fails the proof. For a loop the solver cannot see the end of in symbolic
# execution, over a list whose length it leaves open: unwound to the start
# bound, each pass past the list's end is a pass over what it cannot rule
# out, and the damage queue's ran out of memory so.
LOOP_CAPS = {}


def list_harnesses(src):
    out = []
    for f in sorted(glob.glob(src + '/proofs/**/*.rs', recursive=True)):
        rel = os.path.relpath(f, src)[:-3].split('/')
        mod = [p for p in rel if p != 'mod']
        path = '::'.join(mod)
        raw = open(f).read()
        for m in re.finditer(r'(?m)^\s*// loop bound: (\S+) (\d+)(?: (\w+))?\s*$', raw):
            LOOP_CAPS.setdefault(path, []).append((m.group(1), int(m.group(2)), m.group(3) or ''))
        t = strip(raw)
        for m in re.finditer(r'\b(card_proof|action_proof)!\s*\{', t):
            if t[m.start()-13:m.start()].endswith('macro_rules! '): continue
            body = t[m.end()-1:block(t, m.end()-1)+1]
            n = re.search(r'\bfn (\w+)\(\)', body)
            # A body with a `$` is a macro's own use of the macro, not a proof.
            if n and not body.lstrip('{').strip().startswith('(') and '$' not in body:
                out.append(path + '::' + n.group(1))
        for m in re.finditer(r'(?m)^effect_proof!\s*\{', t):
            body = t[m.end()-1:block(t, m.end()-1)+1]
            n = re.search(r'\bfn (\w+)\(', body)
            if n:
                out.append(path + '::' + n.group(1))
        for m in re.finditer(r'#\[kani::proof\][^\n]*\n(?:\s*#\[[^\n]*\n)*\s*(?:pub )?fn (\w+)', t):
            out.append(path + '::' + m.group(1))
        for m in re.finditer(r'\bfor_each_form!\s*\{', t):
            body = t[m.end()-1:block(t, m.end()-1)+1]
            n = re.search(r'\bfn (\w+)\(form\)', body)
            if n: out += [path + '::' + n.group(1) + '::of_a_card', path + '::' + n.group(1) + '::of_a_token']
        for name in TABLES | FORMS:
            for m in re.finditer(r'(?<!macro_rules! )\b' + name + r'!\s*[{(]', t):
                body = t[m.end()-1:block(t, m.end()-1)+1]
                for e in re.finditer(r'(?m)^\s{4}(\w+):', body):
                    if name in FORMS:
                        out += [path + '::' + e.group(1) + '::of_a_card', path + '::' + e.group(1) + '::of_a_token']
                    else:
                        out.append(path + '::' + e.group(1))
        for name, k in [('equipment', 2), ('aura', 2), ('resolves', 1), ('destroys', 1)]:
            for m in re.finditer(r'(?m)^' + name + r'!\(', t):
                body = t[m.end():block(t, m.end()-1)]
                parts = [p.strip() for p in body.split(',')]
                if parts[0] == 'another targeted ability': parts = parts[1:]
                out += [path + '::' + p for p in parts[:k]]
    seen, names = set(), []
    for h in out:
        if h not in seen:
            seen.add(h); names.append(h)
    return names


TYPES = ('CreatureFilter', 'ContinuousEffect', 'EffectScope', 'TargetRequirement', 'EffectCondition', 'GameState>')
GLUE = re.compile(r'clone|drop|::eq\b|::ne\b|PartialEq|fmt|Debug|to_vec')
ENGINE = re.compile(r'(GameState::matches_filter|GameState::check_condition|ContinuousEffect::scope|ContinuousEffect::describe'
                    r'|valid_targets_for_req|candidate_req|arity_ok|fixed_arity|target_combinations|is_target_legal'
                    r'|distinct_within_each_target_instance|targets_are_legal)$')

CBMC_FLAGS = ['--no-malloc-may-fail', '--no-undefined-shift-check', '--no-signed-overflow-check', '--nan-check',
              '--no-self-loops-to-assumptions', '--no-pointer-primitive-check', '--object-bits', '16',
              '--sat-solver', 'cadical', '--slice-formula', '--max-field-sensitivity-array-size', '4096']
# CBMC's own checks that every dereference is of a live object and in its
# bounds. The proofs prove what the code does; that safe Rust (the engine
# has no `unsafe`) reads only memory it may is the compiler's guarantee,
# and Rust's own checks, an index past a slice's end and every other
# panic, are assertions in the program and stay. An invalid read would
# give a value the solver leaves open, which no assertion holds of unless
# it holds of every value. Without them Rolling Temblor's symbolic
# execution took half the time and left 3876 checks to solve, not 9670.
NO_MEMORY_CHECKS = ['--no-pointer-check', '--no-bounds-check', '--no-div-by-zero-check']


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def codegen(harnesses, crate, target):
    args = ['cargo', 'kani', '-Z', 'stubbing', '-Z', 'unstable-options', '--only-codegen', '--exact']
    for h in harnesses:
        args += ['--harness', h]
    env = dict(os.environ, CARGO_TARGET_DIR=target)
    p = run(args, cwd=crate, env=env)
    if p.returncode != 0 and 'Failed to canonicalize harness model' in p.stdout + p.stderr:
        # A cached build whose goto files were removed: build it again.
        os.utime(f'{crate}/src/lib.rs')
        p = run(args, cwd=crate, env=env)
    if p.returncode != 0:
        sys.stderr.write(p.stdout[-3000:] + p.stderr[-3000:])
        return None
    # The newest metadata names each harness's goto file.
    found = {}
    metas = sorted(glob.glob(f'{target}/kani/x86_64-unknown-linux-gnu/debug/build/mtg-engine/*/out/*.kani-metadata.json'),
                   key=os.path.getmtime, reverse=True)
    for m in metas:
        for h in json.load(open(m))['proof_harnesses']:
            if h['pretty_name'] in harnesses and h['pretty_name'] not in found:
                attrs = h['attributes']
                unwind = attrs.get('unwind_value') if isinstance(attrs, dict) else None
                found[h['pretty_name']] = (h['goto_file'], unwind, h['mangled_name'])
    return found


def bounds(out, bound):
    names = run([f'{K}/bin/goto-instrument', '--list-goto-functions', out]).stdout
    present = set(re.findall(r'/\* ([^ ,]+)', names))
    base = out[:-len('.out')]
    pretty = json.load(open(base + '.pretty_name_map.json'))
    ids = [k for k, v in pretty.items()
           if v and k in present and ((any(t in v for t in TYPES) and GLUE.search(v)) or ENGINE.search(v))]
    return ids


def verify(h, goto, unwind, mangled, bound, timeout, precise=True, cap=None):
    symtab = goto if goto.endswith('.symtab.out') else goto[:-len('.out')] + '.symtab.out'
    out = symtab[:-len('.symtab.out')] + '.out'
    steps = [
        [f'{K}/bin/goto-cc', symtab, KANI_LIB, '-o', out],
        [f'{K}/bin/goto-cc', out, '--function', mangled, '-o', out],
        [f'{K}/bin/goto-instrument', '--add-library', '--no-malloc-may-fail', out, out],
        [f'{K}/bin/goto-instrument', '--generate-function-body-options', 'assert-false-assume-false',
         '--generate-function-body', '.*', '--drop-unused-functions', out, out],
        [f'{K}/bin/goto-instrument', '--ensure-one-backedge-per-target', out, out],
    ]
    if precise:
        steps.append([sys.executable, f'{ROOT}/scripts/precise-enums.py', out, out])
    for cmd in steps:
        p = run(cmd)
        if p.returncode != 0:
            return ('ERROR', '-', f'{os.path.basename(cmd[1] if cmd[0] == sys.executable else cmd[0])} failed: {p.stderr[-200:]}')
    rec = bounds(out, bound)
    by_name = pretty_names(out)
    full = unwind or 7
    caps = {}
    for module, fns in LOOP_CAPS.items():
        if h.startswith(module + '::'):
            loops_of = re.findall(r'^Loop (\S+):', run([f'{K}/bin/goto-instrument', '--show-loops', out]).stdout, re.M)
            for name, n, part in fns:
                if part in h.rsplit('::', 1)[-1]:
                    for m in by_name.get(name, []):
                        caps.update({l: n for l in loops_of if l.startswith(m + '.')})
    learned = load_loops()
    took, rounds = 0.0, 0
    while True:
        rounds += 1
        raised = sorted({f'{m}.{n}' for name, n in learned for m in by_name.get(name, [])} - set(caps))
        unwindset = ','.join(['memcmp.0:33'] + [f'{r}:{bound}' for r in rec] + [f'{l}:{full}' for l in raised]
                             + [f'{l}:{n}' for l, n in sorted(caps.items())])
        cmd = [f'{K}/bin/cbmc'] + CBMC_FLAGS + ([] if MEMORY_CHECKS else NO_MEMORY_CHECKS) + [
            '--unwind', str(min(START_UNWIND, full)), '--unwindset', unwindset, out, '--verbosity', '9', '--json-ui']
        with open(out + '.cmd', 'w') as f:
            f.write(' '.join(f"'{c}'" if ',' in c or '*' in c else c for c in cmd) + '\n')
        if cap:
            cmd = ['prlimit', f'--as={int(cap * 2**30)}', '--'] + cmd
        result, why, seconds, loops, others = solve(cmd, timeout, len(rec))
        took += seconds
        new = loops - learned
        # Again with the loops that needed more than the starting bound at
        # the harness's own, until none does; a loop at the harness's bound
        # that still needs more, or any other failure, is the result.
        if result != 'FAILED' or not new or others:
            return (result, f'{took:.0f}s', why + (f' rounds {rounds}' if rounds > 1 else ''))
        learned |= new
        save_loops(learned)


LOOPS_LOCK = threading.Lock()


def load_loops():
    with LOOPS_LOCK:
        return read_loops()


def read_loops():
    if not os.path.exists(LOOPS):
        return set()
    return {tuple(l.rsplit(' ', 1)) for l in open(LOOPS).read().splitlines() if l.strip()}


def save_loops(loops):
    # Merged with what is there: a harness verified alongside may have
    # raised loops of its own since this one read the file.
    with LOOPS_LOCK:
        os.makedirs(os.path.dirname(LOOPS), exist_ok=True)
        merged = read_loops() | loops
        with open(LOOPS, 'w') as f:
            for name, n in sorted(merged):
                f.write(f'{name} {n}\n')


def pretty_names(out):
    """Each function name a result names, with the identifiers it has in `out`."""
    pretty = json.load(open(out[:-len('.out')] + '.pretty_name_map.json'))
    present = set(re.findall(r'/\* ([^ ,]+)', run([f'{K}/bin/goto-instrument', '--list-goto-functions', out]).stdout))
    by = {}
    for k, v in pretty.items():
        if v and k in present:
            by.setdefault(v, []).append(k)
    return by


def solve(cmd, timeout, nrec):
    """Run cbmc: the result, what to say of it, its seconds, the loops whose
    unwinding assertions failed, and every other failure.

    cbmc's output is read line by line as it is written, and only what is
    said of size, time, errors and results is kept: at the verbosity the
    statistics need it runs to gigabytes. Held in memory and parsed whole,
    it was this driver that the kernel stopped for want of memory, losing
    the harnesses it had left; written to a file beside its program and
    read back, three runs at once filled the disk. KANI_BOUNDED_KEEP writes
    it to that file besides."""
    log = next(c for c in cmd if c.endswith('.out')) + '.json'
    raw = open(log, 'w') if os.environ.get('KANI_BOUNDED_KEEP') else None
    start = time.time()
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors='replace')
    expired = threading.Event()

    def stop():
        expired.set()
        p.kill()

    timer = threading.Timer(timeout, stop) if timeout else None
    if timer:
        timer.daemon = True
        timer.start()
    messages, results, size = [], [], ''
    oom = False
    prop = desc = None

    def value(line):
        # A line cut short (cbmc stopped at the cap mid-write, or the disk
        # full) is no value: skipped, where it stopped the whole run.
        try:
            return json.loads(line.split(': ', 1)[1].rstrip().rstrip(','))
        except ValueError:
            return None

    for line in p.stdout:
        if raw:
            raw.write(line)
        if 'ut of memory' in line or 'bad_alloc' in line:
            oom = True
        if line.startswith('    "messageText": '):
            text = value(line)
            if isinstance(text, str):
                messages.append([text, None])
        elif line.startswith('    "messageType": ') and messages:
            messages[-1][1] = value(line)
            if len(messages) > 400:
                # Only what is said of size, time and errors is kept.
                messages = [m for m in messages if m[1] == 'ERROR' or re.match(
                    r'size of program|Runtime|.* variables, .* clauses|Invalid User Input', m[0])]
        elif line.startswith('        "description": '):
            desc = value(line)
        elif line.startswith('        "property": '):
            prop = value(line)
        elif line.startswith('        "status": ') and prop is not None:
            status = value(line)
            if status is not None:
                results.append((prop, status, desc))
            prop = desc = None
    code = p.wait()
    if timer:
        timer.cancel()
    if raw:
        raw.close()
    timed_out = expired.is_set()
    seconds = time.time() - start
    if timed_out:
        steps = next((m[0] for m in messages if m[0].startswith('size of program expression')), None)
        sym = next((m[0] for m in messages if m[0].startswith('Runtime Symex')), None)
        where = (f" {steps.split(':')[1].split()[0]} steps symex {float(sym.split(':')[1].strip().rstrip('s')):.0f}s, then solving"
                 if steps and sym else ' still in symbolic execution')
        return ('TIMEOUT', f'{nrec} bounded;{where}', seconds, set(), [])
    if code in (-9, 137):
        return ('OOM', f'{nrec} bounded', seconds, set(), [])
    if oom:
        # Under --mem-cap: the cap, not the machine, ran out.
        return ('OOM', f'{nrec} bounded; over the cap', seconds, set(), [])
    failed, others, loops, covers, covered = [], [], set(), 0, 0
    for prop, status, desc in results:
        if '.reachability_check.' in prop:
            continue
        if '.cover.' in prop:
            covers += 1
            covered += status == 'FAILURE'
            continue
        if status == 'FAILURE':
            m = re.match(r'(.*)\.unwind\.(\d+)$', prop)
            failed.append((desc or prop) + (f' ({prop})' if m else ''))
            if m:
                loops.add((m.group(1), m.group(2)))
            else:
                others.append(prop)
    for text, kind in messages:
        if 'Invalid User Input' in text:
            failed.append(text)
            others.append(text)
    errors = [t for t, k in messages if k == 'ERROR']
    size = ''
    for t, _ in messages:
        if t.startswith('size of program expression'):
            size += f" {t.split(':')[1].split()[0]} steps"
        elif t.startswith('Runtime Symex'):
            size += f" symex {float(t.split(':')[1].strip().rstrip('s')):.0f}s"
        elif ' variables, ' in t and 'clauses' in t and 'clauses' not in size:
            size += f" {t.split(',')[1].split()[0]} clauses"
        elif t.startswith('Runtime decision procedure') and 'solve' not in size:
            size += f" solve {float(t.split(':')[1].strip().rstrip('s')):.0f}s"
    if errors and not failed:
        return ('ERROR', ' '.join(errors)[:300], seconds, set(), ['error'])
    if not results and not errors:
        return ('ERROR', f'no results (exit {code})', seconds, set(), ['error'])
    cov = (f' covers {covered}/{covers}' if covers else '') + size
    if failed:
        return ('FAILED', f'{nrec} bounded{cov}; ' + ' | '.join(sorted(set(failed)))[:400], seconds, loops, others)
    return ('SUCCESSFUL', f'{nrec} bounded{cov}', seconds, set(), [])


START_UNWIND, LOOPS, MEMORY_CHECKS = 4, None, False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('harness', nargs='*')
    ap.add_argument('--results', default=f'{ROOT}/logs/kani/results.txt')
    ap.add_argument('--crate', default=f'{ROOT}/mtg-engine')
    ap.add_argument('--target', default=os.environ.get('CARGO_TARGET_DIR', f'{ROOT}/target'))
    ap.add_argument('--batch', type=int, default=8)
    ap.add_argument('--timeout', type=int, default=1800)
    ap.add_argument('--bound', type=int, default=3)
    ap.add_argument('--as-built', action='store_true', help='skip the precise-enums rewrite')
    ap.add_argument('--start-unwind', type=int, default=4)
    ap.add_argument('--loops', default=f'{ROOT}/logs/kani/loop-bounds.txt')
    ap.add_argument('--memory-checks', action='store_true', help="keep CBMC's pointer and bounds checks")
    ap.add_argument('--jobs', type=int, default=1)
    ap.add_argument('--mem-cap', type=float, default=None, help='gigabytes of address space per cbmc, with --jobs')
    ap.add_argument('--solo-cap', type=float, default=12, help='gigabytes for a harness run again alone')
    a = ap.parse_args()
    global START_UNWIND, LOOPS, MEMORY_CHECKS
    START_UNWIND, LOOPS, MEMORY_CHECKS = a.start_unwind, a.loops, a.memory_checks
    os.makedirs(os.path.dirname(a.results), exist_ok=True)
    every = list_harnesses(f'{a.crate}/src')
    # A name selects a harness by its full path or its last part, or every
    # harness in a module by the module's name (`vec_map`, `cards::auras`).
    wanted = [h for h in every if not a.harness
              or any(h == w or h.endswith('::' + w) or f'::{w}::' in f'::{h}' for w in a.harness)]
    def done():
        if not os.path.exists(a.results):
            return set()
        with open(a.results, errors='replace') as res:
            return {l.split()[2] for l in res if len(l.split()) > 2}

    todo = [h for h in wanted if h not in done()]
    # One at a time, a harness is held to the cap it would have had alone,
    # so that a run beside other work cannot take the machine's memory.
    cap = a.mem_cap if a.jobs > 1 else a.solo_cap
    written = threading.Lock()

    def write(line):
        with written, open(a.results, 'a') as res:
            res.write(line + '\n')

    def one(h, goto, unwind, mangled, cap):
        result, took, why = verify(h, goto, unwind, mangled, a.bound, a.timeout, precise=not a.as_built, cap=cap)
        if result == 'OOM' and cap and cap != a.solo_cap:
            print(f"{'OVER_CAP':10s} {took:>8s}  {h}  again alone, after the batch", flush=True)
            return h
        write(f'{result:10s} {took:>8s}  {h}  {why}')
        print(f'{result:10s} {took:>8s}  {h}', flush=True)
        # Its goto files, linked and as built: some thirty megabytes a
        # harness, and a run of the cards filled the disk with them.
        base = goto[:-len('.symtab.out')] if goto.endswith('.symtab.out') else goto[:-len('.out')]
        if not os.environ.get('KANI_BOUNDED_KEEP'):
            for f in glob.glob(glob.escape(base) + '.*'):
                if not f.endswith('.kani-metadata.json'):
                    os.remove(f)
        return None

    for i in range(0, len(todo), a.batch):
        # Read again before each batch: another run writing to the same
        # results may have done some of these since this one began.
        finished = done()
        batch = [h for h in todo[i:i + a.batch] if h not in finished]
        if not batch:
            continue
        found = codegen(batch, a.crate, a.target)
        if found is None:
            for h in batch:
                write(f"{'NOT_RUN':10s} {'-':>8s}  {h}  codegen failed")
            continue
        for h in batch:
            if h not in found:
                write(f"{'NOT_RUN':10s} {'-':>8s}  {h}  no goto file")
        here = [h for h in batch if h in found]
        with concurrent.futures.ThreadPoolExecutor(max_workers=a.jobs) as pool:
            again = [h for h in pool.map(lambda h: one(h, *found[h], cap), here) if h]
        # What ran out of its share of memory, alone and with all of it.
        for h in again:
            one(h, *found[h], a.solo_cap)


if __name__ == '__main__':
    main()

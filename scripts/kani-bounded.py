#!/usr/bin/env python3
"""scripts/kani-bounded.py [NAME...] [--results FILE] [--batch N] [--timeout S] [--bound B]

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

Per batch: `cargo kani --only-codegen` for the batch's harnesses; per
harness: Kani's goto-cc link and goto-instrument passes, then cbmc with
Kani's flags, the bounds and --json-ui. A result is read as Kani reads
it: reachability checks are not failures, and covers are counted.

A NAME is a harness's path or last part, or a module (`vec_map`,
`cards::auras`) for every harness in it; with none, every harness the
proofs define. One line per harness
goes to RESULTS (default logs/kani/results.txt):
    RESULT  SECONDS  NAME  [bounds, covers, size, or what failed]
"""
import argparse, glob, json, os, re, subprocess, sys, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
K = os.path.expanduser('~/.kani/kani-0.68.0')
KANI_LIB = f'{K}/library/kani/kani_lib.c'

# Every harness the proofs define, read off their source: a #[kani::proof]
# fn, a card_proof! or action_proof!, a for_each_form! (one harness per
# form), and the tables and forms the proof modules declare whole sets of
# harnesses with (abilities!, statics!, equipment!, ...). `cargo kani list`
# says the same at the price of building every harness, gigabytes of goto
# files.
TABLES = {'abilities', 'declares', 'werewolves', 'mana', 'statics', 'check_land'}
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
def list_harnesses(src):
    out = []
    for f in sorted(glob.glob(src + '/proofs/**/*.rs', recursive=True)):
        rel = os.path.relpath(f, src)[:-3].split('/')
        mod = [p for p in rel if p != 'mod']
        path = '::'.join(mod)
        t = strip(open(f).read())
        for m in re.finditer(r'\b(card_proof|action_proof)!\s*\{', t):
            if t[m.start()-13:m.start()].endswith('macro_rules! '): continue
            body = t[m.end()-1:block(t, m.end()-1)+1]
            n = re.search(r'\bfn (\w+)\(\)', body)
            # A body with a `$` is a macro's own use of the macro, not a proof.
            if n and not body.lstrip('{').strip().startswith('(') and '$' not in body:
                out.append(path + '::' + n.group(1))
        for m in re.finditer(r'#\[kani::proof\][^\n]*\n(?:\s*#\[[^\n]*\n)*\s*(?:pub )?fn (\w+)', t):
            out.append(path + '::' + m.group(1))
        for m in re.finditer(r'\bfor_each_form!\s*\{', t):
            body = t[m.end()-1:block(t, m.end()-1)+1]
            n = re.search(r'\bfn (\w+)\(form\)', body)
            if n: out += [path + '::' + n.group(1) + '::of_a_card', path + '::' + n.group(1) + '::of_a_token']
        for name in TABLES | FORMS:
            for m in re.finditer(r'(?<!macro_rules! )\b' + name + r'!\s*\{', t):
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


def verify(h, goto, unwind, mangled, bound, timeout):
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
    for cmd in steps:
        p = run(cmd)
        if p.returncode != 0:
            return ('ERROR', '-', f'{os.path.basename(cmd[0])} failed: {p.stderr[-200:]}')
    rec = bounds(out, bound)
    unwindset = ','.join(['memcmp.0:33'] + [f'{r}:{bound}' for r in rec])
    cmd = [f'{K}/bin/cbmc'] + CBMC_FLAGS + ['--unwind', str(unwind or 7), '--unwindset', unwindset, out, '--verbosity', '9', '--json-ui']
    with open(out + '.cmd', 'w') as f:
        f.write(' '.join(f"'{c}'" if ',' in c or '*' in c else c for c in cmd) + '\n')
    start = time.time()
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as e:
        partial = e.stdout.decode(errors='replace') if isinstance(e.stdout, bytes) else (e.stdout or '')
        m = re.search(r'size of program expression: (\d+) steps', partial)
        sym = re.search(r'Runtime Symex: ([0-9.]+)s', partial)
        where = (f' {m.group(1)} steps symex {float(sym.group(1)):.0f}s, then solving' if m and sym
                 else ' still in symbolic execution')
        return ('TIMEOUT', f'{time.time() - start:.0f}s', f'{len(rec)} bounded;{where}')
    took = f'{time.time() - start:.0f}s'
    if p.returncode in (-9, 137):
        return ('OOM', took, f'{len(rec)} bounded')
    try:
        doc = json.loads(p.stdout)
    except json.JSONDecodeError:
        return ('ERROR', took, (p.stdout[-300:] + p.stderr[-300:]).replace('\n', ' '))
    failed, covers, covered = [], 0, 0
    for item in doc:
        for r in item.get('result', []):
            prop, status = r.get('property', ''), r.get('status')
            if '.reachability_check.' in prop:
                continue
            if '.cover.' in prop:
                covers += 1
                covered += status == 'FAILURE'
                continue
            if status == 'FAILURE':
                failed.append(r.get('description', prop))
        if 'messageText' in item and 'Invalid User Input' in item.get('messageText', ''):
            failed.append(item['messageText'])
    errors = [i.get('messageText', '') for i in doc if i.get('messageType') == 'ERROR']
    size = ''
    for i in doc:
        t = i.get('messageText', '')
        if t.startswith('size of program expression'):
            size += f" {t.split(':')[1].split()[0]} steps"
        elif t.startswith('Runtime Symex'):
            size += f" symex {float(t.split(':')[1].strip().rstrip('s')):.0f}s"
        elif ' variables, ' in t and 'clauses' in t and 'clauses' not in size:
            size += f" {t.split(',')[1].split()[0]} clauses"
        elif t.startswith('Runtime decision procedure') and 'solve' not in size:
            size += f" solve {float(t.split(':')[1].strip().rstrip('s')):.0f}s"
    if errors and not failed:
        return ('ERROR', took, ' '.join(errors)[:300])
    cov = (f' covers {covered}/{covers}' if covers else '') + size
    if failed:
        return ('FAILED', took, f'{len(rec)} bounded{cov}; ' + ' | '.join(sorted(set(failed)))[:400])
    return ('SUCCESSFUL', took, f'{len(rec)} bounded{cov}')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('harness', nargs='*')
    ap.add_argument('--results', default=f'{ROOT}/logs/kani/results.txt')
    ap.add_argument('--crate', default=f'{ROOT}/mtg-engine')
    ap.add_argument('--target', default=os.environ.get('CARGO_TARGET_DIR', f'{ROOT}/target'))
    ap.add_argument('--batch', type=int, default=8)
    ap.add_argument('--timeout', type=int, default=1800)
    ap.add_argument('--bound', type=int, default=3)
    a = ap.parse_args()
    os.makedirs(os.path.dirname(a.results), exist_ok=True)
    every = list_harnesses(f'{a.crate}/src')
    # A name selects a harness by its full path or its last part, or every
    # harness in a module by the module's name (`vec_map`, `cards::auras`).
    wanted = [h for h in every if not a.harness
              or any(h == w or h.endswith('::' + w) or f'::{w}::' in f'::{h}' for w in a.harness)]
    done = set()
    if os.path.exists(a.results):
        done = {l.split()[2] for l in open(a.results) if len(l.split()) > 2}
    todo = [h for h in wanted if h not in done]
    for i in range(0, len(todo), a.batch):
        batch = todo[i:i + a.batch]
        found = codegen(batch, a.crate, a.target)
        with open(a.results, 'a') as res:
            if found is None:
                for h in batch:
                    res.write(f"{'NOT_RUN':10s} {'-':>8s}  {h}  codegen failed\n")
                continue
            for h in batch:
                if h not in found:
                    res.write(f"{'NOT_RUN':10s} {'-':>8s}  {h}  no goto file\n"); res.flush(); continue
                goto, unwind, mangled = found[h]
                result, took, why = verify(h, goto, unwind, mangled, a.bound, a.timeout)
                res.write(f'{result:10s} {took:>8s}  {h}  {why}\n'); res.flush()
                print(f'{result:10s} {took:>8s}  {h}', flush=True)
                base = goto[:-len('.symtab.out')] if goto.endswith('.symtab.out') else goto[:-len('.out')]
                if os.path.exists(base + '.out') and not os.environ.get('KANI_BOUNDED_KEEP'):
                    os.remove(base + '.out')


if __name__ == '__main__':
    main()

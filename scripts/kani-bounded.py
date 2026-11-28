#!/usr/bin/env python3
"""scripts/kani-bounded.py [NAME...] [--list FILE] [--results FILE] [--batch N] [--timeout S] [--bound B]
                          [--as-built] [--start-unwind U] [--loops FILE] [--jobs J --mem-cap GB]
                          [--memory-checks] [--shard K/N --times FILE] [--expect-open FILE]
                          [--skip-not-run | --only-not-run]

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
it: reachability checks are not failures, and covers are counted. A
harness none of whose own `assert!`s is reached is VACUOUS: Kani calls it
successful, with every one of them UNREACHABLE. One that reaches some and
not others has its result say which it did not (`not reached: ...`), to be
read against the case it is: an assertion in a branch another case of the
same family takes is expected there, and its own is not.

CBMC's pointer and bounds checks are off (`NO_MEMORY_CHECKS` says why);
--memory-checks keeps them.

Before cbmc, `scripts/static-constants.py` reads the goto program for a
constant Kani lays out over a static (its allocation table shares one
global between a constant and a static of the same initial bytes, so the
constant reads whatever the program last stored there):
`mtg-engine/src/proofs/held.rs` says how the proofs keep their statics
from it. A harness that has one is an ERROR, not run.

With --jobs J, J harnesses of a batch are verified at once, each cbmc
held to GB gigabytes of address space; one that runs out of it is run
again alone, once the rest are done, held to the --solo-cap (12): with
none, one that ran away took the machine, this driver with it. With one
at a time, each is held to the --solo-cap. Kept to one at a time
by default: two solvers sharing memory the kernel runs out of are both
at risk, not only the one that needed it.

A NAME is a harness's path or last part, or a module (`vec_map`,
`cards::auras`) for every harness in it; with none, every harness the
proofs define. --list FILE adds the names in FILE, one a line (`#` starts
a comment). One line per harness
goes to RESULTS (default logs/kani/results.txt):
    RESULT  SECONDS  NAME  [bounds, covers, size, or what failed]

With --shard K/N, only the Kth of N shares of the harnesses selected,
shared out by how long each took in --times FILE (a RESULTS file of an
earlier run; a harness not in it is taken as two minutes), the longest
first, each to the share with least so far: the scheduled run of every
harness in CI is N jobs, each held to six hours.

The exit status is 1 if any harness selected (in this shard) has no
result in RESULTS, or a last result other than SUCCESSFUL, unless it is
named in --expect-open FILE (the harnesses known not to prove, in the
form of --list); 0 otherwise. A harness expected open that proved is
said so, for the list to be pruned.

--times FILE also says which harnesses have been written and not yet run
to a result (NOT_RUN there). --skip-not-run leaves them out, as the
scheduled run does; --only-not-run selects only them, and then the exit
status is 1 only if one of them still has no result: what each one's
first result is, is what such a run is for.
"""
import argparse, collections, concurrent.futures, fcntl, glob, json, os, re, subprocess, sys, tempfile, threading, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
K = os.path.expanduser('~/.kani/kani-0.68.0')
KANI_LIB = f'{K}/library/kani/kani_lib.c'

# Every harness the proofs define, read off their source: a #[kani::proof]
# fn, a card_proof! or action_proof!, a for_each_form! (one harness per
# form), and the tables and forms the proof modules declare whole sets of
# harnesses with (abilities!, statics!, equipment!, ...). `cargo kani list`
# says the same at the price of building every harness, gigabytes of goto
# files.
TABLES = {'abilities', 'declares', 'werewolves', 'mana', 'statics', 'check_land', 'choosing_at_random', 'for_each_size', 'from_each_zone', 'for_each_key', 'each_printed', 'by_each_way', 'each_target', 'each_board', 'each_cost', 'each_pruning', 'each_path', 'over_each_library', 'over_each_hand', 'case_by_case', 'each_shape', 'each_hand_of_discard', 'each_pair', 'each_length', 'each_forest', 'each_player', 'each_holding', 'each_effect', 'each_count', 'each_setting', 'each_source', 'each_resolution', 'each_damage', 'each_filter', 'each_restriction', 'each_trigger_event', 'each_push', 'each_declaration', 'each_recording', 'each_removal', 'each_enchant', 'each_expiry', 'each_block', 'each_block_recorded', 'each_order', 'each_combat', 'each_question', 'each_reply',
          }
FORMS = {'reaches_what_it_describes', 'holds_exactly_then', 'each_subtype_set', 'reaches_its_subtypes'}
# Tables whose every entry is a module of these harnesses.
SUFFIXED = {
    'each_size_and_player': ['for_the_first_player', 'for_the_second_player'],
    'each_pick_and_player': ['for_the_first_player', 'for_the_second_player'],
    'each_set': ['of_one_point', 'of_two_points', 'of_three_points'],
    'each_answer': ['when_they_replace_it', 'when_they_say_none', 'when_they_say_one', 'when_they_say_two'],
    'each_library_and_player': ['for_the_first_player', 'for_the_second_player'],
    'into_play_from_each_zone': ['for_the_first_player', 'for_the_second_player'],
    'each_activation_and_player': ['for_the_first_player', 'for_the_second_player'],
    'each_loyalty_offer_and_player': ['for_the_first_player', 'for_the_second_player'],
    'each_board_and_player': ['for_the_first_player', 'for_the_first_player_with_the_opponent_gone',
                              'for_the_second_player', 'for_the_second_player_with_the_opponent_gone'],
}
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
# that function (its pretty name, as `pretty_names` has it, spaces and all:
# a trait method's is `<Type as Trait>::method`) is unwound to n
# in each harness of the module (whose name holds the part, if one is
# given), the unwinding assertions on as everywhere, so a bound too small
# fails the proof. For a loop the solver cannot see the end of in symbolic
# execution, over a list whose length it leaves open: unwound to the start
# bound, each pass past the list's end is a pass over what it cannot rule
# out, and the damage queue's ran out of memory so. `<function>#<k>` bounds
# only that function's loop number k, as `goto-instrument --show-loops`
# numbers them, where its other loops need more. A `<function>` ending in
# `*` names every function whose pretty name starts with what comes before
# it: an iterator adapter's carries its closure's file, line and column,
# which a bound should not have to repeat.
LOOP_CAPS = {}


NOT_TABLES = {'card_proof', 'action_proof', 'damage_proof', 'effect_proof', 'for_each_form', 'equipment', 'aura'}


def proof_macros(src):
    """The macros of the proofs whose definition writes a harness for each
    row given it, whichever file defines them: a module's macro is its
    submodules' too (`through_the_stack/printed.rs`, `short.rs`), and a
    table looked for only in the file that defines its macro had its other
    rows dropped from every run without a word."""
    found = set()
    for f in glob.glob(src + '/proofs/**/*.rs', recursive=True):
        t = strip(open(f).read())
        found |= {m.group(1) for m in re.finditer(r'\bmacro_rules!\s*(\w+)\s*\{', t)
                  if re.search(r'card_proof!|kani::proof', t[m.end() - 1:block(t, m.end() - 1) + 1])}
    return found - FORMS - set(SUFFIXED) - NOT_TABLES


def list_harnesses(src):
    out = []
    tables = TABLES | FORMS | set(SUFFIXED) | proof_macros(src)
    for f in sorted(glob.glob(src + '/proofs/**/*.rs', recursive=True)):
        rel = os.path.relpath(f, src)[:-3].split('/')
        mod = [p for p in rel if p != 'mod']
        path = '::'.join(mod)
        raw = open(f).read()
        for m in re.finditer(r'(?m)^\s*// loop bound: (.+?) (\d+)(?: (\w+))?\s*$', raw):
            LOOP_CAPS.setdefault(path, []).append((m.group(1), int(m.group(2)), m.group(3) or ''))
        t = strip(raw)
        for m in re.finditer(r'\b(card_proof|action_proof|damage_proof)!\s*\{', t):
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
        # A harness written out inside a macro's definition is the macro's,
        # named by the table that uses it (below).
        macros = [(m.start(), block(t, m.end() - 1)) for m in re.finditer(r'\bmacro_rules!\s*\w+\s*\{', t)]
        for m in re.finditer(r'#\[kani::proof\][^\n]*\n(?:\s*#\[[^\n]*\n)*\s*(?:pub )?fn (\w+)', t):
            if not any(a <= m.start() <= b for a, b in macros):
                out.append(path + '::' + m.group(1))
        for m in re.finditer(r'\bfor_each_form!\s*\{', t):
            body = t[m.end()-1:block(t, m.end()-1)+1]
            n = re.search(r'\bfn (\w+)\(form\)', body)
            if n: out += [path + '::' + n.group(1) + '::of_a_card', path + '::' + n.group(1) + '::of_a_token']
        # Any other macro whose definition writes a proof is a table too
        # (`proof_macros`).
        # In one order every time: a set's order is another in each
        # process, and the files written from this list were written in
        # another order by each run.
        for name in sorted(tables):
            for m in re.finditer(r'(?<!macro_rules! )\b' + name + r'!\s*[{(]', t):
                body = t[m.end()-1:block(t, m.end()-1)+1]
                for e in re.finditer(r'(?m)^\s{4}(\w+):', body):
                    if name in FORMS:
                        out += [path + '::' + e.group(1) + '::of_a_card', path + '::' + e.group(1) + '::of_a_token']
                    elif name in SUFFIXED:
                        out += [path + '::' + e.group(1) + '::' + x for x in SUFFIXED[name]]
                    else:
                        out.append(path + '::' + e.group(1))
        # A row of the Equipment table names two harnesses (the equip
        # ability's own target check, then what the card prints), or the
        # second alone for "another targeted ability"; a row of the Aura
        # table names one.
        for name, k in [('equipment', 2), ('aura', 1)]:
            for m in re.finditer(r'(?m)^' + name + r'!\(', t):
                body = t[m.end():block(t, m.end()-1)]
                parts = [p.strip() for p in body.split(',')]
                n = k
                if parts[0] == 'another targeted ability': parts, n = parts[1:], k - 1
                out += [path + '::' + p for p in parts[:n]]
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
# The harnesses whose programs are rewritten with `--wide-pointer-free`
# (`scripts/precise-enums.py` says what that is and why it is a flag): by
# the start of their names. The proofs that take a card through the
# engine's own cast, stack and resolution need it to be read at all; the
# rest keep the program their recorded result is of. KANI_WIDE_POINTER_FREE
# adds prefixes, comma-separated, to try it on others.
WIDE_POINTER_FREE = tuple(['proofs::through_the_stack::']
                          + [p for p in os.environ.get('KANI_WIDE_POINTER_FREE', '').split(',') if p])


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


# Held while compiling, by every run on the machine: Kani's compiler took
# seven gigabytes for a batch of thirty card harnesses, and three runs
# compiling at once beside their solvers had the kernel stop it, which
# recorded every harness of its batch as not run.
CODEGEN_LOCK = os.path.join(tempfile.gettempdir(), 'kani-bounded-codegen.lock')
# On a machine with the memory for it, KANI_BOUNDED_CODEGEN_SLOTS=N lets N
# runs compile at once (scripts/prove.sh kani-shares, each run building in
# its own target directory).
CODEGEN_SLOTS = max(1, int(os.environ.get('KANI_BOUNDED_CODEGEN_SLOTS', '1')))


def codegen_slot():
    """An open file holding one of the machine's compile slots, locked."""
    while True:
        for i in range(CODEGEN_SLOTS):
            lock = open(CODEGEN_LOCK + (f'.{i}' if i else ''), 'w')
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | (fcntl.LOCK_NB if CODEGEN_SLOTS > 1 else 0))
                return lock
            except BlockingIOError:
                lock.close()
        time.sleep(2)


def codegen(harnesses, crate, target):
    args = ['cargo', 'kani', '-Z', 'stubbing', '-Z', 'unstable-options', '--only-codegen', '--exact']
    for h in harnesses:
        args += ['--harness', h]
    env = dict(os.environ, CARGO_TARGET_DIR=target)
    with codegen_slot():
        p = run(args, cwd=crate, env=env)
        if p.returncode != 0 and 'Failed to canonicalize harness model' in p.stdout + p.stderr:
            # A cached build whose goto files were removed: build it again.
            os.utime(f'{crate}/src/lib.rs')
            p = run(args, cwd=crate, env=env)
    if p.returncode != 0:
        sys.stderr.write(p.stdout[-3000:] + p.stderr[-3000:])
        if len(harnesses) > 1 and 'SIGKILL' in p.stdout + p.stderr:
            # Stopped for want of memory: Kani's compiler took 8.4
            # gigabytes for eight harnesses that each resolve a choice.
            # Each half alone, so that a batch too big costs a retry, not
            # its harnesses.
            half = len(harnesses) // 2
            first = codegen(harnesses[:half], crate, target)
            second = codegen(harnesses[half:], crate, target)
            if first is None and second is None:
                return None
            return {**(first or {}), **(second or {})}
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
    if not os.path.exists(symtab):
        # goto-cc links what it is given that exists and says nothing of
        # what does not: the failure was then "main symbol not found".
        return ('ERROR', '-', f'the build left no goto file for it: {os.path.basename(symtab)}')
    steps = [
        [f'{K}/bin/goto-cc', symtab, KANI_LIB, '-o', out],
        [f'{K}/bin/goto-cc', out, '--function', mangled, '-o', out],
        [f'{K}/bin/goto-instrument', '--add-library', '--no-malloc-may-fail', out, out],
        [f'{K}/bin/goto-instrument', '--generate-function-body-options', 'assert-false-assume-false',
         '--generate-function-body', '.*', '--drop-unused-functions', out, out],
        [f'{K}/bin/goto-instrument', '--ensure-one-backedge-per-target', out, out],
    ]
    if precise:
        wide = ['--wide-pointer-free'] if h.startswith(WIDE_POINTER_FREE) else []
        steps.append([sys.executable, f'{ROOT}/scripts/precise-enums.py', out, out] + wide)
    for cmd in steps:
        p = run(cmd)
        if p.returncode != 0:
            return ('ERROR', '-', f'{os.path.basename(cmd[1] if cmd[0] == sys.executable else cmd[0])} failed: {p.stderr[-200:]}')
    shared = run([sys.executable, f'{ROOT}/scripts/static-constants.py', out])
    if shared.returncode != 0:
        lines = shared.stdout.strip().splitlines()
        return ('ERROR', '-', f'{lines[-1] if lines else shared.stderr[-200:]}: {lines[0] if len(lines) > 1 else ""}')
    rec = bounds(out, bound)
    by_name = pretty_names(out)
    full = unwind or 7
    caps = {}
    for module, fns in LOOP_CAPS.items():
        if h.startswith(module + '::'):
            loops_of = re.findall(r'^Loop (\S+):', run([f'{K}/bin/goto-instrument', '--show-loops', out]).stdout, re.M)
            for name, n, part in fns:
                if part in h.rsplit('::', 1)[-1]:
                    fn, _, k = name.partition('#')
                    named = ([m for p, ms in by_name.items() if p.startswith(fn[:-1]) for m in ms]
                             if fn.endswith('*') else by_name.get(fn, []))
                    for m in named:
                        caps.update({l: n for l in loops_of if (l == f'{m}.{k}' if k else l.startswith(m + '.'))})
    learned = load_loops(h)
    took, rounds = 0.0, 0
    while True:
        rounds += 1
        raised = sorted({f'{m}.{n}' for name, n in learned if name != 'memcmp' for m in by_name.get(name, [])} - set(caps))
        unwindset = ','.join(['memcmp.0:33'] + [f'{r}:{bound}' for r in rec] + [f'{l}:{full}' for l in raised]
                             + [f'{l}:{n}' for l, n in sorted(caps.items())])
        cmd = [f'{K}/bin/cbmc'] + CBMC_FLAGS + ([] if MEMORY_CHECKS else NO_MEMORY_CHECKS) + [
            '--unwind', str(min(START_UNWIND, full)), '--unwindset', unwindset, out, '--verbosity', '9'] + (['--json-ui'] if JSON_UI else [])
        with open(out + '.cmd', 'w') as f:
            f.write(' '.join(f"'{c}'" if ',' in c or '*' in c else c for c in cmd) + '\n')
        if cap:
            cmd = ['prlimit', f'--as={int(cap * 2**30)}', '--'] + cmd
        result, why, seconds, loops, others = solve(cmd, timeout, len(rec))
        took += seconds
        # Never `memcmp`'s: it has its own bound above, and learned, it was
        # unwound to the harness's (7) in every harness after, the later
        # entry overriding the earlier, so each comparing more than six
        # bytes failed.
        new = {l for l in loops - learned if l[0] != 'memcmp'}
        # Again with the loops that needed more than the starting bound at
        # the harness's own, until none does; a loop at the harness's bound
        # that still needs more, or any other failure, is the result.
        if result != 'FAILED' or not new or others:
            return (result, f'{took:.0f}s', why + (f' rounds {rounds}' if rounds > 1 else ''))
        learned |= new
        save_loops(learned, h)


LOOPS_LOCK = threading.Lock()


# The modules whose harnesses keep the loop bounds they learn in a file of
# their own, beside --loops: `loop-bounds.NAME.txt`. A loop learned is
# unwound to each harness's own bound in every harness after, and the
# proofs that go through the stack learn the loops of the state-based
# check and the trigger collectors, over a whole board's objects. Kept
# with the rest, those would be unwound further in every engine proof of
# those functions, whose recorded results are of the bounds they had.
OWN_LOOPS = {'proofs::through_the_stack::': 'through-the-stack'}


def own_loops_file(h):
    for prefix, name in OWN_LOOPS.items():
        if h.startswith(prefix):
            return os.path.join(os.path.dirname(LOOPS), f'loop-bounds.{name}.txt')
    return LOOPS


def load_loops(h=''):
    """The loops learned that apply to `h`: the shared ones and its module's own."""
    with LOOPS_LOCK:
        return read_loops(LOOPS) | read_loops(own_loops_file(h))


def read_loops(path):
    if not os.path.exists(path):
        return set()
    return {tuple(l.rsplit(' ', 1)) for l in open(path).read().splitlines() if l.strip()}


def save_loops(loops, h=''):
    # Merged with what is there: a harness verified alongside may have
    # raised loops of its own since this one read the file. Only what the
    # shared file does not hold goes into a module's own.
    with LOOPS_LOCK:
        path = own_loops_file(h)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        merged = read_loops(path) | (loops if path == LOOPS else loops - read_loops(LOOPS))
        with open(path, 'w') as f:
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
    # Plain output: whether the results have begun, the property whose
    # result has not ended yet, what cbmc says it reported (how many
    # properties, how many failed), a result line this could not read, and
    # the last few lines, which say why if cbmc stops on an error.
    in_results, open_result, said, unread = False, None, None, None
    tail = collections.deque(maxlen=6)

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
        # "Solver ran out ...": the SAT solver's own allocation failing,
        # which is the cap as much as CBMC's is.
        if 'ut of memory' in line or 'bad_alloc' in line or 'olver ran out' in line:
            oom = True
        if not JSON_UI:
            text = line.rstrip('\n')
            if open_result is not None:
                # A description that runs over lines: its result ends it.
                open_result[1] += ' ' + text
                m = RESULT_END.match(open_result[1])
                if m:
                    results.append((open_result[0], m.group(2), m.group(1)))
                    open_result = None
            elif text == '** Results:':
                in_results = True
            elif in_results and text.startswith('** '):
                m = re.match(r'\*\* (\d+) of (\d+) failed', text)
                said = (int(m.group(2)), int(m.group(1))) if m else None
                in_results = False
            elif in_results and text.startswith('['):
                m = RESULT_START.match(text)
                if not m:
                    unread = unread or text
                    continue
                rest = re.sub(r'^line \d+ ', '', m.group(2))
                end = RESULT_END.match(rest)
                if end:
                    results.append((m.group(1), end.group(2), end.group(1)))
                else:
                    open_result = [m.group(1), rest]
            elif STATISTIC.match(text) or 'Invalid User Input' in text:
                messages.append([text, None])
            elif text.strip() and not text.startswith('Unwinding loop '):
                tail.append(text.strip())
            continue
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
    # How far it got: symbolic execution's steps and time, and the
    # formula's clauses and the solver's time, as far as each was reached.
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
        return ('OOM', f"{nrec} bounded; over the cap{';' + size if size else ' in symbolic execution'}", seconds, set(), [])
    failed, others, loops, covers, covered = [], [], set(), 0, 0
    # Kani puts a reachability check, `assert(false)` under the check's id,
    # before each assertion: failing, it says the assertion is reached.
    reached = {desc for prop, status, desc in results
               if '.reachability_check.' in prop and status == 'FAILURE'}
    claims = claimed = 0
    unclaimed = []
    for prop, status, desc in results:
        if '.reachability_check.' in prop:
            continue
        m = re.match(r'\[(KANI_CHECK_ID_\S+)\] assertion failed', desc or '')
        if m and prop.startswith('proofs::') and '.assertion.' in prop:
            claims += 1
            if m.group(1) in reached:
                claimed += 1
            else:
                # Which of the proof's own assertions no execution reaches:
                # one in a branch another case of the same family takes is
                # expected, and one this case is there to check is not.
                what = ' '.join(desc.split('assertion failed', 1)[-1].lstrip(': ').split())
                unclaimed.append(f"{prop.rsplit('::', 1)[-1]} `{what[:70]}`")
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
    if not JSON_UI:
        # Read as text, the results are checked against cbmc's own count of
        # them: a result this did not read is not a result that passed.
        counted = (len(results), sum(status == 'FAILURE' for _, status, _ in results))
        if code not in (0, 10):
            errors = [f'cbmc exit {code}: ' + ' | '.join(tail)]
        elif unread or open_result or said != counted:
            errors = [f"cbmc's results not read: it says {said} (properties, failed), read {counted}"
                      + (f'; first unread: {unread[:120]}' if unread else '')]
            failed = []
    if errors and not failed:
        return ('ERROR', ' '.join(errors)[:300], seconds, set(), ['error'])
    if not results and not errors:
        return ('ERROR', f'no results (exit {code})', seconds, set(), ['error'])
    cov = ((f' covers {covered}/{covers}' if covers else '') + (f' reaches {claimed}/{claims}' if claims else '') + size
           + (f"; not reached: {'; '.join(unclaimed)[:600]}" if unclaimed else ''))
    if failed:
        return ('FAILED', f'{nrec} bounded{cov}; ' + ' | '.join(sorted(set(failed)))[:400], seconds, loops, others)
    if claims and not claimed:
        # Every `assert!` of the proof's own unreached: what it claims holds
        # of no execution, so it proved nothing (an assumption, a stub or an
        # unwinding bound that rules every case out).
        return ('VACUOUS', f'{nrec} bounded{cov}', seconds, set(), [])
    return ('SUCCESSFUL', f'{nrec} bounded{cov}', seconds, set(), [])


START_UNWIND, LOOPS, MEMORY_CHECKS = 4, None, False
# CBMC's output is read as plain text. As JSON (`--json-ui`) CBMC writes an
# execution trace for every property it reports as failed, asked for or
# not, and a Kani reachability check is reported so whenever its assertion
# is reached: one trace as long as the execution for each assertion the
# proof reaches. For a proof that plays a card through the engine's cast
# and stack that was 5.4 gigabytes of output and 427 seconds where the
# plain output is a megabyte and 97 (the same program, 2026-11-27).
# KANI_BOUNDED_UI=json reads the JSON as before.
JSON_UI = os.environ.get('KANI_BOUNDED_UI') == 'json'
# A result in the plain output, between `** Results:` and cbmc's count of
# them: `[property] line N description: STATUS`, the property's name ending
# in `.number` (or `.recursion`), the line left out for a property with no
# source line, and the description at times over lines.
RESULT_START = re.compile(r'^\[(.+?(?:\.\d+|\.recursion))\] (.*)$')
RESULT_END = re.compile(r'^(.*): (SUCCESS|FAILURE|UNREACHABLE|UNKNOWN|ERROR|NOT CHECKED|UNDETERMINED)$', re.S)
STATISTIC = re.compile(r'size of program expression|Runtime |\d+ variables, \d+ clauses')


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
    ap.add_argument('--reverse', action='store_true', help='run the harnesses last first, as a second run sharing the results')
    ap.add_argument('--list', action='append', default=[], help='a file of harness names, one a line')
    ap.add_argument('--shard', default=None, help='K/N: the Kth of N shares of the harnesses, by --times')
    ap.add_argument('--times', default=None, help='a results file whose seconds share out --shard')
    ap.add_argument('--expect-open', default=None, help='a file of the harnesses known not to prove')
    ap.add_argument('--skip-not-run', action='store_true',
                    help='leave out the harnesses --times records as NOT_RUN: written, and not yet run to a result')
    ap.add_argument('--only-not-run', action='store_true',
                    help='only the harnesses --times records as NOT_RUN; any result for each is the run\'s success')
    a = ap.parse_args()
    # As absolute paths. cargo is run in the crate's directory and reads a
    # relative target from there; this script reads the build's output
    # from its own. Given `--target target/kani-snap`, the two were
    # different directories: the build went to one, and the harness was
    # linked from what an earlier build had left in the other, so that a
    # probe of a change measured the code from before it (2026-11-28).
    a.crate, a.target = os.path.abspath(a.crate), os.path.abspath(a.target)
    for f in a.list:
        a.harness += read_names(f)
    global START_UNWIND, LOOPS, MEMORY_CHECKS
    START_UNWIND, LOOPS, MEMORY_CHECKS = a.start_unwind, a.loops, a.memory_checks
    os.makedirs(os.path.dirname(a.results), exist_ok=True)
    every = list_harnesses(f'{a.crate}/src')
    # A name selects a harness by its full path or its last part, or every
    # harness in a module by the module's name (`vec_map`, `cards::auras`).
    wanted = [h for h in every if not a.harness
              or any(h == w or h.endswith('::' + w) or f'::{w}::' in f'::{h}' for w in a.harness)]
    def done():
        # A harness with a result is done, but not one whose build failed:
        # a crate that did not compile, or a codegen killed for memory, is
        # the run's failure and not the harness's, and resuming runs it
        # again. Counted done, a batch built from a tree caught mid-edit
        # was every harness of it NOT_RUN for good.
        if not os.path.exists(a.results):
            return set()
        last = {}
        with open(a.results, errors='replace') as res:
            for l in res:
                p = l.split()
                if len(p) > 2:
                    last[p[2]] = l
        return {h for h, l in last.items() if not (l.startswith('NOT_RUN') and 'codegen failed' in l)}

    # A name that selects no harness is said so, in the results, not passed
    # over: a harness this script does not find is one no run proves.
    unknown = [w for w in a.harness
               if not any(h == w or h.endswith('::' + w) or f'::{w}::' in f'::{h}' for h in every)]
    if unknown:
        with open(a.results, 'a') as res:
            for w in unknown:
                res.write(f"{'NOT_RUN':10s} {'-':>8s}  {w}  not a harness this script finds\n")
        print(f'not harnesses this script finds: {" ".join(unknown)}', file=sys.stderr)
    if a.skip_not_run and a.only_not_run:
        ap.error('--skip-not-run and --only-not-run select nothing together')
    if (a.skip_not_run or a.only_not_run) and a.times:
        # A harness recorded as NOT_RUN has been written and not yet run to
        # a result on a machine that fits it (docs/plans/verification-
        # goals.md, goal 1). The scheduled run proves what has been proved;
        # such a harness joins it with its first result, which a run of
        # only these is for.
        waiting = not_run(a.times)
        wanted = [h for h in wanted if (h in waiting) == a.only_not_run]
    if a.shard:
        wanted = shard_of(wanted, a.shard, a.times)
    todo = [h for h in wanted if h not in done()]
    if a.reverse:
        todo.reverse()
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

    if a.only_not_run:
        return first_results(wanted, a.results)
    return verdict(wanted, a.results, read_names(a.expect_open) if a.expect_open else [])


def not_run(times):
    """The harnesses `times` records as NOT_RUN."""
    with open(times, errors='replace') as f:
        return {p[2] for p in (l.split() for l in f) if len(p) > 2 and p[0] == 'NOT_RUN'}


def read_names(path):
    """The harness names in `path`, one a line, `#` starting a comment."""
    with open(path) as f:
        return [n for n in (l.split('#', 1)[0].strip() for l in f) if n]


def shard_of(wanted, shard, times):
    """The `K/N`th share of `wanted`, by the seconds each took in `times`."""
    k, n = (int(x) for x in shard.split('/'))
    assert 1 <= k <= n, f'--shard {shard}: K from 1 to N'
    took = {}
    if times:
        with open(times, errors='replace') as f:
            for l in f:
                p = l.split()
                m = re.match(r'(\d+)s$', p[1]) if len(p) > 2 else None
                # A harness not yet run has no time to go by: two minutes,
                # as one the file does not name.
                if m and p[0] != 'NOT_RUN':
                    took[p[2]] = took.get(p[2], 0) + int(m.group(1))
    shares = [[0, []] for _ in range(n)]
    for h in sorted(wanted, key=lambda h: (-took.get(h, 120), h)):
        least = min(shares, key=lambda s: s[0])
        least[0] += took.get(h, 120)
        least[1].append(h)
    mine = set(shares[k - 1][1])
    print(f'shard {k}/{n}: {len(mine)} harnesses, about {shares[k - 1][0] / 3600:.1f} hours', flush=True)
    return [h for h in wanted if h in mine]


def first_results(wanted, results):
    """For a run of harnesses that had no result: each one's, counted by
    kind, and exit status 1 only if one still has none (the run broke, or
    its build did). A harness that did not prove is what the run was to
    find out, and not its failure."""
    last = {}
    if os.path.exists(results):
        with open(results, errors='replace') as res:
            for l in res:
                p = l.split()
                if len(p) > 2:
                    last[p[2]] = p[0]
    kinds = collections.Counter(last.get(h, 'NOT_RUN') for h in wanted)
    print(f'{len(wanted)} harnesses: ' + ', '.join(f'{n} {k}' for k, n in kinds.most_common()), flush=True)
    for h in wanted:
        if last.get(h, 'NOT_RUN') != 'SUCCESSFUL':
            print(f'{last.get(h, "no result"):10s} {h}', flush=True)
    return 1 if kinds['NOT_RUN'] else 0


def verdict(wanted, results, expected_open):
    """Exit status 1 if a harness of `wanted` has no result or did not prove
    and is not expected open, and say which."""
    last = {}
    if os.path.exists(results):
        with open(results, errors='replace') as res:
            for l in res:
                p = l.split()
                if len(p) > 2:
                    last[p[2]] = p[0]
    expected = set(expected_open)
    proved = [h for h in wanted if last.get(h) == 'SUCCESSFUL']
    open_still = [h for h in wanted if last.get(h) != 'SUCCESSFUL' and h in expected]
    broken = [h for h in wanted if last.get(h) != 'SUCCESSFUL' and h not in expected]
    pruned = [h for h in proved if h in expected]
    print(f'{len(wanted)} harnesses: {len(proved)} proved, {len(open_still)} expected open, {len(broken)} not proved', flush=True)
    for h in broken:
        print(f'NOT PROVED  {last.get(h, "no result"):10s} {h}', flush=True)
    for h in pruned:
        print(f'now proved, expected open: {h}', flush=True)
    return 1 if broken else 0


if __name__ == '__main__':
    sys.exit(main())

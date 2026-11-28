#!/usr/bin/env python3
"""kani-claims.py [--check | --count]: every line the Oracle prints on
every card of the pool, and the proofs through the game that claim it.

`scripts/kani/claims.tsv` has a row for each line of each face of each
card: the card's module, the face's name, the line as the Oracle has it
(`data/oracle_cache.json`), and what claims it. A face's first rows are
of its frame, which is not a line of its text and is as much the card:
`cost` and its mana cost, if it has one, and `type` and its type line
with its power and toughness or its loyalty.

A claim is one of:

- the name of a harness of `proofs::through_the_stack` whose card this
  is: a proof that plays the card through the engine and holds what comes
  of it against this line;
- `rule:NAME`, for a line that is a keyword: what the keyword does is the
  engine's and is proved of the engine once, for every card that has it,
  by the rows of `proofs::through_the_stack::rules` whose names begin
  with NAME (`flying_...`). The rule is proved when there is such a row
  and every one of them is;
- `-`: nothing claims the line yet;
- `reminder`: the line is reminder text and nothing but, which says again
  what a rule says and is no ability.

The rows `scripts/kani-printed-rows.py` writes claim what they are
written from, and this script writes those claims itself. Of a creature
card: its cost (cast for exactly that, and not for a mana less), its type
line (on the battlefield with those types and that power and toughness,
and cast at no other time than its own) and, with the keyword's rule,
each of its keyword lines (it is on the battlefield with those keywords).
Of a land: its type line (played, it is on the battlefield with those
types, and it is played once a turn, in its owner's own main phase).
Of a permanent with one mana ability that taps for given mana: that line
(tapped for it by the action the engine offers, and not when it may not
be), or, of a basic land, whose text has it only as a reminder, its type
line. Of an Equipment with "Equip" and a cost of mana: that line (equipped for
that cost, and not otherwise) and each line read whole as an Aura's.
Of an Aura: its cost and type line as a creature's; its first line (cast
at what that says it enchants it is attached to it, and it is not cast
at something else); and each line that is "Enchanted creature gets
+A/+B", "has" a keyword, or both and no more (the creature has them).
Of a spell: its cost and its flashback line, by the row that casts it a
mana short of each; and its type line and again its flashback line, by
the rows that cast it at another time than its caster's own main phase,
where an instant is cast and a sorcery is not. Every other claim is
written by hand, by whoever writes the proof, and kept when the file is
written again. A spell's cost
has only that one claim written here; that it is cast for exactly its
cost is every other row of the card, each of which gives its caster the
Oracle's cost and no more.

A line counts as claimed when it has a claim, and as proved when every
claim it has is of a proof whose last recorded result is SUCCESSFUL
(`scripts/kani/times.txt`). What is counted says how much of what the
cards print some proof speaks for. That a proof would notice the line's
code being wrong is not checked here: it is what mutating the code is
for.

With no argument, writes the file again from the Oracle and from the
claims it has. With --check, exits 1 if the file is not what that would
write (a card or a line the Oracle has and the file has not, or the
other way about), a claim names a harness that is not there or is
another card's, or a harness of `proofs::through_the_stack` claims no
line: a proof through the game is of something its card prints, and says
what. With --count, prints the three numbers.
"""
import importlib.util, json, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LEDGER = f'{ROOT}/scripts/kani/claims.tsv'
THROUGH = 'proofs::through_the_stack::'
HEAD = ('# Written by scripts/kani-claims.py from the Oracle\'s text; the claims of the fourth column are kept.\n'
        '# card\tface\tline\tclaims\n')
# The suffixes of the rows scripts/kani-printed-rows.py writes for a creature.
ENTERS = ('_cast_by_the_first_player_enters_as_printed', '_cast_by_the_second_player_enters_as_printed')
TIMES = ('_is_cast_in_the_other_players_turn', '_is_not_cast_in_the_other_players_turn',
         '_is_cast_in_its_casters_upkeep', '_is_not_cast_in_its_casters_upkeep')
SHORT = ('_is_not_cast_for_one_mana_less',)
# And of an Aura: cast at what it enchants, and not at what it does not.
ATTACHED = ('_cast_by_the_first_player_enters_attached_as_printed', '_cast_by_the_second_player_enters_attached_as_printed')
NOT_ENCHANTED = ('_is_not_cast_at_what_it_does_not_enchant',)
# And of an Equipment: equipped, and not when it may not be.
EQUIPPED = ('_is_equipped_to_its_controllers_creature',)
NOT_EQUIPPED = ('_is_not_equipped_to_the_other_players_creature', '_is_not_equipped_for_one_mana_less',
                '_is_not_equipped_in_the_other_players_turn', '_is_not_equipped_in_its_controllers_upkeep')
# And of a permanent with one mana ability: tapped for its mana, and not.
TAPPED = ('_taps_for_its_mana', '_taps_for_its_mana_in_the_other_players_turn', '_does_not_tap_for_mana_when_tapped',
          '_does_not_tap_for_mana_the_turn_it_arrives')
# And of a land: played as printed, and not played when it may not be.
LAND = ('_played_by_the_first_player_is_there_as_printed', '_played_by_the_second_player_is_there_as_printed',
        '_is_not_played_a_second_time_in_a_turn', '_is_not_played_in_the_other_players_turn', '_is_not_played_in_its_owners_upkeep')
# And of a card cast with flashback: a mana short, and at another time.
FLASHED_BACK = ('_is_not_flashed_back_for_one_mana_less',
                '_is_flashed_back_in_the_other_players_turn', '_is_not_flashed_back_in_the_other_players_turn',
                '_is_flashed_back_in_its_casters_upkeep', '_is_not_flashed_back_in_its_casters_upkeep')
WRITTEN = ENTERS + TIMES + SHORT + FLASHED_BACK + LAND + ATTACHED + NOT_ENCHANTED + EQUIPPED + NOT_EQUIPPED + TAPPED
# The rules the rows of proofs::through_the_stack::rules are of, each the
# beginning of its rows' names: the keywords, and what is no keyword's.
RULES = ('combat', 'turn', 'summoning_sickness', 'flying', 'reach', 'intimidate', 'first_strike', 'double_strike', 'deathtouch',
         'lifelink', 'indestructible', 'vigilance', 'haste', 'defender', 'hexproof', 'trample', 'menace')
OF_THE_RULES = 'the rules'


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), f'{ROOT}/scripts/{name}.py')
    module = importlib.util.module_from_spec(spec)
    saved, sys.argv = sys.argv, [f'{name}.py']
    spec.loader.exec_module(module)
    sys.argv = saved
    return module


def frame(face):
    """A face's frame: its cost, if it has one, and its type line with
    its numbers, each as (kind, line)."""
    if face.get('mana_cost'):
        yield 'cost', f'cost {face["mana_cost"]}'
    line = f'type {face.get("type_line") or ""}'
    if face.get('power') is not None:
        line += f' {face["power"]}/{face["toughness"]}'
    if face.get('loyalty') is not None:
        line += f', loyalty {face["loyalty"]}'
    yield 'type', line


def printed(rows_module):
    """(module, face name, line, kind, keywords of the line) for every
    line of every face of every card of the pool, a face's frame first.
    The kind is `cost` or `type` for a frame's line and `text` for a line
    of the text. A back face's line has no keywords: a card is cast as
    its front face, and the rows of its casting say nothing of its back."""
    oracle = json.load(open(f'{ROOT}/data/oracle_cache.json'))['cards']
    for module, _, name, _ in rows_module.pool():
        card = oracle.get(name)
        if card is None:
            sys.exit(f'{module}: the Oracle cache has no card named {name!r}; scripts/oracle_lookup.py fetch it')
        for face in [card] + ([card['back_face']] if card.get('back_face') else []):
            aura = rows_module.aura_reads(face) if face is card and 'Aura' in (face.get('type_line') or '') else (None, None, [], [])
            equipment = rows_module.equipment_reads(face) if face is card and 'Equipment' in (face.get('type_line') or '') else None
            tap = rows_module.mana_reads(face) if face is card else None
            for kind, line in frame(face):
                # A basic land's mana ability is its land type's (CR
                # 305.6), and its text has it only as a reminder.
                yield module, face['name'], line, 'type and mana' if kind == 'type' and tap and tap[0].startswith('(') else kind, []
            for n, line in enumerate(l.strip() for l in (face.get('oracle_text') or '').split('\n') if l.strip()):
                if line in aura[3]:
                    # A line of an Aura its rows read whole: what it
                    # enchants, or what it gives the creature.
                    yield module, face['name'], line, 'enchant' if n == 0 else 'aura', aura[2] if n else []
                elif tap and line == tap[0] and not line.startswith('('):
                    yield module, face['name'], line, 'mana', []
                elif equipment and equipment[0] is not None and line == equipment[1]:
                    yield module, face['name'], line, 'equip', []
                elif equipment and equipment[0] is not None and line in equipment[4]:
                    yield module, face['name'], line, 'equipped', [k for k in equipment[3] if k.lower() in re.sub(r'\s', '', line).lower()]
                else:
                    yield module, face['name'], line, 'text', rows_module.own_keywords(line) if face is card else []


def harnesses():
    """The harnesses of proofs::through_the_stack, by card module: each
    one's own name, and its path, which is what a result is recorded by."""
    kb, ledger = load('kani-bounded'), load('kani-card-ledger')
    by_name = sorted(((n, m) for m, ns in ledger.cards().items() for n in ns), key=lambda x: -len(x[0]))
    found = {}
    for h in kb.list_harnesses(f'{ROOT}/mtg-engine/src'):
        if h.startswith(THROUGH):
            leaf = h.rsplit('::', 1)[1]
            card = next((m for n, m in by_name if leaf == n or leaf.startswith(n + '_')), None)
            if h.startswith(THROUGH + 'rules::'):
                card = OF_THE_RULES
            if leaf in found.setdefault(card, {}):
                sys.exit(f'two harnesses of proofs::through_the_stack are named {leaf}')
            found[card][leaf] = h
    return found


def rules(of):
    """Each rule's rows, by their paths: those of the rules' module whose
    names begin with the rule's."""
    by_rule = {rule: [] for rule in RULES}
    for leaf, path in of.get(OF_THE_RULES, {}).items():
        rule = max((r for r in RULES if leaf.startswith(r + '_')), key=len, default=None)
        if rule is None:
            sys.exit(f'{leaf}: a row of the rules whose name begins with no rule kani-claims.py knows ({", ".join(RULES)})')
        by_rule[rule].append(path)
    return by_rule


def kept():
    """The claims the file has, by (module, face, line)."""
    claims = {}
    if os.path.exists(LEDGER):
        for row in open(LEDGER):
            if not row.startswith('#') and row.strip():
                module, face, line, claim = row.rstrip('\n').split('\t')
                claims[module, face, line] = claim.split()
    return claims


def render(of=None):
    """The file's text, and its rows as (module, face, line, claims)."""
    of = of if of is not None else harnesses()
    rows_module, old, rows, seen = load('kani-printed-rows'), kept(), [], set()
    fronts = {}
    for module, face, line, kind, keywords in printed(rows_module):
        front = fronts.setdefault(module, face) == face
        key = (module, face, line)
        if key in seen:
            sys.exit(f'{module}: {face} prints the same line twice: {line[:60]}')
        seen.add(key)
        if re.fullmatch(r'\(.*\)', line):
            rows.append((module, face, line, ['reminder']))
            continue
        def rows_of(suffixes):
            return [module + s for s in suffixes if module + s in of.get(module, {})]

        if kind == 'cost':
            written = rows_of(ENTERS + ATTACHED + SHORT) if front else []
        elif kind in ('type', 'type and mana'):
            written = rows_of(ENTERS + ATTACHED + TIMES + LAND + (TAPPED if kind == 'type and mana' else ())) if front else []
        elif kind == 'mana':
            written = rows_of(TAPPED)
        elif kind == 'enchant':
            written = rows_of(ATTACHED + NOT_ENCHANTED)
        elif kind == 'equip':
            written = rows_of(EQUIPPED + NOT_EQUIPPED)
        elif kind == 'equipped':
            written = rows_of(EQUIPPED)
            if written:
                written += [f'rule:{re.sub(r"(?<!^)(?=[A-Z])", "_", k).lower()}' for k in keywords]
        elif kind == 'aura':
            # The creature has what the line gives it, a keyword among
            # it; what the keyword does is its rule's.
            written = rows_of(ATTACHED)
            if written:
                written += [f'rule:{re.sub(r"(?<!^)(?=[A-Z])", "_", k).lower()}' for k in keywords]
        elif line.startswith('Flashback '):
            written = rows_of(FLASHED_BACK)
        elif keywords:
            # Flash is when a card may be cast, which the rows that cast
            # it at other times than its own are the proof of; every
            # other keyword is a rule of the game's.
            written = rows_of(ENTERS + TIMES if 'Flash' in keywords else ENTERS)
            if written:
                written += [f'rule:{re.sub(r"(?<!^)(?=[A-Z])", "_", k).lower()}' for k in keywords if k != 'Flash']
        else:
            written = []
        by_hand = [c for c in old.get(key, []) if c not in ('-', 'reminder') and c not in written
                   and not c.startswith('rule:') and not c.endswith(WRITTEN)]
        rows.append((module, face, line, written + by_hand or ['-']))
    return HEAD + ''.join(f'{m}\t{f}\t{l}\t{" ".join(c)}\n' for m, f, l, c in rows), rows


def results():
    last = {}
    for line in open(f'{ROOT}/scripts/kani/times.txt', errors='replace'):
        p = line.split()
        if len(p) > 2 and not line.startswith('#'):
            last[p[2]] = p[0]
    return last


def numbers(rows=None, of=None):
    """Lines printed (reminder text aside), those claimed, those proved."""
    of = of if of is not None else harnesses()
    rows = rows if rows is not None else render(of)[1]
    last = results()
    lines = [(m, c) for m, _, _, c in rows if c != ['reminder']]
    claimed = [(m, c) for m, c in lines if c != ['-']]
    by_rule = rules(of)

    def is_proved(module, claim):
        if claim.startswith('rule:'):
            of_it = by_rule.get(claim[len('rule:'):], [])
            return bool(of_it) and all(last.get(p) == 'SUCCESSFUL' for p in of_it)
        return last.get(of.get(module, {}).get(claim)) == 'SUCCESSFUL'

    proved = [c for m, c in claimed if all(is_proved(m, h) for h in c)]
    return len(lines), len(claimed), len(proved)


def rule_numbers(of=None):
    """The rules with rows, those with every row proved, and the rows."""
    of = of if of is not None else harnesses()
    last, by_rule = results(), rules(of)
    with_rows = [r for r in RULES if by_rule[r]]
    proved = [r for r in with_rows if all(last.get(p) == 'SUCCESSFUL' for p in by_rule[r])]
    return len(RULES), len(with_rows), len(proved), sum(len(v) for v in by_rule.values())


def main():
    of = harnesses()
    text, rows = render(of)
    if sys.argv[1:] == ['--count']:
        print('%d lines printed, %d claimed by a proof, %d proved' % numbers(rows, of))
        return
    if sys.argv[1:] == ['--check']:
        if not os.path.exists(LEDGER) or open(LEDGER).read() != text:
            sys.exit('scripts/kani/claims.tsv is not what scripts/kani-claims.py writes: run it')
        bad = []
        for module, face, line, claims in rows:
            for c in claims:
                if c.startswith('rule:') and c[len('rule:'):] not in RULES:
                    bad.append(f'{module}: {c} claims "{line[:50]}" and is no rule kani-claims.py knows')
                if c in ('-', 'reminder') or c.startswith('rule:'):
                    continue
                if c not in of.get(module, []):
                    bad.append(f'{module}: {c} claims "{line[:50]}" and is not a harness of this card in proofs::through_the_stack')
        claiming = {(module, c) for module, _, _, claims in rows for c in claims}
        bad += [f'{module}: {h} claims no line of its card: say in scripts/kani/claims.tsv what it is a proof of'
                for module in of if module != OF_THE_RULES for h in of[module] if (module, h) not in claiming]
        rules(of)
        if bad:
            sys.exit('\n'.join(bad))
        return
    with open(LEDGER, 'w') as f:
        f.write(text)
    print('%d lines printed, %d claimed by a proof, %d proved' % numbers(rows, of))


if __name__ == '__main__':
    main()

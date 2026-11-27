#!/usr/bin/env python3
"""kani-printed-rows.py [--check]: write, from the Oracle's text, the two
files of `mtg-engine/src/proofs/through_the_stack/` no hand writes.

`oracle.rs` is the mana cost and the flashback cost the Oracle prints for
each card of the pool: what a proof gives a caster to cast a card with
(`a_spell`), so that the cost the engine asks is held against the printed
one and not against the engine's own data for the card.

`printed.rs` is, for each creature card, the rows of what casting it
through the engine comes to: for exactly its printed cost, by each player
in their own main phase, it is on the battlefield as its printed text says
(`enters_as_printed` in `through_the_stack.rs`); for one mana less it is
not cast; and in the other player's turn, or in its caster's upkeep, it is
cast if it has flash and not otherwise (`is_cast_only_at_its_time`).

What a row holds a card against is the Oracle's text for it
(`data/oracle_cache.json`, from Scryfall, which `scripts/oracle_lookup.py`
keeps), and nothing of the engine's own data for the card: its mana cost,
the card types and subtypes of its type line, its power and toughness, and
the keyword abilities it has. A card's own keywords are the ones on a line
of nothing but keywords ("Flying", "First strike, lifelink"), their
reminder text aside: a keyword it has only while something holds, or gives
to something else, is in a sentence and not on such a line, and Scryfall's
list of the keywords a card mentions has those too.

A card is left out, and said so at the foot of the file, when:
- the Oracle cache has no card of its name;
- its text has something happen as it enters (the word "enters"): what
  the game looks like once it has resolved is then that ability's proof
  to say;
- its cost has a symbol the proofs' mana does not know (X, hybrid), or it
  has none;
- its power or toughness is not a number.

With --check, exits 1 if the file is not what this would write: the rows
are the Oracle's, and are not edited by hand.
"""
import glob, json, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CARDS = f'{ROOT}/mtg-engine/src/cards'
OUT = f'{ROOT}/mtg-engine/src/proofs/through_the_stack/printed.rs'
COSTS = f'{ROOT}/mtg-engine/src/proofs/through_the_stack/oracle.rs'
COLORS = {'W': 'White', 'U': 'Blue', 'B': 'Black', 'R': 'Red', 'G': 'Green'}
# The engine's keywords (`types::Keyword`), by the words a card prints.
KEYWORDS = {'flying': 'Flying', 'first strike': 'FirstStrike', 'double strike': 'DoubleStrike', 'trample': 'Trample',
            'deathtouch': 'Deathtouch', 'lifelink': 'Lifelink', 'vigilance': 'Vigilance', 'flash': 'Flash', 'reach': 'Reach',
            'haste': 'Haste', 'defender': 'Defender', 'hexproof': 'Hexproof', 'intimidate': 'Intimidate', 'menace': 'Menace',
            'indestructible': 'Indestructible'}
TYPES = ('Land', 'Creature', 'Instant', 'Sorcery', 'Enchantment', 'Artifact', 'Planeswalker')


def pool():
    """Each card of the engine's pool: its module, the path of its type,
    the name its data gives it, and whether the engine has it a creature
    (asked only of a card the Oracle cache does not have)."""
    for f in sorted(glob.glob(f'{CARDS}/*.rs') + glob.glob(f'{CARDS}/isd/*.rs')):
        module = os.path.basename(f)[:-3]
        if module in ('mod', 'helpers'):
            continue
        src = open(f).read()
        struct = re.search(r'^pub struct (\w+)\s*;', src, re.M)
        name = re.search(r'name:\s*"([^"]+)"', src)
        if struct and name:
            sub = 'isd::' if '/isd/' in f else ''
            yield module, f'crate::cards::{sub}{module}::{struct.group(1)}', name.group(1), 'CardType::Creature' in src


def own_keywords(text):
    """The engine's keywords among those on lines of nothing but keywords."""
    found = []
    for line in text.split('\n'):
        parts = [p.strip().lower() for p in re.split(r'[,;]', re.sub(r'\([^)]*\)', '', line))]
        parts = [p for p in parts if p]
        # A keyword line may hold keywords the engine has no `Keyword` for
        # (protection, transform's reminder): only a line every part of
        # which is one of the engine's is read here.
        if parts and all(p in KEYWORDS for p in parts):
            found += [KEYWORDS[p] for p in parts]
    return found


def symbols(cost):
    """A printed cost's symbols as Rust, or the symbol that has none."""
    out = []
    for sym in re.findall(r'\{([^}]*)\}', cost or ''):
        if sym.isdigit():
            out.append(f'ManaSymbol::Generic({sym})')
        elif sym in COLORS:
            out.append(f'ManaSymbol::Colored(Color::{COLORS[sym]})')
        elif sym == 'X':
            out.append('ManaSymbol::X')
        else:
            return None, sym
    return out, None


def costs():
    """`oracle.rs`: each pool card's printed cost and flashback cost."""
    oracle = json.load(open(f'{ROOT}/data/oracle_cache.json'))['cards']
    cast, flashback = [], []
    for module, _, name, _ in pool():
        card = oracle.get(name)
        if card is None:
            sys.exit(f'{module}: the Oracle cache has no card named {name!r}; scripts/oracle_lookup.py fetch it')
        syms, odd = symbols(card.get('mana_cost'))
        if syms is None:
            sys.exit(f'{module}: its cost has {{{odd}}}, which the proofs have no mana symbol for')
        cast.append(f'        {json.dumps(name)} => &[{", ".join(syms)}],')
        for line in (card.get('oracle_text') or '').split('\n'):
            m = re.match(r'Flashback ((?:\{[^}]*\})+)(?: \(|$)', line)
            if m:
                syms, odd = symbols(m.group(1))
                if syms is None:
                    sys.exit(f'{module}: its flashback cost has {{{odd}}}')
                flashback.append(f'        {json.dumps(name)} => &[{", ".join(syms)}],')
            elif line.startswith('Flashback'):
                sys.exit(f'{module}: a flashback cost that is not only mana: {line[:60]}')
    return '\n'.join([
        '//! Written by `scripts/kani-printed-rows.py` from the Oracle\'s text',
        '//! (`data/oracle_cache.json`); not to be edited by hand. The mana cost',
        '//! and the flashback cost the Oracle prints for each card of the pool,',
        '//! by the card\'s name: what a proof gives a caster to cast it with.', '',
        'use crate::types::{Color, ManaSymbol};', '',
        '/// The mana cost printed on the card of this name; none for a card with',
        '/// no mana cost (a land).',
        'pub(super) fn cost(name: &str) -> &\'static [ManaSymbol] {',
        '    match name {'] + cast + [
        '        _ => panic!("the Oracle\'s costs have no card of this name"),',
        '    }', '}', '',
        '/// The flashback cost printed on the card of this name.',
        'pub(super) fn flashback_cost(name: &str) -> &\'static [ManaSymbol] {',
        '    match name {'] + flashback + [
        '        _ => panic!("the Oracle prints no flashback cost for a card of this name"),',
        '    }', '}', '']) + '\n'


def printed(card):
    """`card`'s row data as Rust, or why it has no row."""
    front = (card.get('type_line') or '').split('//')[0]
    types, _, subtypes = front.partition('—')
    if 'Creature' not in types.split():
        return None, None
    text = card.get('oracle_text') or ''
    if re.search(r'\benters\b', text):
        return None, 'its text has something happen as it enters'
    cost, odd = symbols(card.get('mana_cost'))
    if cost is None or 'ManaSymbol::X' in cost:
        return None, f'its cost has {{{odd or "X"}}}'
    if not cost:
        return None, 'it has no mana cost'
    power, toughness = str(card.get('power')), str(card.get('toughness'))
    if not re.fullmatch(r'-?\d+', power) or not re.fullmatch(r'-?\d+', toughness):
        return None, f'its power and toughness are {power}/{toughness}'
    return ('Printed { cost: &[%s], types: &[%s], subtypes: &[%s], power: %s, toughness: %s, keywords: &[%s] }' % (
        ', '.join(cost), ', '.join(f'CardType::{t}' for t in TYPES if t in types.split()),
        ', '.join(json.dumps(s) for s in subtypes.split()), power, toughness,
        ', '.join(f'Keyword::{k}' for k in own_keywords(text)))), None


def render():
    oracle = json.load(open(f'{ROOT}/data/oracle_cache.json'))['cards']
    rows, left = [], []
    for module, path, name, creature in pool():
        card = oracle.get(name)
        if card is None:
            if creature:
                left.append((module, 'the Oracle cache has no card of its name'))
            continue
        data, why = printed(card)
        if data is None:
            if why:
                left.append((module, why))
            continue
        for who, cast in (('first', 'FIRST'), ('second', 'SECOND')):
            rows.append(f'    {module}_cast_by_the_{who}_player_enters_as_printed: enters_as_printed((Box::new({path}), {data}, {cast}));')
        rows.append(f'    {module}_is_not_cast_for_one_mana_less: is_not_cast_for_one_mana_less((Box::new({path}), {data}));')
        flash = 'Keyword::Flash' in data
        for when, at in (('in_the_other_players_turn', 'At::TheOtherPlayersTurn'), ('in_its_casters_upkeep', 'At::ItsCastersUpkeep')):
            rows.append(f'    {module}_{"is" if flash else "is_not"}_cast_{when}: is_cast_only_at_its_time((Box::new({path}), {data}, {at}));')
    out = ['//! Written by `scripts/kani-printed-rows.py` from the Oracle\'s text',
           '//! (`data/oracle_cache.json`); not to be edited by hand. Each creature',
           '//! card of the pool whose text has nothing happen as it enters, cast by',
           '//! each player for its printed cost, is on the battlefield as printed',
           '//! (`enters_as_printed`).', '',
           'use super::*;', 'use crate::types::Color;', '',
           'through_the_stack! {'] + rows + ['}', '',
           f'// {len(rows)} rows for {len(rows) // 5} cards. Creature cards with no row here, and why:']
    out += [f'// - {module}: {why}' for module, why in left]
    return '\n'.join(out) + '\n'


def main():
    files = {OUT: render(), COSTS: costs()}
    if sys.argv[1:] == ['--check']:
        for path, text in files.items():
            if not os.path.exists(path) or open(path).read() != text:
                sys.exit(f'{os.path.relpath(path, ROOT)} is not what scripts/kani-printed-rows.py writes: run it')
        return
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    for path, text in files.items():
        with open(path, 'w') as f:
            f.write(text)
    print(f'{len(re.findall(r"(?m)^    [a-z0-9_]+: ", files[OUT]))} rows written to {os.path.relpath(OUT, ROOT)}, '
          f'{files[COSTS].count(" => &[")} costs to {os.path.relpath(COSTS, ROOT)}')


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""kani-printed-rows.py [--check]: write the rows of
`mtg-engine/src/proofs/through_the_stack/printed.rs`, a proof for each
creature card that, cast through the engine, it is on the battlefield as
its printed text says (`enters_as_printed` in `through_the_stack.rs`).

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


def printed(card):
    """`card`'s row data as Rust, or why it has no row."""
    front = (card.get('type_line') or '').split('//')[0]
    types, _, subtypes = front.partition('—')
    if 'Creature' not in types.split():
        return None, None
    text = card.get('oracle_text') or ''
    if re.search(r'\benters\b', text):
        return None, 'its text has something happen as it enters'
    symbols = re.findall(r'\{([^}]*)\}', card.get('mana_cost') or '')
    if not symbols:
        return None, 'it has no mana cost'
    cost = []
    for sym in symbols:
        if sym.isdigit():
            cost.append(f'ManaSymbol::Generic({sym})')
        elif sym in COLORS:
            cost.append(f'ManaSymbol::Colored(Color::{COLORS[sym]})')
        else:
            return None, f'its cost has {{{sym}}}'
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
    out = ['//! Written by `scripts/kani-printed-rows.py` from the Oracle\'s text',
           '//! (`data/oracle_cache.json`); not to be edited by hand. Each creature',
           '//! card of the pool whose text has nothing happen as it enters, cast by',
           '//! each player for its printed cost, is on the battlefield as printed',
           '//! (`enters_as_printed`).', '',
           'use super::*;', 'use crate::types::Color;', '',
           'through_the_stack! {'] + rows + ['}', '',
           f'// {len(rows)} rows for {len(rows) // 2} cards. Creature cards with no row here, and why:']
    out += [f'// - {module}: {why}' for module, why in left]
    return '\n'.join(out) + '\n'


def main():
    text = render()
    if sys.argv[1:] == ['--check']:
        if not os.path.exists(OUT) or open(OUT).read() != text:
            sys.exit(f'{os.path.relpath(OUT, ROOT)} is not what scripts/kani-printed-rows.py writes: run it')
        return
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, 'w') as f:
        f.write(text)
    print(f'{text.count("enters_as_printed((")} rows written to {os.path.relpath(OUT, ROOT)}')


if __name__ == '__main__':
    main()

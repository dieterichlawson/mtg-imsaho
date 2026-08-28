## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/87/army-of-the-damned?utm_source=api
**Type line**: `Sorcery` — {5}{B}{B}{B}
**Oracle text**:
```
Create thirteen tapped 2/2 black Zombie creature tokens.
Flashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

Thirteen 2/2 black Zombie tokens with their subtype, created tapped. The tap is applied after creation rather than as an entering replacement; nothing in this set watches an entering creature's tapped state, so it is not observable here — noted rather than changed.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/87/army-of-the-damned?utm_source=api
**Type line**: `Sorcery` — {5}{B}{B}{B}
**Oracle text**:
```
Create thirteen tapped 2/2 black Zombie creature tokens.
Flashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Create **thirteen tapped** 2/2 black Zombie creature tokens" — thirteen, and
  each enters tapped: PASS
- The tokens carry colour and the Zombie subtype, so Endless Ranks of the Dead
  and Undead Alchemist see them as Zombies: PASS
- Flashback {7}{B}{B}{B}, a sorcery, so sorcery timing applies to the flashback
  too: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The thirteen tapped tokens and the flashback: `cards_flashback.rs`, `subtype.rs`

## Audit — 2026-08-28 18:37

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Army of the Damned"`, https://scryfall.com/card/isd/87/army-of-the-damned
**Oracle text**:
```
Create thirteen tapped 2/2 black Zombie creature tokens.
Flashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Sorcery
**Mana cost**: {5}{B}{B}{B}   **Keywords**: Flashback
**Rulings**: 6, all the generic flashback ones.
**Status**: ISSUE (the card is correct; three tests were asserting nothing)

### Code issues
No issues found in `mtg-engine/src/cards/isd/army_of_the_damned.rs`.

`{5}{B}{B}{B}`, `CardType::Sorcery`, `flashback_cost: Some({7}{B}{B}{B})`, oracle text verbatim,
no target requirement. Thirteen calls to `create_token_with_subtypes` with 2/2, `Color::Black`,
`CardType::Creature`, `subtypes: ["Zombie"]`, and `controller_of` for the controller.

"**Tapped**" goes through `state.arrives_tapped`, not `state.tap` — the tokens arrive tapped and
nothing tapped them (CR 614.1c), so no `Tapped` event is emitted. That is the distinction the
two tap verbs exist for and this card gets it right.

### The tests were the problem

Three loops across the suite filtered tokens by a name no token has. CR 111.4 derives a token's
name from its subtypes — "Zombie Token", "Spider Token" — and each loop said `o.name ==
"Zombie"` or `"Spider"`, so it **iterated over nothing** and every assertion inside it passed
vacuously:

- `cards_spells_and_enchantments.rs:57` — "Zombie tokens should enter tapped", plus their P/T.
  Tapped is the only interesting word in this card's text, and it was untested.
- `token_copy.rs:78` — a test whose *entire subject* is that the doubled tokens are tapped too.
- `cards_evasion_and_graveyard_pt.rs:276` — Spider Spawning's Spiders being 1/2.

Each is now bound to a count first, so an empty set fails loudly rather than passing quietly,
and each gained the assertions the card's text supports that were missing (colour, subtype,
reach). Mutation-checked: not tapping the tokens now fails both Zombie tests, and making them
white fails the one that checks colour. Before the fix, neither mutation failed anything.

A fourth filter, `token_copy.rs:59`, looked like the same bug and is not: a token *copy* takes
the copied card's name, so `"Splinterfright"` is right, and it asserts its set is non-empty.

### Tricky interactions checked
- **Thirteen, and all thirteen tapped**: PASS.
- **2/2 black Zombie creature tokens**: PASS.
- **Token doubling**: PASS — each token goes through the CR 614 replacement individually, and
  the doubled ones are tapped too, which is what `token_copy.rs` is for.
- **Thirteen creatures entering at once**: thirteen `EnteredBattlefield` events, so a watcher
  like Mentor of the Meek (power 2 or less) triggers thirteen times. Not tested here.
- **Sorcery timing**, and the flashback cast is a sorcery too: engine-side.
- **Cast via flashback, then exiled**: engine-side, tested generically.

### Test coverage
- thirteen tapped 2/2 black Zombies:
  `cards_spells_and_enchantments.rs:57 army_of_the_damned_creates_13_tapped_zombies` (repaired)
- the doubled tokens are tapped as well: `token_copy.rs:78` (repaired)
- flashback: the generic offering/exile tests in `flashback.rs`

### Changes made
- `cards_spells_and_enchantments.rs`, `token_copy.rs`, `cards_evasion_and_graveyard_pt.rs`:
  the three vacuous loops corrected, each bound to a count. No code change — the card was right
  all along; nothing had ever checked it.

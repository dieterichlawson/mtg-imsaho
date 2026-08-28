## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/59/hysterical-blindness?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Creatures your opponents control get -4/-0 until end of turn.
```
**Status**: PASS

### Code issues
No issues found.

- "Creatures your opponents control get -4/-0 until end of turn" — snapshots the
  affected creatures at resolution and pushes one per-target `ModifyPT`, which is
  what CR 611.2c requires of a spell. This card had it right where four others in
  the set did not.
- -4/-0, not -4/-4: power only.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/59/hysterical-blindness?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Creatures your opponents control get -4/-0 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.

Two rulings, both satisfied. "Only creatures controlled by your opponent when
[it] resolves will get -4/-0" — the ids are collected into a `Vec` at resolve
time. "The effect will continue to apply to a creature even if you ... gain
control of that creature later in the turn" — the effect is
`ModifyPT { target: id }`, keyed on the object, so a control change does not
detach it.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`snapshot_anthems.rs` — the set is fixed at resolution.

## Audit — 2026-08-28 19:06

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Hysterical Blindness"`, https://scryfall.com/card/isd/59/hysterical-blindness
**Oracle text**:
```
Creatures your opponents control get -4/-0 until end of turn.
```
**Type line**: Instant
**Mana cost**: {2}{U}
**Rulings** (2, 2011-09-22):
- "Only creatures controlled by your opponent when Hysterical Blindness resolves will get -4/-0.
  Creatures that enter or that an opponent gains control of later in the turn will be unaffected."
- "Hysterical Blindness's effect will continue to apply to a creature even if you (or a
  teammate) gains control of that creature later in the turn."
**Status**: PASS (both rulings gained tests; neither had one)

### Code issues
No issues found in `mtg-engine/src/cards/isd/hysterical_blindness.rs`.

`{2}{U}`, `CardType::Instant`, oracle text verbatim, no target requirement ("creatures your
opponents control" targets nothing).

`on_resolve` snapshots opponents' creatures **by id** at resolution and pushes a
`ModifyPT { -4, 0 }` per id into `until_end_of_turn`. That one design decision is both rulings:
- the set never grows (CR 611.2c), so a creature entering later is unaffected;
- and the effect is keyed to the creature, not to who controls it, so stealing a debuffed
  creature does not shake the debuff off.

### Tricky interactions checked
- **-4/-0, toughness untouched**: PASS — a 5/5 becomes 1/5, and a 3-toughness creature does not
  die to it (power reduction is not damage).
- **Your own creatures untouched; a creature entering later untouched**: PASS.
- **You steal a debuffed creature (Traitorous Blood)** — reachable in this pool: PASS, newly
  tested through a real cast, so the control change goes through the engine.
- **Power below zero**: `effective_power` arithmetic; a 2/2 at -4/-0 deals no combat damage.
  Engine-wide behaviour, not this card's.
- **Multiple opponents**: "your opponents" plural — the filter is `!= controller`, which is
  every other player. Two-player pool, so equivalent.
- **Expires at end of turn**: `until_end_of_turn`, cleared by cleanup.

### Test coverage
- the debuff, its scope, and ruling 1's late-arrival clause:
  `cards_vanilla_and_keywords.rs:71 hysterical_blindness_debuffs_opponents` (extended)
- ruling 2, stealing the debuffed creature:
  `cards_vanilla_and_keywords.rs:~95 hysterical_blindness_follows_a_creature_you_steal` (NEW)

Mutation-checked: -3 instead of -4 fails both; dropping the opponents filter fails the scope
test (your own 2/2 would read as -2). The steal test survives that second mutation — correctly,
since a superset still debuffs its subject.

### Changes made
- `cards_vanilla_and_keywords.rs`: the late-arrival assertion and the Traitorous Blood test.
  No code change.

## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/246/shimmering-grotto?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{1}, {T}: Add one mana of any color.
```
**Status**: PASS

### Code issues
No issues found.

- Both abilities are mana abilities under CR 605.1a, so both are visible to the
  auto-tap planner; the colored one carries its `{1}` in `ManaAbilityDef::cost`
  rather than being hidden in `activated_abilities`.
- "one mana of any color" is five entries, one per color, so the player picks a
  colour by picking an ability.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/246/shimmering-grotto?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{1}, {T}: Add one mana of any color.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Two separate mana abilities: "{T}: Add {C}" and "{1}, {T}: Add one mana of any
  colour" — the second costs mana as well as the tap, so it is a filter rather
  than a ramp: PASS
- Both use the tap, so only one can be activated: PASS
- "one mana of **any color**" presents a colour choice rather than assuming one:
  PASS
- A mana ability does not use the stack (CR 605.1a): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both abilities and the colour choice: `mana_ability_offers.rs`, `cards_lands_and_mana_sources.rs`

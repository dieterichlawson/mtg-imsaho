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

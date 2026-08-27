## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/247/stensia-bloodhall?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Like other lands, Stensia Bloodhall is colorless. The damage it deals
  is from a colorless source, even though activating its ability requires
  colored mana." The damage source is the land object, so protection from black
  or red does not stop it: PASS
- "target player **or planeswalker**" — `TargetRequirement::PlayerOrPlaneswalker`,
  so it cannot hit a creature: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage ability: `cards_activated_abilities.rs`
- Its mana ability is still offered alongside: `mana_ability_offers.rs`

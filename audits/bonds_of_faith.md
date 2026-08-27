## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/5/bonds-of-faith?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{W}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "gets +2/+2 as long as it's a Human. **Otherwise**, it can't attack or block" —
  three conditional continuous effects: `AttachedHasSubtype("Human")` for the
  pump, `AttachedLacksSubtype("Human")` for both restrictions, so the two halves
  are mutually exclusive by construction: PASS
- Ruling: "Once the enchanted creature has been declared as an attacking or
  blocking creature, causing it to stop being a Human won't remove it from
  combat. It will lose the +2/+2 bonus, however." The P/T is re-evaluated live;
  the attack restriction is only consulted at declaration: PASS
- A Human Werewolf that transforms into a non-Human back face loses the pump and
  gains the restrictions in real time: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both halves and the transform interaction: `enchantments.rs`, `moonmist.rs`, `subtype.rs`

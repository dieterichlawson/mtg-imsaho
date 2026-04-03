# Audit: Gruesome Deformity

## Oracle Reference (Scryfall)
- Cost: {B}
- Type: Enchantment -- Aura
- Oracle: "Enchant creature
  Enchanted creature has intimidate."

## Implementation: gruesome_deformity.rs

## Issues Found

No issues found. Name, cost ({B}), type (Enchantment), subtype (Aura), oracle text, target requirement (Creature), and continuous effect (GrantKeyword Intimidate with EffectScope::Attached) all match.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Enchant creature
Enchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)
```

### Findings
- Name, cost ({B}), type (Enchantment -- Aura) all match.
- Oracle text in code: "Enchanted creature has intimidate." -- correct (reminder text omission is standard).
- Target requirement: Creature -- correct.
- Grants Intimidate via ContinuousEffect::GrantKeyword to Attached scope -- correct.
- Resolves via resolve_aura helper -- correct.

### Verdict: PASS

---

## Audit — 2026-04-02 21:12
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/103/gruesome-deformity), cached 2026-04-01
**Oracle text**: Enchant creature\nEnchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)
**Type line**: Enchantment — Aura

**Status**: PASS

### Code issues
None found. All card data fields (name, cost {B}, type Enchantment, subtype Aura, oracle text, target requirement Creature, continuous effect GrantKeyword Intimidate with EffectScope::Attached) match the oracle text. The on_resolve uses the standard resolve_aura helper. The keywords vec is correctly empty (the aura grants intimidate to the attached creature, it does not itself have intimidate).

### Tricky interactions checked (min 3)
1. **Aura removal**: When Gruesome Deformity leaves the battlefield, the EffectScope::Attached scope no longer matches any creature (the attached_to link is gone), so intimidate correctly stops applying.
2. **Colorless creature with granted intimidate**: The combat code uses `attacker.colors.iter().any(...)` which returns false for a colorless creature, meaning only artifact creatures can block it. This is correct per MTG rules.
3. **Intimidate blocking rules**: Verified in combat.rs -- checks for artifact creature type OR shared color. Matches oracle reminder text exactly ("can't be blocked except by artifact creatures and/or creatures that share a color with it").

### Test coverage
- `gruesome_deformity_grants_intimidate` (innistrad_cards.rs): Casts aura on a creature and asserts intimidate is granted.
- `intimidate_blocks_different_color` (keywords.rs): Verifies a creature of a non-matching color cannot block an intimidate creature, and a creature sharing a color can.
- `artifact_creature_blocks_intimidate` (keywords.rs): Verifies artifact creatures can block intimidate creatures regardless of color.

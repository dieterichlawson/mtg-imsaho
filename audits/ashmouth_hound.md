# Audit: Ashmouth Hound

## Reference (Scryfall/API)
- **Name:** Ashmouth Hound
- **Mana Cost:** {1}{R}
- **Type:** Creature — Elemental Dog
- **Oracle:** Whenever Ashmouth Hound blocks or becomes blocked by a creature, Ashmouth Hound deals 1 damage to that creature.
- **P/T:** 2/1

## Implementation: `ashmouth_hound.rs`
- **Name:** Ashmouth Hound -- CORRECT
- **Mana Cost:** {1}{R} -- CORRECT
- **Type:** Creature — Elemental Dog -- CORRECT (subtypes: ["Elemental", "Dog"])
- **P/T:** 2/1 -- CORRECT
- **Triggered abilities:** Blocks + BecomesBlocked -- CORRECT
- **Effect:** Deals 1 damage via `deal_1_damage` helper -- CORRECT
- **NonCombatDamageDealt:** Correctly emits `NonCombatDamageDealt` event -- CORRECT
- **damaged_by tracking:** Pushes source into `damaged_by` -- CORRECT

## Verdict: PASS -- No issues found

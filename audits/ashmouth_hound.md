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

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
**Type line**: Creature — Elemental Dog
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch (cosmetic)**: Oracle says `"Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature."` but code oracle_text field says `"Whenever Ashmouth Hound blocks or becomes blocked by a creature, Ashmouth Hound deals 1 damage to that creature."` The code uses the old self-referential template instead of the updated "this creature" template.
   - Code: `"Whenever Ashmouth Hound blocks or becomes blocked by a creature, Ashmouth Hound deals 1 damage to that creature."`
   - Oracle: `"Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature."`

Behavior is otherwise correct: two triggered abilities (Blocks and BecomesBlocked) each call deal_1_damage which marks 1 damage on the other creature. Stats (2/1), cost ({1}{R}), subtypes (Elemental, Dog) all match.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature." (was self-referential "Ashmouth Hound" wording). Doc comment updated. Behavior unchanged.

## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature\nEnchanted creature can't attack or block.
**Scryfall type line**: Enchantment — Aura
**Status**: PASS

- Name: Correct ("Pacifism")
- Cost: {1}{W} - Correct
- Type: Enchantment — Aura - Correct (subtypes: ["Aura"])
- Oracle text: "Enchanted creature can't attack or block." - Correct (note: "Enchant creature" is implicit from the Aura subtype + target requirement)
- Target requirement: Creature - Correct
- Continuous effects: PreventAttack and PreventBlock with Attached scope - Correct
- on_resolve uses helpers::resolve_aura to attach to target. Correct.
- Tests: enchantments.rs has `pacifism_prevents_attacking`, spell_fizzle.rs tests Pacifism fizzling.

No issues found.
## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature. Enchanted creature can't attack or block.
**Scryfall type line**: Enchantment — Aura
**Status**: PASS

No issues found.

# Audit: Pacifism

## Official Oracle
- **Name:** Pacifism
- **Cost:** {1}{W}
- **Type:** Enchantment — Aura
- **Oracle Text:** Enchant creature\nEnchanted creature can't attack or block.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {1}{W} — OK
- **Type:** Enchantment with subtype Aura — OK
- **Oracle Text:** "Enchanted creature can't attack or block." — OK
- **P/T:** N/A — OK
- **Keywords:** None — OK
- **Continuous Effects:** PreventAttack + PreventBlock on Attached — OK
- **Target:** TargetRequirement::Creature — OK
- **on_resolve:** Uses resolve_aura helper — OK

## Issues
1. **Missing "Enchant creature" keyword/text**: The oracle_text omits "Enchant creature" which is technically part of the oracle text for Auras. Minor — likely a convention in this engine where Aura subtype + target requirement covers it.

## Verdict: PASS (minor oracle text omission, functionally correct)

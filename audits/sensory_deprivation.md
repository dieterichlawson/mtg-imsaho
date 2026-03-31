# Audit: Sensory Deprivation

## Official Oracle
- **Name:** Sensory Deprivation
- **Cost:** {U}
- **Type:** Enchantment — Aura
- **Oracle Text:** Enchant creature\nEnchanted creature gets -3/-0.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {U} — OK
- **Type:** Enchantment with subtype Aura — OK
- **Oracle Text:** "Enchanted creature gets -3/-0." — OK (omits "Enchant creature" as convention)
- **P/T:** N/A — OK
- **Continuous Effects:** ModifyPT { power: -3, toughness: 0, scope: Attached } — OK
- **Target:** TargetRequirement::Creature — OK
- **on_resolve:** Uses resolve_aura helper — OK

## Issues
None found (same "Enchant creature" omission convention as other Auras).

## Verdict: PASS

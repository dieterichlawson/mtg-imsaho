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

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Sensory Deprivation
- **Mana Cost:** {U}
- **Type:** Enchantment — Aura
- **Oracle Text:** Enchant creature / Enchanted creature gets -3/-0.

### Card Data Audit
- **Name:** Correct ("Sensory Deprivation")
- **Cost:** Correct ({U})
- **Types:** Correct (Enchantment, subtype Aura)
- **Oracle Text String:** MISMATCH
  - **Oracle:** "Enchant creature\nEnchanted creature gets -3/-0."
  - **Code:** "Enchanted creature gets -3/-0."
  - The "Enchant creature" line is missing from the oracle_text string.

### Behavior Audit
- **Aura targeting:** `target_requirement` returns `TargetRequirement::Creature`. Correct.
- **Aura attachment:** `on_resolve` calls `helpers::resolve_aura`. Correct.
- **-3/-0 effect:** `ContinuousEffect::ModifyPT { power: -3, toughness: 0, scope: EffectScope::Attached }`. Correct.
- Behavior is fully correct despite the oracle text string omission.

### Result
**ISSUE** -- Oracle text string missing "Enchant creature" line. Code: `"Enchanted creature gets -3/-0."` vs oracle: `"Enchant creature\nEnchanted creature gets -3/-0."`.

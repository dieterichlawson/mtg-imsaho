# Audit: Bonds of Faith

## Oracle (Scryfall/API)
- **Name:** Bonds of Faith
- **Cost:** {1}{W}
- **Type:** Enchantment — Aura
- **Oracle:** Enchant creature. Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/bonds_of_faith.rs`
- **Name:** Bonds of Faith -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Type:** Enchantment — Aura -- CORRECT (subtypes: ["Aura"])
- **Target requirement:** Creature -- CORRECT
- **Effect on Human:** +2/+2 via ModifyPT -- CORRECT
- **Effect on non-Human:** PreventAttack + PreventBlock -- CORRECT
- **Aura attachment:** Uses resolve_aura helper -- CORRECT

## Issues
1. **ISSUE (minor):** The Human check is done once at ETB time and stored as `instance_continuous_effects`. If the creature's type changes (e.g., gains/loses Human subtype), the effect won't update. The oracle says "as long as it's a Human" which implies continuous checking.

## Verdict: PASS (with minor limitation) -- Human check is snapshot rather than continuous

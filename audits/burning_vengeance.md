# Audit: Burning Vengeance

## Oracle (Scryfall/API)
- **Name:** Burning Vengeance
- **Cost:** {2}{R}
- **Type:** Enchantment
- **Oracle:** Whenever you cast a spell from your graveyard, Burning Vengeance deals 2 damage to any target.
- **P/T:** N/A

## Implementation: `burning_vengeance.rs`
- **Name:** Burning Vengeance -- CORRECT
- **Cost:** {2}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Triggered ability:** SpellCast -- CORRECT
- **Trigger condition:** Only on own spells (`caster == controller`) + only flashback spells (`cast_with_flashback`) -- CORRECT
- **Effect:** Presents target choice, deals 2 damage via PendingEffect::DealDamage -- CORRECT
- **Zone check:** Checks self is on battlefield -- CORRECT

## Issues
1. **ISSUE (minor):** The log message on line 68 always says "deals 2 damage to opponent" even before the target choice is resolved. The actual damage and target choice happen via the PendingEffect, but the premature log is misleading.

## Verdict: PASS (with minor log issue) -- Premature log message

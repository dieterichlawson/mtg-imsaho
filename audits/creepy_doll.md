# Audit: Creepy Doll

## Scryfall Reference
- **Name:** Creepy Doll
- **Cost:** {5}
- **Type:** Artifact Creature -- Construct
- **Oracle:** Indestructible. Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
- **P/T:** 1/1
- **Keywords:** Indestructible

## Implementation: `creepy_doll.rs`
- **Name:** Creepy Doll -- CORRECT
- **Cost:** {5} -- CORRECT
- **Type:** [Artifact, Creature] -- CORRECT
- **Subtypes:** ["Construct"] -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Keywords:** [Indestructible] -- CORRECT
- **Behavior:** Coin flip on combat damage to creature, if win -> destroy -- CORRECT
- **Uses try_destroy:** Yes -- CORRECT

## Issues
1. **ISSUE: Trigger implementation uses on_blocks/on_becomes_blocked instead of "deals combat damage to a creature".** The oracle says "Whenever this creature deals combat damage to a creature" which should trigger after damage is dealt. The current implementation triggers when blocking/becoming blocked (before damage). This means the ability fires at the wrong time -- it should fire after combat damage resolution, not when blocks are declared. The TriggerKinds Blocks and BecomesBlocked are incorrect for this card.

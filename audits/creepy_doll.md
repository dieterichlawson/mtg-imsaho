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

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Creepy Doll is indestructible. Whenever Creepy Doll deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
**Scryfall type line**: Artifact Creature -- Construct
**Status**: PASS

Previous trigger timing issue has been fixed. The implementation now uses `TriggerKind::DealsCombatDamageToCreature` and `on_deals_combat_damage_to_creature` hook, which fires after combat damage is dealt rather than at block declaration.

Verified correct:
- Mana cost: {5} -- matches
- Types: Artifact Creature -- matches
- Subtypes: Construct -- matches
- P/T: 1/1 -- matches
- Keywords: Indestructible -- matches (oracle says "Creepy Doll is indestructible" which is functionally equivalent to keyword Indestructible)
- `triggered_abilities`: DealsCombatDamageToCreature -- correct
- `on_deals_combat_damage_to_creature`: flips coin via `rand::thread_rng().gen_bool(0.5)`, destroys creature on win via `try_destroy` -- correct
- No anti-patterns detected (no `CombatDamageDealt` misuse; this correctly uses the creature-specific hook)
- Tests found in `mtg-engine/tests/tier15_cards.rs` and `mtg-engine/tests/creepy_doll.rs`

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Indestructible. Whenever Creepy Doll deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
**Type line**: Artifact Creature — Construct
**Status**: PASS

Card data correct: name, mana cost ({5}), types (Artifact, Creature), subtypes (Construct), P/T (1/1), keywords (Indestructible).

triggered_abilities correctly declares DealsCombatDamageToCreature.

on_deals_combat_damage_to_creature: checks self is on battlefield, flips coin, calls try_destroy on win. Correct.

Tests in creepy_doll.rs are thorough: trigger kind verification, indestructible keyword, trigger fires on combat damage to creature, does NOT fire on combat damage to player, and destroy effect works. No anti-patterns found.

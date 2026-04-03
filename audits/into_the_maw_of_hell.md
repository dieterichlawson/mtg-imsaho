# Audit: Into the Maw of Hell

## Oracle (Official)
- **Name:** Into the Maw of Hell
- **Cost:** {4}{R}{R}
- **Type:** Sorcery
- **Oracle:** Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
- **P/T:** N/A

## Implementation
- Name: "Into the Maw of Hell" -- CORRECT
- Cost: {4}{R}{R} -- CORRECT
- Type: Sorcery -- CORRECT
- Oracle text matches -- CORRECT
- Two targets: land + creature via TwoTargets -- CORRECT
- Destroys land via try_destroy -- CORRECT
- Deals 13 damage to creature -- CORRECT
- Emits NonCombatDamageDealt event -- CORRECT
- Calls move_spell_after_resolve -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Into the Maw of Hell
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Sorcery
- **Cost:** {4}{R}{R}
- **Oracle:** Destroy target land. Into the Maw of Hell deals 13 damage to target creature.

### Card Data
- **Name:** Into the Maw of Hell -- PASS
- **Cost:** {4}{R}{R} -- PASS
- **Types:** Sorcery -- PASS
- **P/T:** None -- PASS

### Oracle Text Match
- Exact match. -- PASS

### Behavior Audit
- **Targeting:** TwoTargets requiring a Land permanent and a Creature. -- PASS
- **is_valid_target:** Checks battlefield zone, allows lands or creatures. -- PASS
- **on_resolve (land):** Destroys target land via try_destroy. -- PASS
- **on_resolve (creature):** Deals 13 damage (damage_marked += 13), emits NonCombatDamageDealt event. -- PASS
- **Independent targets:** Each target checked independently; if one is illegal, the other still resolves. Consistent with rulings. -- PASS
- **Cleanup:** Calls move_spell_after_resolve. -- PASS

### Result: PASS

## Audit — 2026-04-03 07:04
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/150/into-the-maw-of-hell), cached 2026-04-01
**Oracle text**: Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
**Type line**: Sorcery
**Status**: PASS

### Code issues
None found. Implementation is correct and complete.

- **Card data**: Name "Into the Maw of Hell", cost {4}{R}{R} (MV 6), type Sorcery, oracle text -- all match Scryfall exactly.
- **Targeting**: Uses `TwoTargets` with first target = land permanent (via `HasCardType(Land)`), second target = creature (via `Creature`). Correct per oracle text and rulings (must choose both targets on cast).
- **`is_valid_target`**: Validates target is on battlefield and is either a land (by card type) or a creature (by `power.is_some()`). Correct.
- **`on_resolve`**: targets[0] = land destroyed via `try_destroy` (respects indestructible/regeneration). targets[1] = creature receives 13 damage (damage_marked += 13, damaged_by tracked, NonCombatDamageDealt event emitted). Spell cleanup via `move_spell_after_resolve`. All correct.
- **Damage source**: Uses `object_id` (the spell on the stack) as damage source. Correct -- the card says "Into the Maw of Hell deals 13 damage."
- **No anti-patterns detected**: No hardcoded player IDs, no zone assumptions beyond battlefield checks, no missing event emissions.

### Tricky interactions checked (min 3)
1. **Partial target illegality (ruling 2011-09-22)**: If one target becomes illegal before resolution, the other still resolves. The engine handles this correctly: `stack.rs` only fizzles when ALL targets are illegal, and `on_resolve` checks each target's zone independently before acting on it.
2. **Indestructible land**: `try_destroy` checks for indestructible keyword before destroying. If the land is indestructible, destruction is prevented but the 13 damage to the creature still happens. Correct.
3. **Creature with protection from red**: The engine's fizzle check would catch an illegal target at resolution time. If the creature gains protection after casting, the damage portion would be skipped (target illegal) but the land would still be destroyed. This matches the partial resolution ruling.
4. **Both targets fizzle**: If both the land and creature leave the battlefield before resolution, `stack.rs` detects all targets illegal and fizzles the spell. Correct per CR 608.2b.

### Test coverage
- **`into_the_maw_of_hell_card_data`** (innistrad_simple_cards.rs): Verifies card type is Sorcery and mana value is 6. PASS.
- **Missing**: No resolution test that verifies the land is destroyed and 13 damage is dealt to the creature. No test for partial target illegality. Coverage is minimal -- only card data is tested, not behavior.

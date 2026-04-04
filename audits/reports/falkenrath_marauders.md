# Audit: Falkenrath Marauders

## Reference (Scryfall)
- **Name:** Falkenrath Marauders
- **Cost:** {3}{R}{R}
- **Type:** Creature -- Vampire Warrior
- **Oracle:** Flying, haste. Whenever Falkenrath Marauders deals combat damage to a player, put two +1/+1 counters on it.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{R}{R})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Vampire, Warrior)
- Oracle text: CORRECT
- P/T: CORRECT (2/2)
- Keywords: CORRECT (Flying, Haste)
- Combat damage trigger: CORRECT (TriggerKind::CombatDamageToPlayer)
- Two +1/+1 counters: CORRECT (add_counters with PlusOnePlusOne, 2)

## Issues
None found.

## Audit - 2026-04-02

### Oracle Text (Scryfall)

```
Flying
Haste (This creature can attack and {T} as soon as it comes under your control.)
Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
```

**Name:** Falkenrath Marauders
**Mana Cost:** {3}{R}{R}
**Type:** Creature — Vampire Warrior
**P/T:** 2/2
**Keywords:** Flying, Haste

### Implementation Review

**File:** `mtg-engine/src/cards/isd/falkenrath_marauders.rs`

#### Card Data
- **Name:** Correct. `"Falkenrath Marauders"`
- **Mana Cost:** Correct. `Generic(3), Red, Red` = `{3}{R}{R}`
- **Card Types:** Correct. `Creature`
- **Supertypes:** Correct. Empty.
- **Subtypes:** Correct. `["Vampire", "Warrior"]`
- **Power/Toughness:** Correct. `2/2`
- **Keywords:** Correct. `[Flying, Haste]`

#### Oracle Text Field
- MISMATCH (cosmetic): The `oracle_text` field uses the card's name while Scryfall uses the modern templating with "this creature" / "it".
  - **Oracle:** `"Whenever this creature deals combat damage to a player, put two +1/+1 counters on it."`
  - **Impl:** `"Whenever Falkenrath Marauders deals combat damage to a player, put two +1/+1 counters on Falkenrath Marauders."`
  - This is functionally equivalent; Scryfall updated to the modern "this creature" template. No behavioral impact.

#### Triggered Abilities
- Correct. One `TriggeredAbilityDef` with `kind: CombatDamageToPlayer`.
- Description string matches the ability's effect.

#### Combat Damage Behavior (`on_combat_damage_to_player`)
- Correctly checks that the creature is on the battlefield before adding counters.
- Correctly adds 2 `PlusOnePlusOne` counters via `state.add_counters(self_id, CounterType::PlusOnePlusOne, 2)`.

#### Tests
- **File:** `mtg-engine/tests/tier6_cards.rs`
- `falkenrath_marauders_two_counters_on_combat_damage`: Verifies that exactly 2 +1/+1 counters are placed after combat damage to a player. Correct.

### Verdict

**PASS.** Implementation is functionally correct. One cosmetic oracle text difference (old-style card name vs. modern "this creature" template) with no behavioral impact.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
Haste (This creature can attack and {T} as soon as it comes under your control.)
Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
**Type line**: Creature — Vampire Warrior
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:58
**Oracle text source**: Scryfall API (via scripts/oracle_lookup.py, cached 2026-04-01)
**Oracle text**: Flying
Haste (This creature can attack and {T} as soon as it comes under your control.)
Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
**Type line**: Creature — Vampire Warrior
**Status**: PASS

### Code issues
None. Implementation is functionally correct. The `oracle_text` field uses the card's name ("Whenever Falkenrath Marauders deals combat damage...") rather than Scryfall's modern template ("Whenever this creature deals combat damage..."), but this is a cosmetic text difference with no behavioral impact, and the codebase is inconsistent on this convention.

### Tricky interactions checked (min 3)
1. **Combat damage to player only (not creature)**: The trigger uses `TriggerKind::CombatDamageToPlayer` and `on_combat_damage_to_player`, so it correctly does NOT trigger when dealing combat damage to a blocking creature. Verified in trigger dispatch (`triggers.rs`).
2. **Zone check before adding counters**: The handler checks `o.zone == Zone::Battlefield` before adding counters (line 39), correctly handling the edge case where the creature is removed from the battlefield before the trigger resolves.
3. **Double strike interaction**: With double strike (supported by the engine), each combat damage step would generate a separate `CombatDamageDealt` event, causing the trigger to fire twice and add 4 total +1/+1 counters. The implementation handles this correctly since each event is processed independently.
4. **Counter count**: Adds exactly 2 `PlusOnePlusOne` counters per trigger, matching the oracle text "put two +1/+1 counters on it."

### Test coverage
- `falkenrath_marauders_two_counters_on_combat_damage` (tier6_cards.rs): Creates the creature, pushes a `CombatDamageDealt` event targeting a player, processes triggers, and asserts exactly 2 +1/+1 counters. Test passes.

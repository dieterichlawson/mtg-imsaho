# Audit: Falkenrath Noble

## Reference (Scryfall)
- **Name:** Falkenrath Noble
- **Cost:** {3}{B}
- **Type:** Creature -- Vampire Noble
- **Oracle:** Flying. Whenever Falkenrath Noble or another creature dies, target player loses 1 life and you gain 1 life.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{B})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Vampire, Noble)
- Oracle text: CORRECT (uses "this creature" but Scryfall says "Falkenrath Noble" -- functionally equivalent)
- P/T: CORRECT (2/2)
- Keywords: CORRECT (Flying)
- Self-dies trigger: CORRECT (TriggerKind::SelfDies in on_dies)
- Any creature dies trigger: CORRECT (TriggerKind::AnyCreatureDies in on_any_creature_dies)
- Target player loses 1 life: CORRECT (auto-targets opponent)
- You gain 1 life: CORRECT

## Issues
None found.

---

## Audit 2 (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
```
Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
```

### Implementation: `mtg-engine/src/cards/isd/falkenrath_noble.rs`

| Field | Oracle | Implementation | Verdict |
|---|---|---|---|
| Name | Falkenrath Noble | `"Falkenrath Noble"` | CORRECT |
| Mana cost | {3}{B} | `Generic(3), Colored(Black)` | CORRECT |
| Type | Creature -- Vampire Noble | `Creature`, subtypes `["Vampire", "Noble"]` | CORRECT |
| P/T | 2/2 | `power: Some(2), toughness: Some(2)` | CORRECT |
| Keywords | Flying | `vec![Keyword::Flying]` | CORRECT |
| Oracle text field | (see above) | `"Flying\nWhenever this creature or another creature dies, target player loses 1 life and you gain 1 life."` | CORRECT |

### Triggered Ability Analysis

**Trigger condition -- "this creature or another creature dies":**
- `TriggerKind::SelfDies` registered: triggers `on_dies` when Noble itself dies. CORRECT.
- `TriggerKind::AnyCreatureDies` registered: triggers `on_any_creature_dies` for any other creature dying. CORRECT.
- No double-trigger on self-death: the engine's DeathWatch loop filters `o.zone == Zone::Battlefield && o.id != dead_id`, so Noble does not get a DeathWatch trigger for its own death. Only `SelfDies` fires. CORRECT per ruling: "If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them."

**Effect -- "target player loses 1 life and you gain 1 life":**
- `drain()` function subtracts 1 from opponent's life and adds 1 to controller's life. CORRECT.
- Emits `LifeChanged` events for both. CORRECT.

**Targeting -- "target player":**
- Oracle says "target player" (any player), implementation hardcodes `state.opponent(controller)`. In 2-player this is functionally correct since you would never target yourself to lose 1 life. Acceptable per project convention (comment in code: "In 2-player, auto-targets the opponent").
- MINOR NOTE: In multiplayer, "target player" would allow choosing which opponent. Not relevant for this 2-player engine.

### Tests
- `falkenrath_noble_drains_on_any_death` (tier3_cards.rs): PASS
- `falkenrath_noble_triggers_on_opponent_creature_death` (bug_fixes.rs): PASS
- `falkenrath_noble_triggers_on_own_creature_death` (bug_fixes.rs): PASS
- `falkenrath_noble_triggers_on_self_death` (bug_fixes.rs): PASS

### Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:58
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/100/falkenrath-noble), cached 2026-04-01
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: PASS

### Code issues
1. **Simultaneous death limitation (systemic, not card-specific):** Per Scryfall ruling: "If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them." The engine's SBA processing moves creatures to the graveyard one-by-one before triggers are collected, so when Noble dies simultaneously with other creatures, the DeathWatch system won't find Noble on the battlefield when processing other creatures' deaths. Noble only triggers once (via SelfDies) instead of N times. This is an engine-level architectural limitation affecting all death-watch triggers, not specific to this card's implementation.
2. **"Target player" hardcoded to opponent:** Oracle says "target player" (could target any player), but implementation always targets opponent via `state.opponent(controller)`. Acceptable per 2-player project convention (documented in code comment). You would never strategically target yourself to lose 1 life.

### Tricky interactions checked (min 3)
1. **Self-death trigger:** When Noble itself dies, `SelfDies` fires `on_dies`, which correctly drains. The `AnyCreatureDies` path is excluded by the engine filter `o.id != dead_id`. No double-trigger. Verified by test `falkenrath_noble_triggers_on_self_death`.
2. **Opponent's creature dying:** Noble triggers on ANY creature's death, not just your own. The `on_any_creature_dies` handler does not filter by `dead_controller`. Verified by test `falkenrath_noble_triggers_on_opponent_creature_death`.
3. **Noble in graveyard doesn't trigger:** The `on_any_creature_dies` handler checks `o.zone == Zone::Battlefield` before proceeding, and the engine's DeathWatch similarly filters to battlefield-only watchers. If Noble is already dead, it won't trigger for subsequent deaths.
4. **Life events emitted:** The `drain()` function pushes `LifeChanged` events for both the opponent (life loss) and the controller (life gain), ensuring the UI and other game systems can react.

### Test coverage
- `falkenrath_noble_drains_on_any_death` (tier3_cards.rs): own creature dies, drains opponent -- PASS
- `falkenrath_noble_triggers_on_opponent_creature_death` (bug_fixes.rs): opponent's creature dies -- PASS
- `falkenrath_noble_triggers_on_own_creature_death` (bug_fixes.rs): own creature dies -- PASS
- `falkenrath_noble_triggers_on_self_death` (bug_fixes.rs): Noble itself dies -- PASS
- **Not tested:** simultaneous death with multiple creatures (systemic limitation noted above)
- **Not tested:** multiple Nobles on battlefield (would work correctly per engine architecture)

## Re-evaluation — 2026-04-02 21:10

**Status**: ISSUE (reclassified from PASS)

### Code issues
- Simultaneous death triggers only fire once instead of N times: when Falkenrath Noble dies at the same time as other creatures, the engine processes deaths one-by-one and the DeathWatch system does not find Noble on the battlefield when processing other creatures' deaths, so Noble only triggers once (via SelfDies) instead of once for each creature that died

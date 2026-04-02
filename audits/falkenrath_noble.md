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

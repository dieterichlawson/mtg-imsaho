# Audit: Elder Cathar

## Reference (Scryfall)
- **Name:** Elder Cathar
- **Cost:** {2}{W}
- **Type:** Creature -- Human Soldier
- **Oracle:** When Elder Cathar dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Soldier)
- Oracle text: CORRECT
- P/T: CORRECT (2/2)
- Dies trigger: CORRECT (TriggerKind::SelfDies)
- +1/+1 counter on target creature: CORRECT
- Human bonus (2 counters instead of 1): CORRECT
- Targets creature you control: CORRECT (filters by controller)

## Issues

### BUG (minor): Human subtype check ignores obj.subtypes (affects tokens)

Both the single-target path in `on_dies` (elder_cathar.rs lines 51-52) and the multi-target path in `PendingEffect::AddCounters` (engine.rs lines 2022-2025) check Human status via registry only:

```
registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Human"))
```

They do NOT check `obj.subtypes` on the game object. For normal cards this is fine because subtypes come from the registry. However, for **tokens** whose subtypes are stored on `obj.subtypes` (not in registry), a Human token would incorrectly receive only 1 counter instead of 2.

Compare with `combat.rs` `get_subtypes()` (line 356-369) which correctly merges both `obj.subtypes` and `registry.card_data().subtypes`.

No test currently covers this case (Human token receiving counters from Elder Cathar's death trigger).

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues

The previously identified issue has been fixed:

1. **Human subtype check now covers both obj.subtypes and registry: FIXED.** Lines 50-58 now check both sources:
   ```rust
   let obj_has = o.subtypes.iter().any(|s| s == "Human");
   let card_has = registry.card_data(o.card_id)
       .map(|d| d.subtypes.iter().any(|s| s == "Human"))
       .unwrap_or(false);
   obj_has || card_has
   ```
   This correctly handles Human tokens (whose subtypes live on `obj.subtypes`) and normal cards (whose subtypes come from the registry).

2. **Card data correct.** Cost `{2}{W}` (lines 17-19), type Creature (line 21), subtypes Human/Soldier (line 23), P/T 2/2 (lines 24-25).

3. **Counter logic correct.** `count = if is_human { 2 } else { 1 }` (line 59), using `CounterType::PlusOnePlusOne` (line 60). Matches oracle: "put a +1/+1 counter ... If that creature is a Human, put two +1/+1 counters on it instead."

4. **Dies trigger correct.** `TriggerKind::SelfDies` (line 30), `on_dies` handler (line 37).

5. **Targeting correct.** Filters to creatures controlled by the same player, excludes self (`o.id != object_id` at line 41).

6. **Multi-target path correct.** When multiple targets exist, presents `ChooseTarget` with `PendingEffect::AddCounters { count: 1, human_bonus: true }` (line 74). The `human_bonus` flag ensures the engine applies the 2-counter logic for Humans.

### Tricky interactions checked
- Elder Cathar dying with no other creatures: correctly does nothing (lines 45-46).
- Single target auto-selected: lines 47-63 handle the single-creature case without requiring a choice.
- Elder Cathar cannot target itself (already dead and excluded by `o.id != object_id`).

### Test coverage
- Tests exist for basic counter placement and Human bonus.
- No test for Human token receiving 2 counters (would validate the fix).

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
No issues found.

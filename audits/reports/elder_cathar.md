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

## Audit — 2026-04-02 20:54

**Oracle text source**: Scryfall API (via `scripts/oracle_lookup.py`)
**Oracle text**: When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
**Type line**: Creature — Human Soldier
**Status**: ISSUE

### Code issues

1. **Oracle text minor mismatch (cosmetic).** Scryfall returns `"When this creature dies, ..."` but the implementation's `oracle_text` field (line 25) uses `"When Elder Cathar dies, ..."`. Functionally identical -- Scryfall uses the modern templated form while the implementation uses the original printed card name. No behavioral impact.

2. **LLM card knowledge omits the Human bonus.** In `mtg-player/src/llm.rs` line 108:
   - Current: `"Elder Cathar ({2}{W} creature 2/2): When it dies, puts a +1/+1 counter on one of your creatures."`
   - Missing: the Human bonus of two +1/+1 counters instead of one. AI players will not know to prioritize Human targets when Elder Cathar dies, potentially leading to suboptimal play. Should read something like: `"Elder Cathar ({2}{W} creature 2/2): When it dies, puts a +1/+1 counter on one of your creatures (two +1/+1 counters if that creature is a Human)."`

3. **Implementation logic is correct.** The `on_dies` handler correctly:
   - Filters targets to battlefield creatures controlled by the same player (line 40-43)
   - Excludes itself via `o.id != object_id` (line 41)
   - Checks Human status from both `obj.subtypes` and `registry.card_data().subtypes` (lines 50-57)
   - Applies 2 counters for Humans, 1 for non-Humans (line 59)
   - Auto-selects when only one target exists (lines 47-63)
   - Presents a choice when multiple targets exist (lines 64-75)
   - Uses `PendingEffect::AddCounters { count: 1, human_bonus: true }` for the multi-target path (line 72), and the engine handler (engine.rs:2216-2236) correctly applies the same dual-check Human logic

4. **Card data correct.** Name "Elder Cathar", cost {2}{W}, Creature type, subtypes Human/Soldier, P/T 2/2, TriggerKind::SelfDies.

### Tricky interactions checked (min 3)

1. **Elder Cathar dying with no other creatures on your side**: The `targets.is_empty()` check (line 45) correctly handles this by doing nothing. No crash or incorrect behavior.

2. **Elder Cathar cannot target itself**: Even though `object_id` still exists (now in graveyard), the filter `o.zone == Zone::Battlefield` (line 41) and `o.id != object_id` (line 41) both prevent self-targeting. The self-exclusion is redundant with the zone check but adds safety.

3. **Human token receiving counters**: The Human subtype check at lines 50-57 checks both `o.subtypes` (for tokens, which store subtypes on the object) and `registry.card_data().subtypes` (for normal cards). A Human Spirit token created by Doomed Traveler's death trigger would correctly receive 2 counters if it were on the battlefield when Elder Cathar's trigger resolves. However, per MTG rules and the Doomed Traveler interaction research, if both die simultaneously the Spirit token won't exist yet when Elder Cathar's target must be chosen.

4. **Simultaneous death with Doomed Traveler**: If both Elder Cathar and Doomed Traveler die at the same time (e.g., a board wipe), Elder Cathar's trigger targets must be chosen when put on the stack. The Spirit token from Doomed Traveler hasn't been created yet, so it cannot be chosen as a target. The implementation handles this correctly because targets are computed at trigger resolution time from what's currently on the battlefield.

5. **Multi-target path human_bonus flag**: The `PendingEffect::AddCounters { count: 1, human_bonus: true }` correctly defers the Human check to resolution time in the engine (engine.rs:2218-2230), ensuring the counter count is determined based on the target's actual type at resolution.

### Test coverage

- `elder_cathar_grants_counter_on_death` (tier3_cards.rs:404): Basic death trigger, single non-Human target gets 1 counter, P/T updated
- `elder_cathar_gives_two_counters_to_human` (card_mechanics.rs:412): Human target (Doomed Traveler) gets 2 counters
- `elder_cathar_gives_one_counter_to_non_human` (card_mechanics.rs:432): Non-Human target gets 1 counter
- **Missing**: No test for the multi-target choice path (when 2+ creatures exist)
- **Missing**: No test for the zero-target path (no other creatures when Elder Cathar dies)
- **Missing**: No test for Human token receiving 2 counters

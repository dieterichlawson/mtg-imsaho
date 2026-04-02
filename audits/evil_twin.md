# Audit: Evil Twin

## Reference (Scryfall)
- **Name:** Evil Twin
- **Cost:** {2}{U}{B}
- **Type:** Creature -- Shapeshifter
- **Oracle:** You may have Evil Twin enter the battlefield as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
- **P/T:** 0/0

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{U}{B})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Shapeshifter)
- Oracle text: CORRECT
- P/T: CORRECT (0/0)
- Copies a creature on ETB: CORRECT
- Gains destroy ability: CORRECT (activated ability with {U}{B}, tap cost)
- Destroy ability requires tap: CORRECT (requires_tap: true)
- Targets creature with same name: CORRECT (checks target_name == my_name)
- Uses try_destroy: CORRECT (destroy, not sacrifice)

## Issues

### ISSUE 1 (Gameplay, High): Copy is not optional -- "You may" is ignored

**Oracle text**: "**You may** have this creature enter as a copy of any creature on the battlefield"

**Ruling**: "You can choose not to copy anything. In that case, Evil Twin enters the battlefield as a 0/0 creature, and is probably put into the graveyard immediately."

**Code** (`on_enter_battlefield`, lines 46-49):
```rust
let target: Option<ObjectId> = state.objects.values()
    .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != object_id)
    .max_by_key(|o| if o.controller != controller { 1 } else { 0 })
    .map(|o| o.id);
```

The code automatically picks a creature to copy (preferring opponent's). The player is never given a choice to decline copying. The "you may" clause means the controller must be able to choose (a) whether to copy at all, and (b) which creature to copy. Both choices are missing.

### ISSUE 2 (Gameplay, High): Player does not choose which creature to copy

**Oracle text**: "...a copy of **any creature** on the battlefield"

The code at line 48 uses `max_by_key` to automatically select an opponent's creature. The controller should be able to choose any creature on the battlefield, not have one auto-selected. This removes a significant strategic decision.

### ISSUE 3 (Gameplay, Medium): Copy does not include card types

**Oracle text**: "enter as a **copy** of any creature"

**Ruling**: "Evil Twin copies exactly what was printed on the original creature..."

The copy effect at lines 60-73 copies: `name`, `power`, `toughness`, `card_id`, `subtypes`, and `keywords`. It does **not** copy `card_types` (e.g., if the target is an Artifact Creature, the copy stays plain Creature). The `card_id` reassignment partially addresses this (by pointing to the copied card's definition), but the `card_types` on the object itself are never updated.

### ISSUE 4 (Gameplay, Medium): Subtypes are merged instead of replaced

**Code** (lines 67-73):
```rust
let mut all_subtypes = reg_subtypes;
for s in subtypes {
    if !all_subtypes.contains(&s) {
        all_subtypes.push(s);
    }
}
obj.subtypes = all_subtypes;
```

A copy effect should replace the subtypes entirely with the copied creature's printed subtypes. The code merges the target's runtime subtypes with its registered subtypes, which could produce incorrect results for creatures modified by other effects.

### ISSUE 5 (Gameplay, Medium): Target filter is `Any` instead of same-name restriction

**Oracle text**: "Destroy target creature **with the same name as this creature**"

**Code** (line 101):
```rust
target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::Any)),
```

The target requirement allows targeting any creature on the battlefield. The name check only happens at resolution in `on_activate_ability` (line 127: `if target_name == my_name`). The engine will present all creatures as valid targets to the player/AI, and the ability will silently fail if a non-matching creature is chosen. No `SameName` variant exists in `TargetFilter`, so this may be a framework limitation, but it is still a behavioral mismatch.

### ISSUE 6 (Gameplay, Low): `is_evil_twin` marker is not a copiable value

**Ruling**: "The activated ability that Evil Twin gains as part of its copy effect is a copiable value that other effects may copy."

**Code** (line 75):
```rust
obj.card_state.insert("is_evil_twin".into(), ObjectId(1));
```

The destroy ability is gated on `card_state.contains_key("is_evil_twin")` (line 88). If another clone effect copies the Evil Twin, it would need to also copy this `card_state` entry for the ability to be available. The `card_state` map is likely not included in standard copy operations, so subsequent copies of Evil Twin would lose the destroy ability, contradicting the ruling.

## Tests

One test exists (`evil_twin_copies_creature_on_etb` in `mtg-engine/tests/tier15_cards.rs:1505`). It verifies that ETB copy sets name/power/toughness and the `is_evil_twin` marker. No tests exist for:
- The destroy activated ability
- Optional copy (choosing not to copy)
- Player choice of copy target
- Copying another copy/clone
- The copiable value interaction

## LLM Player Knowledge

No Evil Twin-specific entries found in `mtg-player/src/llm.rs`.

## Summary

| # | Severity | Issue |
|---|----------|-------|
| 1 | High | Copy is mandatory; should be optional ("you may") |
| 2 | High | Copy target is auto-selected; player should choose |
| 3 | Medium | Copy does not update card_types (Artifact Creature, etc.) |
| 4 | Medium | Subtypes are merged instead of cleanly replaced |
| 5 | Medium | Target filter is `Any` instead of same-name restriction |
| 6 | Low | `is_evil_twin` marker is not copiable by other clone effects |

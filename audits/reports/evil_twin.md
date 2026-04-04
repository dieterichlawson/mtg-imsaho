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

---

## Re-Audit (2026-04-02)

### Oracle Text (Scryfall, verbatim)
"You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature.""

### Previous Issues -- Status

| # | Previous Issue | Status | Notes |
|---|---------------|--------|-------|
| 1 | Copy is mandatory; should be optional | FIXED | Now uses `present_optional_target_choice` (helpers.rs) with `optional: true`. Player can decline. |
| 2 | Copy target is auto-selected; player should choose | FIXED | All creatures presented as targets via `creature_targets_except`; player picks. |
| 3 | Copy does not update card_types | FIXED | `CopyCreature` handler (engine.rs:2388) now assigns `obj.card_types = card_types;`. |
| 4 | Subtypes are merged instead of replaced | FIXED | `CopyCreature` handler (engine.rs:2389) now does clean replacement: `obj.subtypes = subtypes;`. |
| 5 | Target filter is `Any` instead of same-name | FIXED | Activated ability now uses `TargetFilter::SameNameAsSource` (evil_twin.rs:87). Engine filter (engine.rs:1219-1224) checks `source.name == obj.name`. |
| 6 | `is_evil_twin` marker is not copiable | OPEN | `card_state` map is still not part of the standard `CopyCreature` copy path. Another clone copying Evil Twin will not inherit the destroy ability. Ruling: "The activated ability that Evil Twin gains as part of its copy effect is a copiable value that other effects may copy." |

### New Issues Found

#### ISSUE 7 (Gameplay, Low): Copy does not include colors

**Oracle text**: "enter as a **copy** of any creature"

**Ruling**: "Evil Twin copies exactly what was printed on the original creature..."

The `CopyCreature` handler (engine.rs:2368-2392) copies: `name`, `power`, `toughness`, `card_id`, `keywords`, `card_types`, `subtypes`. It does **not** copy `colors` (GameObject field at state.rs:1129). If the copied creature is e.g. a red creature, Evil Twin's `colors` vec will not be updated. Since `card_id` is reassigned, some systems may derive color from the card definition, but any runtime color state on the object will be stale.

#### ISSUE 8 (Cosmetic, Low): Oracle text uses old template wording

**Scryfall oracle**: "You may have this creature **enter** as a copy..."

**Code oracle_text** (evil_twin.rs:27): "You may have Evil Twin **enter the battlefield** as a copy..."

The code uses the pre-2024 wording template. Not gameplay-affecting but does not match the current Scryfall oracle text.

### Card Data Verification

| Field | Oracle | Code | Match |
|-------|--------|------|-------|
| Name | Evil Twin | "Evil Twin" | CORRECT |
| Mana cost | {2}{U}{B} | Generic(2), Blue, Black | CORRECT |
| Type line | Creature -- Shapeshifter | Creature, subtypes: ["Shapeshifter"] | CORRECT |
| P/T | 0/0 | power: Some(0), toughness: Some(0) | CORRECT |
| Clone mechanic | Optional ("you may"), player chooses | `present_optional_target_choice` | CORRECT |
| Destroy ability cost | {U}{B}, {T} | Blue + Black mana, requires_tap: true | CORRECT |
| Destroy ability target | creature with same name | `SameNameAsSource` filter | CORRECT |
| once_per_turn | No restriction in oracle | false | CORRECT |
| sorcery_speed_only | No restriction in oracle | false | CORRECT |

### Tests

One test exists: `evil_twin_copies_creature_on_etb` (tier15_cards.rs). It verifies:
- ETB presents an optional choice
- After applying `CopyCreature` effect, name/power/toughness match the copied creature
- `is_evil_twin` marker persists after copy

Test passes. Missing test coverage:
- Destroy activated ability (targeting, resolution, same-name check)
- Declining the optional copy (Evil Twin stays 0/0, dies to SBA)
- Another clone copying Evil Twin (copiable value issue, ISSUE 6)
- Copying a multitype creature (colors propagation, ISSUE 7)

### Re-Audit Summary

| # | Severity | Issue | Status |
|---|----------|-------|--------|
| 1 | High | Copy is mandatory | FIXED |
| 2 | High | Copy target auto-selected | FIXED |
| 3 | Medium | Copy missing card_types | FIXED |
| 4 | Medium | Subtypes merged not replaced | FIXED |
| 5 | Medium | Target filter was `Any` | FIXED |
| 6 | Low | `is_evil_twin` not copiable | OPEN |
| 7 | Low | Copy does not include colors | NEW |
| 8 | Low | Oracle text uses old wording template | NEW |

---

## Final Re-Audit (2026-04-02)

### Oracle Text (Scryfall, verbatim)
"You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature.""

### Previous Open/New Issues -- Status

| # | Previous Issue | Status | Notes |
|---|---------------|--------|-------|
| 6 | `is_evil_twin` not copiable | FIXED | `CopyCreature` handler (engine.rs:2370,2389-2391) now reads `is_evil_twin` from the target's `card_state` and propagates it to the copy: `if is_evil_twin { obj.card_state.insert("is_evil_twin".into(), ObjectId(1)); }`. Another clone copying an Evil Twin will inherit the destroy ability. |
| 7 | Copy does not include colors | FIXED | `CopyCreature` handler (engine.rs:2372,2385) now copies colors: `(o.colors.clone(), ...)` extracted from target, then `obj.colors = colors;` assigned to source. |
| 8 | Oracle text uses old wording template | FIXED | Code oracle_text (evil_twin.rs:27) now reads `"You may have this creature enter as a copy..."` which matches the current Scryfall oracle text. |

### Full Checklist

| Check | Result |
|-------|--------|
| Clone is optional ("you may") -- player choice presented | PASS -- uses `present_optional_target_choice` which allows declining |
| Copy includes name | PASS -- `obj.name = name.clone()` (engine.rs:2378) |
| Copy includes P/T | PASS -- `obj.power = power; obj.toughness = toughness` (engine.rs:2379-2380) |
| Copy includes card_types | PASS -- `obj.card_types = card_types` (engine.rs:2383) |
| Copy includes subtypes | PASS -- `obj.subtypes = subtypes` (engine.rs:2384) |
| Copy includes keywords | PASS -- `obj.keywords = keywords` (engine.rs:2382) |
| Copy includes colors | PASS -- `obj.colors = colors` (engine.rs:2385) |
| is_evil_twin marker is copiable | PASS -- explicitly propagated in CopyCreature handler (engine.rs:2389-2391) |
| Destroy ability targets creatures with same name | PASS -- `TargetFilter::SameNameAsSource` (evil_twin.rs:87); engine filter checks `source.name == obj.name` (engine.rs:1213-1217) |
| Destroy ability costs {U}{B}, {T} | PASS -- Blue + Black mana cost, `requires_tap: true` (evil_twin.rs:81-85) |
| Card data: name | PASS -- "Evil Twin" |
| Card data: mana cost | PASS -- {2}{U}{B} |
| Card data: type line | PASS -- Creature, subtypes: ["Shapeshifter"] |
| Card data: P/T | PASS -- 0/0 |
| Card data: oracle text | PASS -- matches Scryfall verbatim |

### Tests

One test: `evil_twin_copies_creature_on_etb` (tier15_cards.rs:1580). Passes. Verifies:
- ETB presents optional choice
- After CopyCreature effect, name/power/toughness match copied creature
- `is_evil_twin` marker persists after copy

### Status: PASS

All six originally reported issues (1-6) and two issues from the second audit (7-8) are now fixed. The implementation correctly handles optional clone choice, copies all relevant characteristics (name, P/T, card_types, subtypes, keywords, colors), propagates the is_evil_twin marker as a copiable value, and uses SameNameAsSource targeting for the destroy ability.

---

## Audit -- 2026-04-02 20:58
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/212/evil-twin), cached 2026-04-01
**Oracle text**: "You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature.""
**Type line**: Creature -- Shapeshifter
**Mana cost**: {2}{U}{B}
**P/T**: 0/0

### Card data verification

| Field | Oracle (Scryfall) | Code (evil_twin.rs) | Match |
|-------|-------------------|---------------------|-------|
| Name | Evil Twin | `"Evil Twin"` (line 16) | PASS |
| Mana cost | {2}{U}{B} | `Generic(2), Colored(Blue), Colored(Black)` (lines 18-20) | PASS |
| Card types | Creature | `vec![CardType::Creature]` (line 22) | PASS |
| Supertypes | (none) | `vec![]` (line 23) | PASS |
| Subtypes | Shapeshifter | `vec!["Shapeshifter".into()]` (line 24) | PASS |
| P/T | 0/0 | `power: Some(0), toughness: Some(0)` (lines 25-26) | PASS |
| Oracle text | (see above) | line 27 | PASS -- verbatim match |

### Behavior verification

| Behavior | Oracle / Ruling | Implementation | Status |
|----------|----------------|----------------|--------|
| Clone is optional ("you may") | "You can choose not to copy anything" | `present_optional_target_choice` (line 57) | PASS |
| Player chooses which creature | "a copy of any creature on the battlefield" | `creature_targets_except` builds target list; player picks (lines 45, 57-64) | PASS |
| Copy includes name, P/T | "copies exactly what was printed on the original creature" | `CopyCreature` handler (engine.rs:2455-2457) sets name, power, toughness | PASS |
| Copy includes card_types | same ruling | engine.rs:2460 `obj.card_types = card_types` | PASS |
| Copy includes subtypes | same ruling | engine.rs:2461 `obj.subtypes = subtypes` -- clean replacement | PASS |
| Copy includes keywords | same ruling | engine.rs:2459 `obj.keywords = keywords` -- from registry card_data | PASS |
| Copy includes colors | same ruling | engine.rs:2462 `obj.colors = colors` | PASS |
| Copy sets card_id | required for registry-based behavior | engine.rs:2458 `obj.card_id = card_id` | PASS |
| Destroy ability: cost {U}{B}, {T} | "{U}{B}, {T}:" | `ManaCost: Blue, Black`, `requires_tap: true` (lines 81-85) | PASS |
| Destroy ability: targets same name | "target creature with the same name as this creature" | `TargetFilter::SameNameAsSource` (line 87); engine filter checks `source.name == obj.name` (engine.rs:1269-1274) | PASS |
| Destroy ability: destroy (not exile/sacrifice) | "Destroy target creature" | `try_destroy` (line 110) | PASS |
| Destroy ability: no speed restriction | No sorcery-speed text | `sorcery_speed_only: false` (line 89) | PASS |
| Destroy ability: no once-per-turn | No such restriction in oracle | `once_per_turn: false` (line 88) | PASS |
| is_evil_twin marker is copiable | "The activated ability... is a copiable value that other effects may copy" | CopyCreature handler reads `is_evil_twin` from target card_state and propagates it (engine.rs:2447, 2466-2468) | PASS |

### Code issues

None found. All eight previously reported issues (1-8) remain fixed. The implementation correctly matches the oracle text and rulings.

### Tricky interactions checked (min 3)

1. **Copiable value propagation**: If another clone copies Evil Twin (which has already copied something), the `is_evil_twin` marker in `card_state` is explicitly checked on the copy target (engine.rs:2447) and propagated to the new copy (engine.rs:2466-2468). This means the second clone also gets the `{U}{B}, {T}: Destroy` ability, matching the ruling "The activated ability that Evil Twin gains as part of its copy effect is a copiable value that other effects may copy."

2. **SameNameAsSource targeting**: The `SameNameAsSource` filter (engine.rs:1269-1274) compares `source.name == obj.name`. Since Evil Twin's name changes when it copies a creature, the destroy ability correctly targets creatures sharing that copied name. The filter does not exclude the Evil Twin itself from targeting -- this is correct per the rules (you can target yourself if you share the name, though it would be suicidal).

3. **Optional copy declining**: When no creatures exist on the battlefield, the `targets` vector from `creature_targets_except` is empty and the `if !targets.is_empty()` check (line 49) skips presenting a choice. Evil Twin stays as 0/0 and will die to state-based actions, matching the ruling: "You can choose not to copy anything. In that case, Evil Twin enters the battlefield as a 0/0 creature, and is probably put into the graveyard immediately." When creatures do exist, `present_optional_target_choice` allows the player to decline.

4. **Copy of a copy**: If the chosen creature is itself copying something else, the implementation copies the object's current runtime characteristics (name, P/T, types, etc.), which are already the characteristics of whatever that creature is copying. This matches the ruling: "If the chosen creature is copying something else... then your Evil Twin enters the battlefield as whatever the chosen creature copied."

### Test coverage

One test: `evil_twin_copies_creature_on_etb` (tier15_cards.rs:1756). Passes. Verifies:
- ETB presents an optional choice (`awaiting_action.is_some()`)
- After applying `CopyCreature` effect, name/power/toughness match the copied creature
- `is_evil_twin` marker persists after copy

Missing test coverage (non-blocking):
- Destroy activated ability (targeting + resolution)
- Declining the optional copy
- Copying a creature that is itself a copy
- Another clone copying Evil Twin (copiable value chain)

### Status: PASS

The implementation is correct. All card data matches the Scryfall oracle text verbatim. Clone mechanics (optional choice, full characteristic copy, copiable destroy ability) are properly implemented. The destroy ability correctly uses SameNameAsSource targeting with {U}{B} + tap cost and calls `try_destroy`. No issues found.

---

## Audit — 2026-04-03 21:31

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
**Type line**: Creature — Shapeshifter
**Status**: ISSUE

### Code issues
- **Replacement effect vs triggered ability** (`mtg-engine/src/cards/isd/evil_twin.rs:32-38`)
  - Oracle text says: `You may have this creature enter as a copy of any creature on the battlefield`
  - Code does: Uses `TriggerKind::EntersBattlefield` and `on_enter_battlefield()` method, making Evil Twin enter as 0/0 first, then copy via triggered ability
  
The oracle text uses replacement effect language ("enter as a copy") meaning the copying should happen as part of entering the battlefield, not as a triggered ability afterwards. The current implementation causes Evil Twin to enter as a 0/0 creature, then the triggered ability goes on the stack to potentially make it copy something. This is incorrect timing per MTG rules.

### Tricky interactions checked
- **Copy timing vs ETB triggers**: FAIL - Should enter as copy directly, triggering copied creature's ETB abilities
- **Destroy ability targeting same-named creatures**: PASS - TargetFilter::SameNameAsSource correctly implemented  
- **"You may" optionality for copying**: PASS - Uses present_optional_target_choice correctly
- **Preserve destroy ability through copy**: PASS - "is_evil_twin" marker set before copy and preserved by engine
- **Cannot copy itself**: PASS - Uses creature_targets_except to exclude self
- **Dies if no copy chosen**: PASS - Stays as 0/0 without copying, dies to state-based actions

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic copying functionality**: `mtg-engine/tests/tier15_cards.rs:1756` / TESTED
- **Destroy ability with same-name targeting**: NOT TESTED
- **"You may" decline to copy**: NOT TESTED 
- **Copy timing vs replacement effect**: NOT TESTED
- **ETB abilities of copied creature**: NOT TESTED
- **Copying tokens with different names**: NOT TESTED
- **"is_evil_twin" marker preservation**: `mtg-engine/tests/tier15_cards.rs:1780` / TESTED
- **Dies if no creatures to copy**: NOT TESTED
- **Cannot target non-creatures with destroy ability**: NOT TESTED

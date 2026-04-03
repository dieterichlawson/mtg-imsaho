# Audit: Ludevic's Test Subject // Ludevic's Abomination

## Oracle (Official)
### Front: Ludevic's Test Subject
- **Cost:** {1}{U}
- **Type:** Creature — Lizard Egg
- **Oracle:** Defender. {1}{U}: Put a hatchling counter on Ludevic's Test Subject. Then if there are five or more hatchling counters on it, remove all of them and transform Ludevic's Test Subject.
- **P/T:** 0/3

### Back: Ludevic's Abomination
- **Type:** Creature — Lizard Horror
- **Oracle:** Trample
- **P/T:** 13/13

## Implementation
- Front name: "Ludevic's Test Subject" -- CORRECT
- Front cost: {1}{U} -- CORRECT
- Front P/T: 0/3 -- CORRECT
- Front keywords: [Defender] -- CORRECT
- Front oracle text matches -- CORRECT
- Back name: "Ludevic's Abomination" -- CORRECT
- Back subtypes: ["Lizard", "Horror"] -- CORRECT
- Back P/T: 13/13 (via dynamic_pt) -- CORRECT
- Back keywords: [Trample] -- CORRECT
- Activated ability: {1}{U}, adds hatchling counter, transforms at 5 -- CORRECT
- Uses card_state for hatchling counter tracking -- OK (workaround)

## Issues
1. **ISSUE (minor):** Front face subtypes are ["Lizard"] but should be ["Lizard", "Egg"]. The official type line is "Creature — Lizard Egg".

## Verdict: PASS (with minor subtype issue)

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: (Front) Defender. {1}{U}: Put a hatchling counter on Ludevic's Test Subject. Then if there are five or more hatchling counters on it, remove all of them and transform Ludevic's Test Subject. (Back) Trample.
**Scryfall type line**: (Front) Creature -- Lizard Egg. (Back) Creature -- Lizard Horror.
**Status**: PASS

Findings:
1. **Mana cost {1}{U}**: Correct.
2. **Front face P/T 0/3**: Correct.
3. **Front face subtypes**: Code has `["Lizard", "Egg"]` (line 23). Previous audit said "Egg" was missing, but the current code includes it. This is now correct.
4. **Front keywords [Defender]**: Correct.
5. **Activated ability {1}{U}**: Correct cost. Adds hatchling counter, transforms at 5+. Logic correct.
6. **Back face name (Ludevic's Abomination)**: Correct.
7. **Back face subtypes ["Lizard", "Horror"]**: Correct per Scryfall.
8. **Back face P/T 13/13**: Correct (via `dynamic_pt`).
9. **Back face keywords [Trample]**: Correct.
10. **Hatchling counter tracking**: Uses `card_state` HashMap with "hatchling_counters" key. Functional workaround for lack of native counter type.
11. **No anti-patterns detected**: No spells, no damage, no triggered abilities needed.
12. **Tests**: Found in `mtg-engine/tests/tier15_cards.rs`.

No new issues found. Previous subtype issue appears resolved.

## Audit — 2026-04-01 14:13

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/64/ludevics-test-subject-ludevics-abomination) and MTG Salvation (https://www.mtgsalvation.com/cards/innistrad/19131-ludevics-abomination)
**Oracle text (front)**: Defender. {1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.
**Oracle text (back)**: Trample
**Type line (front)**: Creature — Lizard Egg
**Type line (back)**: Creature — Lizard Horror
**Mana cost**: {1}{U}
**P/T (front)**: 0/3
**P/T (back)**: 13/13
**Status**: PASS

Findings:
1. **Name**: "Ludevic's Test Subject" / "Ludevic's Abomination" -- correct.
2. **Mana cost {1}{U}**: Correct (Generic(1), Blue).
3. **Front face subtypes ["Lizard", "Egg"]**: Correct per Scryfall.
4. **Front face P/T 0/3**: Correct.
5. **Front face keywords [Defender]**: Correct.
6. **Activated ability {1}{U}**: Correct cost (Generic(1), Blue). Not restricted to once-per-turn (correct, can activate multiple times). Not sorcery-speed-only (correct, activated abilities can be used at instant speed). Only available when not transformed (line 66, correct).
7. **Hatchling counter logic**: Counter tracked via card_state. Increments, checks >= 5, removes all and transforms. Correct per oracle. Per rulings, the check happens as part of the ability's resolution, which this correctly implements.
8. **Back face subtypes ["Lizard", "Horror"]**: Correct per Scryfall.
9. **Back face P/T 13/13**: Correct (via `dynamic_pt`).
10. **Back face keywords [Trample]**: Correct.
11. **should_transform returns false**: Correct -- transformation is handled by the activated ability, not by a trigger.
12. **Tests**: Found in `mtg-engine/tests/tier15_cards.rs`. Tests verify 4 activations do not transform, 5th does, and back face stats are 13/13. Assertions correct.

No issues found.

## Audit — 2026-04-01 15:30

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: Defender
{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.
**Oracle text (back)**: Trample
**Type line (front)**: Creature — Lizard Egg
**Type line (back)**: Creature — Lizard Horror
**Mana cost**: {1}{U}
**P/T (front)**: 0/3
**P/T (back)**: 13/13
**Rulings**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article.
**Status**: PASS

### Code issues
No issues found.

### Detailed verification
1. **Name**: "Ludevic's Test Subject" / "Ludevic's Abomination" — correct.
2. **Mana cost {1}{U}**: `Generic(1), Colored(Blue)` — correct.
3. **Front card types [Creature]**: correct.
4. **Front subtypes ["Lizard", "Egg"]** (line 26): correct per Scryfall "Creature — Lizard Egg".
5. **Front P/T 0/3** (lines 27-28): correct.
6. **Front keywords [Defender]** (line 30): correct.
7. **Back face name "Ludevic's Abomination"** (line 40): correct.
8. **Back face subtypes ["Lizard", "Horror"]** (line 44): correct per Scryfall "Creature — Lizard Horror".
9. **Back face P/T 13/13** (lines 45-46): correct. Also implemented via `dynamic_pt` (line 58) which returns `Some((13, 13))` when `is_transformed`.
10. **Back face keywords [Trample]** (line 48): correct. The `has_keyword` system correctly uses back face keywords when transformed (state.rs:933-936), so Defender is lost and Trample is gained upon transformation.
11. **Activated ability cost {1}{U}** (lines 73-74): correct.
12. **Activated ability availability**: Only on battlefield, only when not transformed (line 66). Correct — the back face has no activated abilities.
13. **Not once-per-turn** (line 80): correct — the ability can be activated multiple times per turn.
14. **Not sorcery-speed-only** (line 81): correct — activated abilities default to instant speed.
15. **Hatchling counter logic** (lines 87-108): Increments counter, checks >= 5, removes all and sets `is_transformed = true`, updates name. Correct per oracle "Then if there are five or more hatchling counters on it, remove all of them and transform it."
16. **should_transform returns false** (line 111): correct — transformation is handled by the activated ability, not by the engine's automatic transform system.
17. **Counter storage via card_state**: Uses `card_state` HashMap instead of a proper counter type. This is a workaround since `CounterType` enum lacks a `Hatchling` variant. Functionally correct but means hatchling counters won't be visible through the standard counter API (e.g., `get_counter_count`).

### Tricky interactions checked
- Defender lost on transform: PASS (has_keyword checks back face keywords when transformed)
- Trample gained on transform: PASS (back face keywords include Trample)
- 4 activations does not transform: PASS (checked in test)
- 5th activation transforms: PASS (checked in test)
- Multiple activations in one turn: PASS (once_per_turn is false)
- Subtypes change on transform: PARTIAL — obj.subtypes is NOT updated on transform (line 95-98 only updates card_state, is_transformed, and name). The `CreatureFilter::HasSubtype` system correctly checks back face data for transformed creatures (state.rs:583-586), but direct `obj.subtypes` checks elsewhere may see stale front-face subtypes.

### Test coverage
- Transforms at 5 counters (including 4 not transforming): `tier15_cards.rs:796` (ludevics_test_subject_transforms_at_five_counters)
- Back face name is correct: `tier15_cards.rs:814`
- Back face P/T via dynamic_pt: `tier15_cards.rs:815`
- Defender prevents attacking (front face): NOT TESTED
- Trample works after transform: NOT TESTED
- Subtypes change on transform: NOT TESTED
- Multiple activations per turn: NOT TESTED (implicitly tested since the test activates 5 times)

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text (front — Ludevic's Test Subject)**:
> Defender
> {1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.

**Oracle text (back — Ludevic's Abomination)**:
> Trample

**Type line (front)**: Creature — Lizard Egg
**Type line (back)**: Creature — Lizard Horror
**Mana cost**: {1}{U}
**P/T (front)**: 0/3
**P/T (back)**: 13/13
**Keywords**: Transform, Trample, Defender
**Status**: ISSUE

### Code issues

1. **Manual transform instead of `helpers::apply_transform`** (medium severity)

   In `on_activate_ability` (lines 95-99), the transform is done manually:
   ```rust
   obj.card_state.remove("hatchling_counters");
   obj.is_transformed = true;
   obj.name = "Ludevic's Abomination".into();
   ```
   This does NOT update `obj.keywords` or `obj.subtypes`. After transforming, the object still carries `keywords: [Defender]` and `subtypes: ["Lizard", "Egg"]` on the object itself.

   Other DFCs in the codebase (e.g., `cloistered_youth.rs`, `screeching_bat.rs`) correctly use `helpers::apply_transform(state, self_id, _registry)` which updates name, keywords, and subtypes atomically, as documented at `helpers.rs` lines 228-230:
   > "This is the correct way to transform a DFC. Card-specific code should call this instead of manually flipping `obj.is_transformed` and `obj.name`, which leaves `obj.keywords` and `obj.subtypes` stale."

   The engine's `has_keyword()` (state.rs lines 931-941) and `matches_filter()` (state.rs lines 582-590) compensate by checking `back_face_data()` when `is_transformed` is true, so runtime behavior is **mostly correct**. However, any code that directly reads `obj.keywords` or `obj.subtypes` (e.g., display, logging, some filter paths) would see stale front-face data.

   **Fix**: Replace the manual transform block (lines 95-99) with:
   ```rust
   obj.card_state.remove("hatchling_counters");
   // drop obj borrow before calling apply_transform
   helpers::apply_transform(state, object_id, _registry);
   ```

2. **Stacked activations can re-trigger transform log** (low severity)

   If 10+ activations are stacked, the first 5 resolutions transform the creature. Subsequent resolutions continue calling `on_activate_ability` which does not check `is_transformed`. Counter accumulation continues on the back face, and at 10 total activations the code would hit `new_count >= 5` again, re-executing the transform block (setting `is_transformed = true` again — a no-op — and logging "transforms into Ludevic's Abomination" a second time). This is cosmetically incorrect but functionally harmless since the back face has no ability to activate and the creature is already transformed.

### Tricky interactions checked

- **Defender on front face**: Correctly declared in `keywords: vec![Keyword::Defender]`. Engine resolves keywords from `back_face_data()` when transformed, so Defender is correctly lost after transform.
- **Trample on back face**: Correctly declared in back face `keywords: vec![Keyword::Trample]`. Correctly gained after transform via engine keyword resolution.
- **Multiple activations per turn**: `once_per_turn: false` — correct per oracle (no restriction).
- **Instant-speed activation**: `sorcery_speed_only: false` — correct.
- **No tap cost**: `requires_tap: false` — correct.
- **Ability unavailable on back face**: `activated_abilities()` returns empty when `is_transformed` (line 66) — correct, Ludevic's Abomination has no activated abilities.
- **Stacking activations**: Counters are added one at a time on resolution. If multiple copies resolve after transform, counters accumulate on the back face but have no meaningful effect (see issue #2 above).

### Test coverage

File: `mtg-engine/tests/tier15_cards.rs`, test `ludevics_test_subject_transforms_at_five_counters` (line 1117).

Covered:
- 4 activations do not transform
- 5th activation transforms
- Back face name is "Ludevic's Abomination"
- Back face P/T is 13/13 via `dynamic_pt`

Not covered:
- Defender prevents attacking on front face
- Defender is lost after transform
- Trample is gained after transform
- Subtypes change from "Lizard Egg" to "Lizard Horror" after transform
- Stacking multiple activations on the stack
- Activated ability is unavailable on the back face

### LLM knowledge

No entry for Ludevic's Test Subject in `mtg-player/src/llm.rs`.

## Re-audit — 2026-04-02 (post-fix)

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text (front — Ludevic's Test Subject)**:
> Defender
> {1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.

**Oracle text (back — Ludevic's Abomination)**:
> Trample

**Type line (front)**: Creature — Lizard Egg
**Type line (back)**: Creature — Lizard Horror
**Mana cost**: {1}{U}
**P/T (front)**: 0/3
**P/T (back)**: 13/13
**Keywords**: Transform, Trample, Defender
**Status**: PASS

### Previous issue verification

1. **Manual transform instead of `helpers::apply_transform`** (previously medium severity) — **FIXED**. The code now calls `helpers::apply_transform(state, object_id, registry)` at line 98, which atomically updates `is_transformed`, `name`, `keywords`, and `subtypes` on the object. The manual `obj.is_transformed = true; obj.name = ...` block has been replaced.

2. **Stacked activations can re-trigger transform log** (previously low severity) — **NOT FIXED**. The `on_activate_ability` method still does not check `is_transformed` before proceeding. If multiple activations are stacked, resolutions after the 5th will continue adding counters on the back face and could re-trigger the transform path. With `apply_transform` now in use, a second call would flip the creature *back* to the front face, which is **worse** than the previous behavior (where it was a no-op). This is now a **medium severity** issue: if a player stacks 10 activations, the 5th resolution transforms to Ludevic's Abomination, then subsequent resolutions accumulate counters again, and the 10th resolution would call `apply_transform` a second time, flipping back to Ludevic's Test Subject.

### Full card data verification

| Field | Oracle | Code | Match |
|-------|--------|------|-------|
| Front name | Ludevic's Test Subject | `"Ludevic's Test Subject"` (line 19) | PASS |
| Front cost | {1}{U} | `Generic(1), Colored(Blue)` (lines 20-23) | PASS |
| Front types | Creature | `[CardType::Creature]` (line 24) | PASS |
| Front subtypes | Lizard Egg | `["Lizard", "Egg"]` (line 26) | PASS |
| Front P/T | 0/3 | `power: Some(0), toughness: Some(3)` (lines 27-28) | PASS |
| Front keywords | Defender | `[Keyword::Defender]` (line 30) | PASS |
| Front oracle text | "Defender\n{1}{U}: Put a hatchling counter..." | Matches verbatim (line 29) | PASS |
| Back name | Ludevic's Abomination | `"Ludevic's Abomination"` (line 40) | PASS |
| Back types | Creature | `[CardType::Creature]` (line 42) | PASS |
| Back subtypes | Lizard Horror | `["Lizard", "Horror"]` (line 44) | PASS |
| Back P/T | 13/13 | `power: Some(13), toughness: Some(13)` (lines 45-46), also `dynamic_pt` returns `(13, 13)` when transformed (line 58) | PASS |
| Back keywords | Trample | `[Keyword::Trample]` (line 48) | PASS |
| Back oracle text | "Trample" | `"Trample"` (line 49) | PASS |
| Back cost | None | `cost: None` (line 41) | PASS |

### Activated ability verification

- Cost `{1}{U}`: `Generic(1), Colored(Blue)` (lines 73-74) — correct.
- No tap cost: `requires_tap: false` (line 77) — correct.
- No sacrifice cost: `SacrificeCost::None` (line 78) — correct.
- No target: `target_requirement: None` (line 79) — correct.
- Not once-per-turn: `once_per_turn: false` (line 80) — correct.
- Not sorcery-speed-only: `sorcery_speed_only: false` (line 81) — correct.
- Only available on front face: checks `!o.is_transformed` (line 66) — correct.
- Only available on battlefield: checks `o.zone == Zone::Battlefield` (line 66) — correct.

### Transform logic verification

- `on_activate_ability` increments counter (lines 87-90), checks `>= 5` (line 93), removes counters (lines 95-97), calls `helpers::apply_transform` (line 98) — correct.
- `helpers::apply_transform` (helpers.rs lines 231-265) flips `is_transformed`, copies name/keywords/subtypes from `back_face_data()` — correct.
- `should_transform` returns `false` (line 110-112) — correct, transformation is ability-driven not automatic.

### Issues

1. **Stacked activations regression** (medium severity): With the fix to use `apply_transform`, stacked activations beyond the 5th will now call `apply_transform` again, which toggles `is_transformed` back to `false`, effectively un-transforming the creature. The `on_activate_ability` method should guard against this by checking `is_transformed` at the top and returning early if the creature is already transformed.

   **Suggested fix** — add at the top of `on_activate_ability`:
   ```rust
   if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
       return;
   }
   ```

### Test coverage

File: `mtg-engine/tests/tier15_cards.rs`
- 4 activations do not transform: covered
- 5th activation transforms: covered
- Back face name "Ludevic's Abomination": covered
- Back face P/T 13/13: covered
- Stacking 10+ activations (regression): NOT covered
- Defender prevents attacking on front face: NOT covered
- Trample gained after transform: NOT covered

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Defender / {1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it. // Back: Trample
**Type line**: Creature — Lizard Egg // Creature — Lizard Horror
**Status**: PASS

### Code issues
No issues found. Card data correct: cost {1}{U}, P/T 0/3 front / 13/13 back, subtypes Lizard Egg / Lizard Horror. Defender keyword on front. Trample keyword on back. Activated ability costs {1}{U}, correctly tracks hatchling counters in card_state, transforms at 5+ counters after removing all. Ability is only available on front face (not transformed). `dynamic_pt` correctly overrides to 13/13 when transformed. `should_transform` returns false since transformation is handled by the activated ability, not upkeep triggers.

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**:
Front (Ludevic's Test Subject): Defender
{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.
Back (Ludevic's Abomination): Trample
**Type line**: Creature — Lizard Egg // Creature — Lizard Horror
**Mana cost**: {1}{U}
**P/T**: 0/3 // 13/13
**Status**: PASS

### Code issues
No issues found.

All card data matches oracle text exactly:

| Field | Oracle | Code | Match |
|-------|--------|------|-------|
| Front name | Ludevic's Test Subject | `"Ludevic's Test Subject"` (line 19) | PASS |
| Front cost | {1}{U} | `Generic(1), Colored(Blue)` (lines 20-23) | PASS |
| Front types | Creature | `[CardType::Creature]` (line 24) | PASS |
| Front subtypes | Lizard Egg | `["Lizard", "Egg"]` (line 26) | PASS |
| Front P/T | 0/3 | `power: Some(0), toughness: Some(3)` (lines 27-28) | PASS |
| Front keywords | Defender | `[Keyword::Defender]` (line 30) | PASS |
| Front oracle text | "Defender\n{1}{U}: Put a hatchling counter..." | Stored verbatim (line 29) | PASS |
| Back name | Ludevic's Abomination | `"Ludevic's Abomination"` (line 40) | PASS |
| Back types | Creature | `[CardType::Creature]` (line 42) | PASS |
| Back subtypes | Lizard Horror | `["Lizard", "Horror"]` (line 44) | PASS |
| Back P/T | 13/13 | `power: Some(13), toughness: Some(13)` (lines 45-46), `dynamic_pt` returns `(13, 13)` when transformed (line 58) | PASS |
| Back keywords | Trample | `[Keyword::Trample]` (line 48) | PASS |

Activated ability:
- Cost {1}{U}: correct (`Generic(1), Colored(Blue)`, lines 73-75)
- No tap cost: correct (`requires_tap: false`, line 77) -- oracle has no {T} in cost
- No sacrifice cost: correct (`SacrificeCost::None`, line 78)
- No target: correct (`target_requirement: None`, line 79)
- Not once-per-turn: correct (`once_per_turn: false`, line 80)
- Instant speed: correct (`sorcery_speed_only: false`, line 81)
- Only available on front face on battlefield: correct (line 66 checks `!o.is_transformed` and `o.zone == Zone::Battlefield`)

Transform logic:
- Hatchling counter tracked via `card_state` HashMap (workaround for missing `CounterType::Hatchling`): functionally correct
- Counter incremented by 1 each activation (line 95): correct per oracle "Put a hatchling counter"
- Threshold check `new_count >= 5` (line 97): correct per oracle "five or more"
- Counters removed before transform (line 100): correct per oracle "remove all of them"
- Uses `helpers::apply_transform` (line 102): correct -- atomically updates `is_transformed`, `name`, `keywords`, `subtypes`
- Guard against stacked activations (lines 87-89): prevents re-transform if creature already transformed
- `should_transform` returns false (line 115): correct -- transformation is ability-driven, not automatic

### Tricky interactions checked (min 3)
1. **Defender lost on transform**: PASS. `apply_transform` replaces `obj.keywords` with back face keywords `[Trample]`. The engine's `has_keyword` also checks back face data when transformed. Defender is correctly absent after transform, confirmed in test assertion at tier15_cards.rs:1385.
2. **Trample gained on transform**: PASS. Back face `keywords: [Keyword::Trample]` is applied by `apply_transform`. Test asserts `obj.keywords.contains(&Keyword::Trample)` at tier15_cards.rs:1384.
3. **Stacked activations safety**: PASS. If a player puts 10 activations on the stack, the 5th resolution transforms the creature and subsequent resolutions hit the `is_transformed` guard at line 87 and return early, preventing double-transform or counter accumulation on back face.
4. **Subtypes change on transform**: PASS. `apply_transform` copies back face subtypes `["Lizard", "Horror"]` over front face subtypes `["Lizard", "Egg"]`. Test asserts `obj.subtypes.contains(&"Horror".to_string())` at tier15_cards.rs:1387.
5. **Defender prevents attacking on front face**: PASS. Combat system at `combat.rs:579` checks `!state.has_keyword(o.id, Keyword::Defender, registry)` before allowing a creature to be declared as attacker.

### Test coverage
File: `mtg-engine/tests/tier15_cards.rs`, test `ludevics_test_subject_transforms_at_five_counters` (line 1362).
- 4 activations do not transform: covered (line 1375)
- 5th activation transforms: covered (line 1380)
- Back face name "Ludevic's Abomination": covered (line 1381)
- Back face P/T 13/13 via dynamic_pt: covered (line 1382)
- Back face gains Trample: covered (line 1384)
- Back face loses Defender: covered (line 1385)
- Back face subtypes include Lizard and Horror: covered (lines 1386-1387)

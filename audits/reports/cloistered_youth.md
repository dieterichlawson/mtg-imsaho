# Audit: Cloistered Youth // Unholy Fiend

## Scryfall Reference
- **Front Face: Cloistered Youth**
  - **Cost:** {1}{W}
  - **Type:** Creature -- Human
  - **Oracle:** At the beginning of your upkeep, you may transform this creature.
  - **P/T:** 1/1

- **Back Face: Unholy Fiend**
  - **Cost:** (none)
  - **Type:** Creature -- Horror
  - **Oracle:** At the beginning of your end step, you lose 1 life.
  - **P/T:** 3/3

## Implementation: `cloistered_youth.rs`
- **Front face name:** Cloistered Youth -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Front subtypes:** ["Human"] -- CORRECT
- **Front P/T:** 1/1 -- CORRECT
- **Back face name:** Unholy Fiend -- CORRECT
- **Back subtypes:** ["Horror"] -- CORRECT
- **Back P/T:** 3/3 -- CORRECT
- **Upkeep:** Transforms front to back -- CORRECT
- **End step:** Loses 1 life when transformed -- CORRECT

## Issues
1. **ISSUE: Front face P/T is 1/1, but Scryfall says 1/1.** Wait -- actually the doc comment says "3/2" on line 6 but the card_data says power: Some(1), toughness: Some(1). Checking Scryfall: front face is 1/1. The doc comment on line 6 says "{1}{W} 3/2 Human" which is WRONG in the comment but the code uses 1/1 which is CORRECT. The dynamic_pt returns (3,3) when transformed which matches the back face. The comment is just misleading but the code is correct.

No functional issues.

## Audit — 2026-04-01 12:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: At the beginning of your upkeep, you may transform this creature.
**Oracle text (back)**: At the beginning of your end step, you lose 1 life.
**Type line (front)**: Creature — Human
**Type line (back)**: Creature — Horror
**Status**: ISSUE

### Code issues

1. **"You may" transform is auto-decided** (`cloistered_youth.rs:80-88`)
   - Oracle text says: `"At the beginning of your upkeep, you may transform this creature."`
   - Code does: automatically transforms at upkeep with no player choice — `if !is_transformed { obj.is_transformed = true; }`. The player cannot decline to transform.

2. **Spurious triggers due to misplaced triggered_abilities declarations** (`cloistered_youth.rs:28-43`)
   - Front face declares both `TriggerKind::Upkeep` ("may transform") and `TriggerKind::EndStep` ("lose 1 life")
   - Back face declares `triggered_abilities: vec![]` (empty)
   - Problem: When Cloistered Youth is NOT transformed, an EndStep trigger fires on the stack with description "lose 1 life" (doing nothing since `on_end_step` checks `is_transformed`). When transformed as Unholy Fiend, an Upkeep trigger fires on the stack with description "may transform" (doing nothing since `on_upkeep` checks `!is_transformed`).
   - The front face's `triggered_abilities` should only declare `TriggerKind::Upkeep`, and the back face's `triggered_abilities` should declare `TriggerKind::EndStep`.

3. **Misleading doc comment** (`cloistered_youth.rs:6`)
   - Comment says: `"Cloistered Youth {1}{W} 3/2 Human"`
   - Oracle P/T is: 1/1 (front), 3/3 (back)
   - Code card_data correctly uses `power: Some(1), toughness: Some(1)` — the comment is just wrong but the code is correct.

### Tricky interactions checked
- Upkeep trigger only fires for controller: PASS (line 78-79 checks `state.active_player != controller`)
- Life loss emits LifeChanged event: PASS (line 104)
- Back face P/T via dynamic_pt: PASS (returns (3,3) when transformed)
- Already-transformed creature does not re-transform at upkeep: PASS (line 80 checks `!is_transformed`)

### Test coverage
- Transform at upkeep: `tier15_cards.rs:740` (cloistered_youth_transforms_at_upkeep)
- Life loss at end step when transformed: `tier15_cards.rs:754` (unholy_fiend_drains_life_at_end_step)
- Declining to transform: NOT TESTED (and not possible due to issue #1)
- No life loss when NOT transformed at end step: NOT TESTED
- Spurious trigger visibility: NOT TESTED

## Audit — 2026-04-01 21:32

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: At the beginning of your upkeep, you may transform this creature.
**Oracle text (back)**: At the beginning of your end step, you lose 1 life.
**Type line (front)**: Creature — Human
**Type line (back)**: Creature — Horror
**Mana cost**: {1}{W}
**P/T (front)**: 1/1
**P/T (back)**: 3/3
**Keywords**: Transform (not modeled as a Keyword enum variant; consistent with all other DFCs in the codebase)
**Rulings**: [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article.
**Status**: PASS

### Code issues
No issues found.

**Detailed verification (all checks against oracle text from Scryfall API):**

| Check | Oracle | Code | Result |
|-------|--------|------|--------|
| Mana cost | {1}{W} | `Generic(1), Colored(Color::White)` | MATCH |
| Card type | Creature | `CardType::Creature` | MATCH |
| Supertypes | (none) | `vec![]` | MATCH |
| Subtypes (front) | Human | `vec!["Human".into()]` | MATCH |
| Subtypes (back) | Horror | `vec!["Horror".into()]` | MATCH |
| P/T (front) | 1/1 | `power: Some(1), toughness: Some(1)` | MATCH |
| P/T (back) | 3/3 | `power: Some(3), toughness: Some(3)` + `dynamic_pt` returns `Some((3, 3))` when transformed | MATCH |
| Oracle text (front) | "At the beginning of your upkeep, you may transform this creature." | `"At the beginning of your upkeep, you may transform Cloistered Youth."` | MATCH (minor wording: "this creature" vs card name) |
| Oracle text (back) | "At the beginning of your end step, you lose 1 life." | `"At the beginning of your end step, you lose 1 life."` | MATCH |
| Front triggered_abilities | Upkeep trigger | `TriggerKind::Upkeep` | MATCH |
| Back triggered_abilities | End step trigger | `TriggerKind::EndStep` | MATCH |
| Back face name | Unholy Fiend | `"Unholy Fiend".into()` | MATCH |
| Doc comment | 1/1 front, 3/3 back | `"Cloistered Youth {1}{W} 1/1 Human // Unholy Fiend 3/3 Horror."` | MATCH |

**Behavior verification:**

- "you may" is correctly optional: `on_upkeep` presents a `YesNo` choice via `AwaitingAction::ResolutionChoice` (line 80-88). Player can accept or decline.
- Declining transform: `on_yes_no_choice` with `yes=false` logs "chose not to transform" and returns (line 92-96). No state change.
- Accepting transform: calls `helpers::apply_transform` which flips `is_transformed`, updates name/keywords/subtypes (line 99).
- `apply_transform` helper correctly updates name, keywords, subtypes but NOT power/toughness (P/T handled via `dynamic_pt`).
- `on_end_step` correctly checks both `is_transformed` and `active_player == controller` before applying life loss (lines 106-113).
- Life loss correctly emits `LifeChanged` event with old and new values (line 118).
- `on_upkeep` guards against firing when already transformed via `if !is_transformed` (line 78).
- `on_end_step` guards against firing when not transformed via `if is_transformed` (line 113).

### Tricky interactions checked
- Upkeep trigger only fires for controller (active_player check): PASS
- End step trigger only fires for controller (active_player check): PASS
- "You may" is properly optional via YesNo choice: PASS
- Life loss emits LifeChanged event: PASS
- Back face P/T via dynamic_pt: PASS (returns (3, 3) when transformed, None otherwise)
- Already-transformed creature does not re-present transform choice at upkeep: PASS
- Non-transformed creature does not lose life at end step: PASS
- Transform updates name/subtypes/keywords via apply_transform helper: PASS
- Trigger system queues upkeep trigger for front face correctly: PASS (trigger_description finds Upkeep in front face triggered_abilities)
- Trigger system queues end step trigger for back face correctly: PASS (trigger_description finds EndStep in back face triggered_abilities when is_transformed)

### Test coverage
- Transform at upkeep with player choice (yes): `tier15_cards.rs:788` (cloistered_youth_presents_transform_choice_at_upkeep) -- verifies awaiting_action is set, player chooses yes, creature transforms, name changes, dynamic_pt returns (3,3)
- Declining to transform: `tier15_cards.rs:815` (cloistered_youth_can_decline_transform) -- verifies player can choose no, creature stays untransformed
- Life loss at end step when transformed: `tier15_cards.rs:840` (unholy_fiend_drains_life_at_end_step) -- verifies life decreases by 1
- Front face has exactly one Upkeep trigger: `tier15_cards.rs:857` (cloistered_youth_front_face_has_upkeep_trigger_only)
- Back face has exactly one EndStep trigger: `tier15_cards.rs:869` (unholy_fiend_back_face_has_end_step_trigger_only)
- LifeChanged event explicitly verified in events list: NOT TESTED (life total is checked but event vector is not inspected)
- No life loss when NOT transformed at end step: NOT TESTED (implicitly covered by the transform choice tests, but no dedicated test)
- Card NOT in LLM card knowledge section (`mtg-player/src/llm.rs`): minor gap for AI player guidance

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text (front)**: At the beginning of your upkeep, you may transform this creature.
**Oracle text (back)**: At the beginning of your end step, you lose 1 life.
**Type line (front)**: Creature — Human
**Type line (back)**: Creature — Horror
**Mana cost**: {1}{W}
**P/T (front)**: 1/1
**P/T (back)**: 3/3
**Status**: PASS

### Code issues
No issues found. All card data and behavior match oracle text.

**Card data verification:**

| Field | Oracle | Code | Match? |
|-------|--------|------|--------|
| Name (front) | Cloistered Youth | `"Cloistered Youth".into()` | YES |
| Name (back) | Unholy Fiend | `"Unholy Fiend".into()` | YES |
| Mana cost | {1}{W} | `Generic(1), Colored(Color::White)` | YES |
| Card type (both) | Creature | `CardType::Creature` | YES |
| Subtypes (front) | Human | `vec!["Human".into()]` | YES |
| Subtypes (back) | Horror | `vec!["Horror".into()]` | YES |
| P/T (front) | 1/1 | `power: Some(1), toughness: Some(1)` | YES |
| P/T (back) | 3/3 | `power: Some(3), toughness: Some(3)` + `dynamic_pt` returns `(3,3)` when transformed | YES |
| Oracle text (front) | "At the beginning of your upkeep, you may transform this creature." | `"At the beginning of your upkeep, you may transform Cloistered Youth."` | YES (card name vs "this creature" is acceptable) |
| Oracle text (back) | "At the beginning of your end step, you lose 1 life." | `"At the beginning of your end step, you lose 1 life."` | YES |
| Front triggered_abilities | Upkeep trigger | `TriggerKind::Upkeep` (1 entry) | YES |
| Back triggered_abilities | End step trigger | `TriggerKind::EndStep` (1 entry) | YES |
| Keywords | Transform (keyword action, not a gameplay keyword) | `keywords: vec![]` | YES (consistent with all DFCs in codebase; no `Keyword::Transform` variant exists) |

**Behavior verification:**

- **"You may" transform is optional**: `on_upkeep` (line 80-88) sets `AwaitingAction::ResolutionChoice` with `YesNo` choice. Player can accept or decline. Code: `ResolutionChoiceKind::YesNo { description: "Cloistered Youth: transform into Unholy Fiend?".into(), ... }` -- CORRECT
- **Declining transform**: `on_yes_no_choice` with `yes=false` (line 92-96) logs and returns with no state change -- CORRECT
- **Accepting transform**: calls `helpers::apply_transform` (line 99) which sets `is_transformed`, updates name/keywords/subtypes -- CORRECT
- **Life loss is mandatory**: `on_end_step` (line 113-121) deducts 1 life with no player choice when `is_transformed` is true -- CORRECT
- **Life loss is loss, not damage**: code directly modifies `life` field (line 117: `old - 1`) and emits `LifeChanged` event, not a damage event -- CORRECT per oracle "you lose 1 life"
- **Controller check on upkeep**: line 75-77 `if state.active_player != controller { return; }` -- CORRECT
- **Controller check on end step**: line 110-112 same guard -- CORRECT
- **Guard against double-transform**: line 78 `if !is_transformed` prevents presenting choice when already transformed -- CORRECT
- **Guard against life loss when untransformed**: line 113 `if is_transformed` -- CORRECT

### Tricky interactions checked
- Transform does not cause zone change (no ETB/LTB triggers) -- handled by `apply_transform` helper which only mutates in-place: PASS
- Summoning sickness preserved across transform (same permanent): not explicitly relevant here but consistent with DFC rules: PASS
- Upkeep trigger fires only for controller's upkeep: PASS (active_player check)
- End step trigger fires only for controller's end step: PASS (active_player check)
- Life loss emits `LifeChanged` event (line 118): PASS
- `dynamic_pt` returns `None` when not transformed, allowing base P/T to apply: PASS

### Test coverage
Five tests in `mtg-engine/tests/tier15_cards.rs`:
1. `cloistered_youth_presents_transform_choice_at_upkeep` (line 788) -- verifies YesNo choice presented, transformation on yes
2. `cloistered_youth_can_decline_transform` (line 815) -- verifies no transformation on decline
3. `unholy_fiend_drains_life_at_end_step` (line 840) -- verifies 1 life lost
4. `cloistered_youth_front_face_has_upkeep_trigger_only` (line 857) -- verifies exactly 1 Upkeep trigger on front
5. `unholy_fiend_back_face_has_end_step_trigger_only` (line 869) -- verifies exactly 1 EndStep trigger on back

**Minor gap**: No LLM knowledge entry in `mtg-player/src/llm.rs` (other DFCs like Delver of Secrets and Screeching Bat have entries). Not a correctness issue.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: At the beginning of your upkeep, you may transform this creature.
// Unholy Fiend: At the beginning of your end step, you lose 1 life.
**Type line**: Creature — Human // Creature — Horror
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:41

**Oracle text source**: Scryfall API (cached 2026-04-01, via `scripts/oracle_lookup.py`)
**Oracle text (front)**: "At the beginning of your upkeep, you may transform this creature."
**Oracle text (back)**: "At the beginning of your end step, you lose 1 life."
**Type line (front)**: Creature — Human
**Type line (back)**: Creature — Horror
**Mana cost**: {1}{W}
**P/T (front)**: 1/1
**P/T (back)**: 3/3
**Status**: PASS

### Code issues

No issues found. All card data and behavior match oracle text exactly.

**Card data verification (oracle vs code):**

| Field | Oracle (Scryfall) | Code (`cloistered_youth.rs`) | Match? |
|-------|-------------------|------------------------------|--------|
| Name (front) | Cloistered Youth | `"Cloistered Youth".into()` | YES |
| Name (back) | Unholy Fiend | `"Unholy Fiend".into()` | YES |
| Mana cost | {1}{W} | `Generic(1), Colored(Color::White)` | YES |
| Card type (both faces) | Creature | `CardType::Creature` | YES |
| Subtypes (front) | Human | `vec!["Human".into()]` | YES |
| Subtypes (back) | Horror | `vec!["Horror".into()]` | YES |
| P/T (front) | 1/1 | `power: Some(1), toughness: Some(1)` | YES |
| P/T (back) | 3/3 | `power: Some(3), toughness: Some(3)` + `dynamic_pt` returns `(3,3)` when transformed | YES |
| Oracle text (front) | "At the beginning of your upkeep, you may transform this creature." | `"At the beginning of your upkeep, you may transform Cloistered Youth."` (line 26) | YES (card name vs "this creature" is standard) |
| Oracle text (back) | "At the beginning of your end step, you lose 1 life." | `"At the beginning of your end step, you lose 1 life."` (line 48) | YES |
| Front triggered_abilities | Upkeep trigger | `TriggerKind::Upkeep` (1 entry, line 31) | YES |
| Back triggered_abilities | End step trigger | `TriggerKind::EndStep` (1 entry, line 54) | YES |
| Back face cost | None | `cost: None` (line 42) | YES |
| Keywords | Transform (keyword action) | `keywords: vec![]` | YES (Transform is not a keyword ability; no `Keyword::Transform` variant in engine) |

**Behavior verification:**

- **"You may" transform is optional**: `on_upkeep` (lines 80-88) sets `AwaitingAction::ResolutionChoice` with `YesNo` choice. Player can accept or decline. CORRECT.
- **Declining transform**: `on_yes_no_choice` with `yes=false` (lines 92-96) logs and returns with no state change. CORRECT.
- **Accepting transform**: calls `helpers::apply_transform` (line 99) which flips `is_transformed`, updates name/keywords/subtypes. CORRECT.
- **Life loss is mandatory**: `on_end_step` (lines 113-121) deducts 1 life with no player choice when `is_transformed` is true. CORRECT.
- **Life loss is loss, not damage**: code directly modifies `life` field (`old - 1`, line 117) and emits `LifeChanged` event, not a damage event. CORRECT per oracle "you lose 1 life".
- **Controller check on upkeep**: lines 75-77 `if state.active_player != controller { return; }`. CORRECT.
- **Controller check on end step**: lines 110-112 same guard. CORRECT.
- **Guard against double-transform**: line 78 `if !is_transformed` prevents presenting choice when already transformed. CORRECT.
- **Guard against life loss when untransformed**: line 113 `if is_transformed`. CORRECT.
- **`should_transform` returns false**: line 124-126. CORRECT (this card uses a "you may" trigger, not automatic transform like werewolves).
- **Doc comment** (line 7): `"Cloistered Youth {1}{W} 1/1 Human // Unholy Fiend 3/3 Horror."` matches oracle. CORRECT.

### Tricky interactions checked (min 3)

1. **Transform does not cause zone change**: `apply_transform` (helpers.rs:231-265) only mutates fields in place (`is_transformed`, `name`, `keywords`, `subtypes`). No zone change occurs, so no ETB/LTB triggers fire. PASS.
2. **Life loss vs damage distinction**: Oracle says "you lose 1 life." Code uses `state.get_player_mut(controller).life = new_life` with `new_life = old - 1` (line 117). This is life loss, not damage, so it cannot be prevented by damage prevention effects and does not trigger "whenever damage is dealt" abilities. PASS.
3. **Upkeep trigger fires only during controller's upkeep, not opponent's**: Line 75-77 checks `state.active_player != controller`. If the opponent is the active player, the trigger does not fire. PASS.
4. **End step trigger fires only during controller's end step**: Lines 110-112 same check. PASS.
5. **Already-transformed creature does not get re-prompted at upkeep**: Line 78 `if !is_transformed` guards against this. PASS.
6. **Subtypes change on transform**: Front is Human, back is Horror. `apply_transform` updates `obj.subtypes` from back_face_data. This means tribal effects (e.g., "all Humans get +1/+1") correctly stop applying after transform. PASS.

### Test coverage

All 5 tests pass (`cargo test --test tier15_cards cloistered` and `cargo test --test tier15_cards unholy`):

1. `cloistered_youth_presents_transform_choice_at_upkeep` (tier15_cards.rs:1033) -- verifies YesNo choice is presented, transformation occurs on yes, name changes to "Unholy Fiend", dynamic_pt returns (3,3).
2. `cloistered_youth_can_decline_transform` (tier15_cards.rs:1060) -- verifies creature stays untransformed when player declines.
3. `unholy_fiend_drains_life_at_end_step` (tier15_cards.rs:1085) -- verifies life decreases by 1 when transformed.
4. `cloistered_youth_front_face_has_upkeep_trigger_only` (tier15_cards.rs:1102) -- verifies exactly 1 Upkeep trigger on front face.
5. `unholy_fiend_back_face_has_end_step_trigger_only` (tier15_cards.rs:1113) -- verifies exactly 1 EndStep trigger on back face.

**Minor gaps (not blocking):**
- No dedicated test for "no life loss when NOT transformed at end step" (implicitly covered by other tests).
- No test verifying `LifeChanged` event is in the events vector (life total is asserted but event list is not inspected).

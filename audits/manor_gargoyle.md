# Audit: Manor Gargoyle

## Oracle (Official)
- **Name:** Manor Gargoyle
- **Cost:** {5}
- **Type:** Artifact Creature — Gargoyle
- **Oracle:** Defender. Manor Gargoyle is indestructible as long as it has defender. {1}: Until end of turn, Manor Gargoyle loses defender and gains flying.
- **P/T:** 4/4

## Implementation
- Name: "Manor Gargoyle" -- CORRECT
- Cost: {5} -- CORRECT
- Types: [Artifact, Creature] -- CORRECT
- Subtypes: ["Gargoyle"] -- CORRECT
- P/T: 4/4 -- CORRECT
- Keywords: [Defender] -- CORRECT
- Oracle text matches -- CORRECT
- Conditional indestructible when it has defender via ConditionalKeyword -- CORRECT
- Activated ability {1}: loses defender, gains flying until end of turn -- CORRECT
- Uses until_end_of_turn_removed_keywords for removing defender -- CORRECT
- Uses UntilEndOfTurnKeyword for granting flying -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Manor Gargoyle
- **Cost:** {5}
- **Type:** Artifact Creature — Gargoyle
- **P/T:** 4/4
- **Oracle Text:** Defender / This creature has indestructible as long as it has defender. / {1}: Until end of turn, this creature loses defender and gains flying.

### Card Data Checks
- [x] Name: "Manor Gargoyle" — correct
- [x] Cost: {5} — correct
- [x] Types: Artifact, Creature — correct
- [x] Subtypes: Gargoyle — correct
- [x] P/T: 4/4 — correct
- [x] Keywords: Defender — correct
- [ ] Oracle text: minor mismatch (cosmetic)
  - **Oracle:** `"This creature has indestructible as long as it has defender."` / `"{1}: Until end of turn, this creature loses defender and gains flying."`
  - **Implementation:** `"Manor Gargoyle has indestructible as long as it has defender."` / `"{1}: Until end of turn, Manor Gargoyle loses defender and gains flying."`
  - Note: Scryfall uses modern "this creature" templating; implementation uses card name. Functionally equivalent.

### Behavior Checks
- [x] Defender keyword present — correct
- [x] Conditional indestructible (when has Defender) via `ContinuousEffect::ConditionalKeyword` — correct
- [x] Activated ability costs {1} — correct
- [x] Ability only available on battlefield — correct
- [x] Does not require tap — correct
- [x] Grants flying until end of turn — correct
- [x] Removes defender until end of turn — correct (which also removes indestructible via the conditional)

### Result: PASS

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/228/manor-gargoyle)
**Oracle text**: `Defender\nThis creature has indestructible as long as it has defender.\n{1}: Until end of turn, this creature loses defender and gains flying.`
**Type line**: `Artifact Creature — Gargoyle`
**Status**: ISSUE

### Code issues

1. **Test asserts on the wrong thing (passes vacuously):**
   In `mtg-engine/tests/tier15_cards.rs` line 750, the test checks:
   ```rust
   let obj = state.get_object(gargoyle).unwrap();
   assert!(!obj.keywords.contains(&Keyword::Defender),
       "Manor Gargoyle should lose Defender after activation");
   ```
   This checks `obj.keywords` (the raw object-level keyword vector), which is always empty for cards placed by `named_creature` (it never populates keywords from card_data). The assertion passes vacuously -- it would pass even if the ability did nothing. The correct check is:
   ```rust
   assert!(!state.has_keyword(gargoyle, Keyword::Defender, &reg),
       "Manor Gargoyle should lose Defender after activation");
   ```

2. **Missing test coverage for the indestructible conditional interaction:**
   No test verifies that Manor Gargoyle:
   - Has indestructible while it has defender (the key conditional behavior).
   - Loses indestructible when it loses defender via ability activation.
   - Can be destroyed by lethal damage after losing defender mid-turn (the key ruling).

3. **Oracle text cosmetic mismatch:**
   Scryfall oracle uses `"This creature"` (modern templating); implementation uses `"Manor Gargoyle"`. Functionally equivalent, cosmetic only.

### Tricky interactions checked (min 3)

1. **Lethal damage while indestructible, then losing indestructible:**
   Per ruling (2013-07-01), if Manor Gargoyle takes lethal damage while indestructible, and then its ability is activated (losing defender and thus indestructible), SBAs will destroy it. The engine handles this correctly: `try_destroy` in SBA calls `has_keyword(Indestructible)`, which calls `has_conditional_keyword`, which calls `check_condition(SelfHasKeyword(Defender))`, which checks `until_end_of_turn_removed_keywords` and finds Defender removed. So indestructible is gone, and destruction proceeds.

2. **Infinite recursion avoidance in conditional keyword check:**
   `has_keyword(Indestructible)` -> `has_conditional_keyword` -> `check_condition(SelfHasKeyword(Defender))`. This path avoids infinite recursion by checking base keywords directly (obj.keywords + card_data.keywords) in `check_condition`, rather than calling `has_keyword` recursively. Verified correct.

3. **Multiple activations in a turn:**
   The ability has `once_per_turn: false` and `requires_tap: false`, so it can be activated multiple times. This is correct per MTG rules -- activating it a second time is redundant but legal. The implementation handles this safely (pushing duplicate entries to the until-EOT vectors is harmless).

4. **End-of-turn cleanup restores defender:**
   At cleanup step, `until_end_of_turn_removed_keywords.clear()` and `until_end_of_turn_keywords.clear()` are called, restoring defender and removing flying. This means Manor Gargoyle returns to being a 4/4 with defender and indestructible at end of turn. Verified correct.

### Test coverage

- `manor_gargoyle_loses_defender_and_gains_flying` -- tests activation but assertion on defender loss is vacuous (checks wrong field). Flying grant check is correct.
- No test for indestructible conditional behavior.
- No test for survives-lethal-damage-while-indestructible scenario.
- No test for dies-after-losing-indestructible-with-marked-damage scenario.

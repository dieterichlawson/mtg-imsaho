# Audit: Caravan Vigil

## Scryfall Reference
- **Name:** Caravan Vigil
- **Cost:** {G}
- **Type:** Sorcery
- **Oracle:** Search your library for a basic land card, reveal it, put it into your hand, then shuffle. Morbid -- You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
- **P/T:** N/A
- **Keywords:** Morbid

## Implementation: `caravan_vigil.rs`
- **Name:** Caravan Vigil -- CORRECT
- **Cost:** {G} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Subtypes:** none -- CORRECT
- **P/T:** N/A -- CORRECT
- **Keywords:** [] -- CORRECT (Morbid is an ability word, not a keyword mechanic)
- **Behavior:** Searches for basic land, puts in hand; Morbid puts on battlefield -- CORRECT
- **Oracle text note:** Implementation oracle text says "then shuffle your library" vs Scryfall "then shuffle" (minor wording modernization, not functional)

## Issues

### Issue 1: Morbid choice is forced — missing "you may" optionality (Medium)

**Oracle text:** "You may put that card onto the battlefield **instead** of putting it into your hand if a creature died this turn."

**Ruling (2011-09-22):** "You can choose to put the basic land card into your hand even if a creature died the turn you cast Caravan Vigil."

**Code (caravan_vigil.rs lines 58-63):**
```rust
if state.creature_died_this_turn {
    // Morbid: "You may put that card onto the battlefield instead."
    // Auto-choose battlefield (strictly better in almost all cases).
    state.move_object(land_id, Zone::Battlefield);
```

The implementation always puts the land onto the battlefield when morbid is active. The comment acknowledges this ("Auto-choose battlefield (strictly better in almost all cases)") but it removes player agency. There are game situations where putting a land into hand is preferable (e.g., opponent has an Ankh of Mishra, or player wants to avoid triggering landfall for the opponent). The player should be given the choice.

### Issue 2 (Engine Limitation): Library search is deterministic, not player-chosen

The code uses `.find()` (line 39) to pick the first basic land in library order. Ideally the player should choose which basic land to get. This is a known engine-wide limitation for library search effects.

### Issue 3 (Engine Limitation): No reveal step

Oracle says "reveal it" but the engine has no reveal mechanism. Known limitation.

## Resolve Behavior
- `move_spell_after_resolve` is called (line 83). Correct for a sorcery.
- Library is shuffled after the search (lines 71-73, 78-80), including when no card is found. Correct.

## Tests
- `caravan_vigil_finds_basic_land` — verifies non-morbid puts land in hand. Correct.
- `caravan_vigil_morbid_puts_land_on_battlefield` — verifies morbid puts land on battlefield. Tests the current (forced) behavior, not the "you may" choice.
- No test for the case where morbid is active but the player chooses hand instead.

## Verdict
One functional bug (forced morbid instead of optional). Two engine-limitation notes.

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
**Type line**: Sorcery
**Status**: PASS

### Code issues

The major issue from the prior audit (forced morbid) has been fixed:

1. **Morbid "you may" choice: FIXED.** Lines 58-75 now present a `YesNo` choice to the player via `AwaitingAction::ResolutionChoice` when `creature_died_this_turn` is true. The `on_yes_no_choice` handler (lines 99-126) correctly puts the land on the battlefield if yes, or into hand if no. This matches the oracle text "You may put that card onto the battlefield instead" and the ruling "You can choose to put the basic land card into your hand even if a creature died."

2. **Card data correct.** Cost `{G}` (line 18), type Sorcery (line 21).

3. **Non-morbid path correct.** Lines 77-81 put the land into hand when no creature died this turn.

4. **Library shuffle correct.** Shuffle occurs in all paths: non-morbid (lines 84-86), no-land-found (lines 91-93), and after yes/no choice (lines 121-123).

5. **Oracle text field (minor).** Code says "then shuffle your library" vs oracle "then shuffle". This is a modernized wording difference with no gameplay impact.

6. **Engine limitations unchanged.** Library search is deterministic (`.find()` picks first match), and there is no reveal mechanism.

### Tricky interactions checked
- Empty library (no basic land found): handled at line 87, still shuffles.
- Morbid choice deferred correctly: `move_spell_after_resolve` is called only after the choice is made (line 125), not before.

### Test coverage
- `caravan_vigil_finds_basic_land` -- non-morbid path.
- `caravan_vigil_morbid_puts_land_on_battlefield` -- morbid path.
- No test for morbid-active-but-player-chooses-hand path (would be good to add).

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.\nMorbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues
1. **Oracle text mismatch**: Oracle says "then shuffle" but code oracle_text says "then shuffle your library." The oracle has been updated to modern template wording. No gameplay impact — behavior is correct.

## Audit — 2026-04-02 (final-pass)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found. Oracle text field matches current Scryfall template.

## Audit — 2026-04-02 20:41
**Oracle text source**: Scryfall API (via oracle_lookup.py, cached 2026-04-01)
**Oracle text**: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No functional issues found.

1. **Card data correct.** Name "Caravan Vigil", cost `{G}` (line 18), type Sorcery (line 21), no subtypes, no P/T. All match oracle.

2. **Oracle text field matches.** Code oracle_text (line 25) matches Scryfall verbatim: "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.\nMorbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn."

3. **Non-morbid path correct.** When `creature_died_this_turn` is false (line 58), the land is moved to Hand (line 78) and library is shuffled (lines 84-86). Matches oracle "put it into your hand, then shuffle."

4. **Morbid choice correct.** When `creature_died_this_turn` is true, a YesNo choice is presented (lines 64-74) via `AwaitingAction::ResolutionChoice`. The `on_yes_no_choice` handler (lines 99-126) puts the land on the battlefield if yes (line 111), or into hand if no (line 115), then shuffles (lines 121-123). Matches oracle "You may put that card onto the battlefield instead" and ruling: "You can choose to put the basic land card into your hand even if a creature died the turn you cast Caravan Vigil."

5. **Spell cleanup correct.** `move_spell_after_resolve` is called at end of all paths: non-morbid (line 96), no-land-found (line 96), and after YesNo choice (line 125). In the morbid path, `on_resolve` returns early (line 76) so the spell stays on the stack until the choice resolves.

6. **No-land-found path correct.** If no basic land is found in the library (line 87), the library is still shuffled (lines 91-93) per rules (you searched).

7. **Engine limitations (not bugs).** Library search uses `.find()` (line 39) picking the first basic land in library order rather than letting the player choose. No reveal mechanism. Both are codebase-wide limitations.

### Tricky interactions checked (min 3)
1. **Morbid active but player declines:** Tested in `on_yes_no_choice` with `yes=false` -- land goes to hand, library shuffles. Matches the official ruling.
2. **Empty library / no basic land found:** Handled at line 87; logs "no basic land found" and still shuffles. No crash or undefined behavior.
3. **Spell object lifetime during deferred choice:** In the morbid path, `move_spell_after_resolve` is NOT called in `on_resolve` (early return at line 76), so the spell object and its `card_state` remain accessible when `on_yes_no_choice` runs. The land_id is stored on the spell via `card_state.insert("morbid_land", land_id)` (line 62) and retrieved in `on_yes_no_choice` (line 100). If retrieval fails, the handler gracefully cleans up the spell (line 103).
4. **Land removed from library_order before choice:** The land is removed from `library_order` at line 56 before the YesNo choice is presented. This means the shuffle in `on_yes_no_choice` (line 123) won't accidentally shuffle the found land back in. Correct behavior.

### Test coverage
- `caravan_vigil_finds_basic_land` -- non-morbid path: land goes to hand. PASS.
- `caravan_vigil_morbid_choose_battlefield` -- morbid path with yes choice: land goes to battlefield. PASS.
- Missing: test for morbid-active-but-player-chooses-no (land goes to hand). Would be good to add for completeness.

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

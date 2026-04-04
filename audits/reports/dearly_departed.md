# Audit: Dearly Departed

## Scryfall Reference
- **Name:** Dearly Departed
- **Cost:** {4}{W}{W}
- **Type:** Creature -- Spirit
- **Oracle:** Flying. As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
- **P/T:** 5/5
- **Keywords:** Flying

## Implementation: `dearly_departed.rs`
- **Name:** Dearly Departed -- CORRECT
- **Cost:** {4}{W}{W} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Spirit"] -- CORRECT
- **P/T:** 5/5 -- CORRECT
- **Keywords:** [Flying] -- CORRECT
- **Trigger:** AnyCreatureEnters -- CORRECT
- **Behavior:** When in graveyard, Human creatures entering under your control get +1/+1 counter -- CORRECT
- **Zone check:** Checks self is in Graveyard -- CORRECT
- **Human check:** Checks subtypes via registry and object -- CORRECT

## Issues
None

---

## Audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
```
Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
```

### Oracle Text String Mismatch (cosmetic)
- **Oracle:** `"As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it."`
- **Implementation:** `"As long as Dearly Departed is in your graveyard, Human creatures you control enter the battlefield with an additional +1/+1 counter on them."`
- Differences: "this creature" vs "Dearly Departed"; "each Human creature" vs "Human creatures"; "enters" vs "enter the battlefield"; "on it" vs "on them". Functionally equivalent.

### Triggered Ability vs. Replacement Effect
- **Oracle:** "enters with an additional +1/+1 counter" is a replacement effect modifying how the creature enters the battlefield.
- **Implementation:** Uses `TriggerKind::AnyCreatureEnters` / `on_any_creature_enters` — a triggered ability that fires after the creature has entered.
- **Impact:** In most cases the result is the same. The difference matters for interactions that care about the creature's state as it enters (e.g., Doubling Season doubling counters on replacement effects, or state-based actions checking toughness at the moment of entry). May be an engine limitation.

### Behavior Checks
- **Zone check:** PASS. Checks `o.zone == Zone::Graveyard` (line 43).
- **Owner check:** PASS. Uses `self_obj.owner` and compares against `entered_controller` (lines 45-48).
- **Human subtype check:** PASS. Dual check on registry data and runtime object subtypes (lines 52-58).
- **Counter placement:** PASS. Adds 1 `PlusOnePlusOne` counter (line 60).
- **Cumulative stacking:** PASS. Multiple graveyard copies each independently trigger per ruling.

### Test Coverage
- `dearly_departed_gives_counter_to_entering_humans` — positive case. PASS.
- **Missing tests:** non-Human entering (negative), opponent's Human entering (negative), Dearly Departed on battlefield not in graveyard (negative).

### Summary
- Functionally correct for standard gameplay.
- Triggered ability used instead of replacement effect — low severity for typical play, incorrect for precise rules interactions.
- Oracle text string has minor cosmetic wording differences.
- Test coverage limited to positive case only.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying\nAs long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
**Type line**: Creature — Spirit
**Status**: ISSUE

### Code issues
1. Oracle text mismatch: code uses older template `"As long as Dearly Departed is in your graveyard, Human creatures you control enter the battlefield with an additional +1/+1 counter on them."` but current oracle uses `"As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it."` The wording changed but is semantically equivalent.
2. Behavior is correct: on_any_creature_enters checks that Dearly Departed is in the graveyard (Zone::Graveyard), that the entering creature is controlled by the owner of Dearly Departed, and that the creature is a Human (via subtypes). Adds one +1/+1 counter. Flying keyword is present. Cost {4}{W}{W}, P/T 5/5, subtype Spirit all match.

## Audit — 2026-04-02 (final-pass)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found. Oracle text field matches current Scryfall template.

## Audit — 2026-04-02 20:50
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Flying\nAs long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
**Type line**: Creature — Spirit
**Status**: ISSUE

### Code issues
1. **CRITICAL: Trigger system never fires for graveyard-based watcher.** The trigger system in `triggers.rs` line 369 only collects watchers with `o.zone == Zone::Battlefield`. Dearly Departed's ability works from the graveyard, so it is never discovered as a watcher when a creature enters. Additionally, the resolution gate at line 915 (`o.zone == Zone::Battlefield`) would reject it even if it were found. The unit test masks this by calling `on_any_creature_enters` directly, bypassing the trigger system entirely. In actual integrated gameplay, the ability would never fire.
2. **Replacement effect modeled as triggered ability.** The oracle text "enters with an additional +1/+1 counter" describes a replacement effect (modifying how a creature enters), not a triggered ability (firing after it enters). The implementation uses `TriggerKind::AnyCreatureEnters` / `on_any_creature_enters`. This matters for Doubling Season interactions and state-based action timing, but may be an engine limitation.
3. **Card data is correct.** Name, mana cost ({4}{W}{W}), type (Creature), subtype (Spirit), P/T (5/5), keywords (Flying), and oracle text all match Scryfall exactly.

### Tricky interactions checked (min 3)
1. **Cumulative stacking (multiple Dearly Departed in graveyard):** Each copy independently triggers `on_any_creature_enters`. Since the trigger system scans all objects, multiple copies would each add a counter -- correct per ruling "The effect is cumulative." However, due to Issue #1, none of them fire in practice.
2. **Opponent's Human creatures should not get counters:** The implementation checks `entered_controller != owner` and returns early -- correct. Opponent's Humans are excluded.
3. **Dearly Departed on the battlefield (not in graveyard) should not trigger:** The implementation checks `o.zone == Zone::Graveyard` and returns early if not in graveyard -- correct.
4. **Loyal Cathar interaction (creature dying and returning as Human):** Per MTG Salvation ruling, Loyal Cathar returning from graveyard as a Human should get the counter. The `on_any_creature_enters` handler would correctly apply, provided the trigger system fires (which it does not due to Issue #1).

### Test coverage
- `dearly_departed_gives_counter_to_entering_humans` -- positive case, calls `on_any_creature_enters` directly. PASS.
- **Missing:** Negative test for non-Human creature entering (should not get counter).
- **Missing:** Negative test for opponent's Human entering (should not get counter).
- **Missing:** Negative test for Dearly Departed on battlefield (not in graveyard).
- **Missing:** Integration test through the trigger system (would reveal Issue #1).

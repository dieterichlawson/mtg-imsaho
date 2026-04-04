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

## Audit — 2026-04-03 22:06

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying\nAs long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
**Type line**: Creature — Spirit
**Status**: ISSUE

### Code issues
- **CRITICAL: Engine trigger dispatch only scans Battlefield, so Dearly Departed's graveyard ability never fires in integrated gameplay.** The trigger dispatch in `mtg-engine/src/triggers.rs` line 369 filters watchers with `o.zone == Zone::Battlefield`. Dearly Departed's oracle text says its ability works "As long as this creature is in your graveyard," meaning it must be in the graveyard to function. The engine never discovers it as a watcher.
  - Oracle text says: `As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.`
  - Code does: `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != *object)` (triggers.rs:369) — only Battlefield objects are scanned as watchers, graveyard objects are excluded.
- **CRITICAL: Engine trigger resolution also gates on Battlefield.** Even if the trigger were somehow queued, `mtg-engine/src/triggers.rs` line 915 checks `state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before calling `on_any_creature_enters`. This is a second barrier preventing graveyard-based abilities from resolving.
  - Oracle text says: `As long as this creature is in your graveyard`
  - Code does: `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` (triggers.rs:915) — rejects any watcher not on the Battlefield.
- **Replacement effect modeled as triggered ability.** The oracle text "enters with an additional +1/+1 counter on it" is a replacement effect (the counter is placed simultaneously with the creature entering). The implementation uses `TriggerKind::AnyCreatureEnters` / `on_any_creature_enters`, which fires after the creature has entered. This means other ETB triggers (e.g., Mentor of the Meek) would see the creature without the counter, which is incorrect per MTG rules.
  - Oracle text says: `each Human creature you control enters with an additional +1/+1 counter on it`
  - Code does: `triggered_abilities: vec![TriggeredAbilityDef { kind: TriggerKind::AnyCreatureEnters, ... }]` (dearly_departed.rs:30-35) — modeled as a triggered ability, not a replacement effect.
- **Test bypasses the trigger system.** The test at `mtg-engine/tests/tier15_cards.rs:62` calls `behavior.on_any_creature_enters()` directly, masking the fact that the engine would never dispatch this trigger for a graveyard-based card.
  - Code does: `behavior.on_any_creature_enters(&mut state, dd, human, P0, &reg);` (tier15_cards.rs:62) — direct call, not via the trigger system.

### Tricky interactions checked
- Cumulative stacking (multiple Dearly Departed in graveyard): fail — the card handler logic is correct (each copy would independently add a counter), but the trigger dispatch never reaches graveyard objects, so zero copies fire.
- Opponent's Human creatures excluded: pass — handler correctly checks `entered_controller != owner` at line 47-48.
- Dearly Departed on battlefield (not in graveyard) does not trigger: pass — handler checks `o.zone == Zone::Graveyard` at line 42-43.
- Replacement effect timing (counter present as creature enters): fail — modeled as post-entry triggered ability rather than replacement effect. Other ETB triggers would see incorrect P/T.
- Non-Human creature entering does not get counter: pass — handler checks subtypes for "Human" at lines 52-58.
- Token Humans entering: pass — handler checks object subtypes as well as card_data subtypes at lines 52-58, covering tokens.

### Test coverage
- Positive case (Human entering with Dearly Departed in graveyard): `tier15_cards.rs:49` — but bypasses trigger system
- Negative case (non-Human entering): NOT TESTED
- Negative case (opponent's Human entering): NOT TESTED
- Negative case (Dearly Departed on battlefield, not in graveyard): NOT TESTED
- Integration test through trigger system: NOT TESTED (would reveal the critical dispatch bug)
- Cumulative stacking with multiple copies: NOT TESTED
- Replacement effect timing (counter visible to simultaneous ETB triggers): NOT TESTED

## Audit — 2026-04-03 22:06

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
**Type line**: Creature — Spirit
**Status**: ISSUE

### Code issues

- **Engine bug prevents card from working**: `mtg-engine/src/triggers.rs:915`
  - Oracle text says: `As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.`
  - Code does: Trigger dispatch system checks `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` which prevents graveyard-based triggers from ever firing.

- **Wrong implementation approach**: `mtg-engine/src/cards/isd/dearly_departed.rs:30-35`
  - Oracle text says: `enters with an additional +1/+1 counter` (replacement effect - counters present as creature enters)
  - Code does: Uses `TriggerKind::AnyCreatureEnters` triggered ability that would add counters after entering, but this never fires due to engine bug above.

- **Test bypasses engine trigger system**: `mtg-engine/tests/tier15_cards.rs:62`
  - Oracle text says: Card should work through normal game flow when Human creatures are cast
  - Code does: Test manually calls `behavior.on_any_creature_enters()` directly, bypassing trigger dispatch that would fail in real gameplay.

### Tricky interactions checked
- **Multiple Dearly Departed in graveyard (cumulative effect)**: FAIL - No mechanism exists for graveyard-based replacement effects
- **Timing vs triggered abilities**: FAIL - Should be replacement effect (counters on entry) not triggered (counters after entry)
- **Human creature type recognition**: PASS - Code correctly checks both card subtypes and object subtypes
- **Controller matching**: PASS - Code correctly verifies creature controller matches Dearly Departed owner
- **Zone verification**: PASS - Card code correctly checks if Dearly Departed is in graveyard (though this check is never reached)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic functionality (Human gets counter from graveyard Dearly Departed)**: `tier15_cards.rs:49` / INVALID TEST (bypasses engine)
- **Cumulative effect with multiple Dearly Departed**: NOT TESTED
- **Replacement effect timing (enters with vs added after)**: NOT TESTED
- **Only affects Human creatures**: NOT TESTED
- **Only affects creatures you control**: NOT TESTED
- **Integration with actual creature casting**: NOT TESTED

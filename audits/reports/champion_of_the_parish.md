# Audit: Champion of the Parish

## Scryfall Reference
- **Name:** Champion of the Parish
- **Cost:** {W}
- **Type:** Creature -- Human Soldier
- **Oracle:** Whenever another Human you control enters, put a +1/+1 counter on this creature.
- **P/T:** 1/1
- **Keywords:** none

## Implementation: `champion_of_the_parish.rs`
- **Name:** Champion of the Parish -- CORRECT
- **Cost:** {W} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Human", "Soldier"] -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Keywords:** none -- CORRECT
- **Trigger:** AnyCreatureEnters -- CORRECT
- **Self-exclusion ("another"):** Handled by trigger infrastructure in `triggers.rs` line 352: `.filter(|o| o.zone == Zone::Battlefield && o.id != *object)` excludes the entering creature from the watcher list. No card-level check needed. -- CORRECT
- **Controller check ("you control"):** `entered_controller != controller` guard at line 43. -- CORRECT
- **Human subtype check:** Checks both registry `card_data.subtypes` and instance `obj.subtypes` (lines 48-54), covering both registered cards and tokens. -- CORRECT
- **+1/+1 counter:** `state.add_counters(self_id, CounterType::PlusOnePlusOne, 1)` at line 56. -- CORRECT
- **Zone check:** Confirms Champion is on the battlefield before triggering (line 39). -- CORRECT

## Tests (`mtg-engine/tests/tier6_cards.rs`)
- `champion_of_the_parish_counter_on_human_etb` -- triggers on friendly Human ETB. PASS
- `champion_of_the_parish_no_counter_on_non_human` -- does not trigger on non-Human. PASS
- `champion_of_the_parish_no_counter_on_opponent_human` -- does not trigger on opponent's Human. PASS
- No test for self-entering (Champion entering while another Champion is already on the battlefield) but infrastructure handles this correctly.

## LLM hint (`mtg-player/src/llm.rs`)
- "Champion of the Parish ({W} creature 1/1): Gets a +1/+1 counter whenever another Human enters under your control. Play Humans to grow it!" -- CORRECT, matches oracle semantics.

## Issues

### Issue 1: Oracle text field uses outdated wording (cosmetic)

**Scryfall oracle (current):**
> Whenever another Human you control enters, put a +1/+1 counter on this creature.

**Implementation `oracle_text` field (line 23):**
> "Whenever another Human creature enters the battlefield under your control, put a +1/+1 counter on Champion of the Parish."

Differences: (a) includes "creature" which was removed in 2023 oracle update, (b) includes "the battlefield" which was removed in 2023 oracle update, (c) says "Champion of the Parish" instead of "this creature". This is cosmetic only and does not affect game behavior since the trigger logic correctly implements the intended semantics.

### Issue 2: Comment in doc-string uses outdated wording (cosmetic)

**Line 7:**
> `/// Whenever another Human creature enters the battlefield under your control,`

Same outdated wording as the oracle_text field. Cosmetic only.

## Verdict
**No functional bugs.** The implementation correctly handles all aspects of the card: self-exclusion via trigger infrastructure, controller check, Human subtype check (both registry and instance), and +1/+1 counter placement. Two cosmetic issues with outdated oracle text wording in the `oracle_text` field and doc comment.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Whenever another Human you control enters, put a +1/+1 counter on this creature.
**Type line**: Creature — Human Soldier
**Status**: ISSUE

### Code issues
1. **Oracle text mismatch**: Oracle says "Whenever another Human you control enters, put a +1/+1 counter on this creature." but code oracle_text says "Whenever another Human creature enters the battlefield under your control, put a +1/+1 counter on Champion of the Parish." The oracle has been updated to modern template (dropping "creature", "the battlefield", and using self-referential "this creature"). No gameplay impact — behavior is correct.

## Audit — 2026-04-02 (final-pass)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found. Oracle text field matches current Scryfall template.

## Audit — 2026-04-02 20:41
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Whenever another Human you control enters, put a +1/+1 counter on this creature.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
None. All card data fields (name, cost, types, subtypes, P/T, oracle text) match Scryfall exactly. Behavior implementation is correct:
- "Another" self-exclusion handled by trigger infrastructure (`triggers.rs` line 369: `o.id != *object`)
- "You control" enforced by controller check (line 42)
- "Human" check covers both registry data and runtime instance subtypes (lines 47-53)
- +1/+1 counter placed via `add_counters` (line 55)
- Zone check ensures Champion must be on battlefield (line 38)

### Tricky interactions checked (min 3)
1. **Self-trigger prevention**: Engine filter `o.id != *object` in trigger dispatch prevents Champion from triggering on its own ETB. No card-level guard needed.
2. **Opponent's Human ignored**: Controller comparison on line 42 ensures only Humans entering under Champion's controller's control trigger the ability. Covered by test.
3. **Non-Human creatures ignored**: Subtype check on lines 47-53 filters out creatures without the Human subtype. Covered by test.
4. **Runtime subtype changes**: Code checks both card registry subtypes AND object instance subtypes, correctly handling creatures that gain/lose Human subtype at runtime (e.g., via Moonmist transform).
5. **Off-battlefield Champion**: Zone check on line 38 returns early if Champion is not on the battlefield.

### Test coverage
- `champion_of_the_parish_counter_on_human_etb` — triggers on friendly Human ETB, gets +1/+1 counter. PASS
- `champion_of_the_parish_no_counter_on_non_human` — does not trigger on non-Human creature. PASS
- `champion_of_the_parish_no_counter_on_opponent_human` — does not trigger on opponent's Human. PASS
- Also used as a Human fixture in tests for: Butcher's Cleaver, Silver-Inlaid Dagger, Sharpened Pitchfork, Hamlet Captain, Night Revelers, Angelic Overseer, Dearly Departed.

## Audit — 2026-04-10 12:00

**Oracle text source**: Oracle cache (Scryfall API, cached 2026-04-01, URL https://scryfall.com/card/isd/6/champion-of-the-parish)
**Oracle text**: Whenever another Human you control enters, put a +1/+1 counter on this creature.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
No issues found.

Card data verification (`mtg-engine/src/cards/isd/champion_of_the_parish.rs`):
- Mana cost `{W}` matches.
- Card types `[Creature]` matches.
- Supertypes `[]` matches (not legendary).
- Subtypes `[Human, Soldier]` matches.
- Power/toughness `1/1` matches.
- Oracle text field matches verbatim.
- Keywords empty — no keyword abilities on the card.
- Triggered ability declared as `TriggerKind::AnyCreatureEnters` and implemented via `on_any_creature_enters`.

Behavior verification:
- "Another" semantics: trigger collection in `mtg-engine/src/triggers.rs:376` filters watchers with `o.id != *object`, so Champion will not self-trigger when it itself enters. Correct.
- "You control" semantics: the hook checks `entered_controller != controller` and bails out if mismatched (line 42-44).
- "Human" check: looks at both registry card data subtypes and the live object subtypes (line 47-53), correctly covering tokens and any instance overrides.
- "enters": uses the standard EnterWatch trigger path; `trigger_zones` defaults to Battlefield only, so a graveyard-zone Champion won't create a phantom trigger.
- `+1/+1` counter applied via `state.add_counters(self_id, CounterType::PlusOnePlusOne, 1)`.

### Tricky interactions checked
- Self-ETB (Champion's own ETB should not count): PASS — excluded at trigger collection via `o.id != *object`.
- Opponent's Human entering: PASS — explicit `entered_controller != controller` check.
- Non-Human creature entering: PASS — `is_human` gated.
- Champion in graveyard while Human enters: PASS — `trigger_zones` defaults to Battlefield, and hook also double-checks `zone == Battlefield`.
- Token Human (e.g., produced by another spell) entering: PASS — subtype check falls back to live object subtypes when registry data has no Human.
- Multiple Humans entering simultaneously (e.g., Mass ETB): trigger system collects one trigger per ETB event, so each qualifying entrant adds one counter. Not tested directly for Champion but consistent with generic trigger pipeline.

### Test coverage
- Human enters under our control -> counter: `mtg-engine/tests/tier6_cards.rs:87` (`champion_of_the_parish_counter_on_human_etb`).
- Non-Human entering does not trigger: `mtg-engine/tests/tier6_cards.rs:108` (`champion_of_the_parish_no_counter_on_non_human`).
- Opponent's Human does not trigger: `mtg-engine/tests/tier6_cards.rs:129` (`champion_of_the_parish_no_counter_on_opponent_human`).
- Champion in graveyard does not trigger: `mtg-engine/tests/phantom_triggers.rs:74` (`champion_in_graveyard_does_not_trigger`).
- Champion on battlefield does trigger: `mtg-engine/tests/phantom_triggers.rs:104` (`champion_on_battlefield_does_trigger`).
- Self-ETB exclusion (Champion does not counter itself on its own ETB): NOT TESTED directly, though the engine-level behavior is covered by the `o.id != *object` filter in `triggers.rs:376`.
- Simultaneous multi-Human ETB: NOT TESTED for Champion specifically.
- Token Human entering (e.g., Doomed Traveler's Spirit token is not a Human, but Mausoleum Guard etc. — token subtype path): NOT TESTED.

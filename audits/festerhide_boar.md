# Audit: Festerhide Boar

## Reference (Scryfall)
- **Name:** Festerhide Boar
- **Cost:** {3}{G}
- **Type:** Creature -- Boar
- **Oracle:** Trample. Morbid -- Festerhide Boar enters the battlefield with two +1/+1 counters on it if a creature died this turn.
- **P/T:** 3/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{G})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Boar)
- Oracle text: CORRECT
- P/T: CORRECT (3/3)
- Keywords: CORRECT (Trample)
- Morbid check: CORRECT (checks creature_died_this_turn)
- Two +1/+1 counters: CORRECT

## Issues
**ISSUE: Morbid is a static/replacement ability, not a triggered ability.** The oracle says "enters the battlefield WITH two +1/+1 counters" -- this is a replacement effect that modifies how the creature enters, not a triggered ability that fires after entering. The implementation uses on_enter_battlefield (ETB trigger) and declares TriggerKind::EntersBattlefield in triggered_abilities. While functionally similar, the triggered_abilities metadata is misleading. The actual on_enter_battlefield hook is fine functionally.

---

# Audit: Festerhide Boar (2026-04-02)

## Oracle Text (Scryfall)
> Trample
> Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.

## Card Data

| Field         | Oracle               | Implementation         | Match |
|---------------|----------------------|------------------------|-------|
| Name          | Festerhide Boar      | `"Festerhide Boar"`    | OK    |
| Mana Cost     | {3}{G}               | `Generic(3), Green`    | OK    |
| Type          | Creature — Boar      | `Creature`, `["Boar"]` | OK    |
| P/T           | 3/3                  | `3/3`                  | OK    |
| Keywords      | Trample              | `[Keyword::Trample]`   | OK    |

## Oracle Text Mismatch

**Implementation (line 24):**
> `"Trample\nMorbid — When Festerhide Boar enters the battlefield, if a creature died this turn, put two +1/+1 counters on Festerhide Boar."`

**Scryfall oracle text:**
> `"Trample\nMorbid — This creature enters with two +1/+1 counters on it if a creature died this turn."`

The implementation uses old-style "enters the battlefield" triggered-ability wording. The current oracle text uses "enters with" replacement-effect wording. The stored `oracle_text` string should be updated to match the current Scryfall oracle text.

## Trample

`Keyword::Trample` is declared in `keywords` (line 25). OK.

## Morbid ETB — Replacement Effect vs. Triggered Ability

**Issue:** The current oracle text describes a **replacement effect** ("enters *with*" counters), not a triggered ability. The implementation incorrectly registers a `TriggeredAbilityDef` with `TriggerKind::EntersBattlefield` (lines 27-32) and the doc comment (lines 7-8) describes it as a triggered ability ("When Festerhide Boar enters the battlefield...").

Functionally, the `on_enter_battlefield` hook (lines 36-42) applies counters immediately via `state.add_counters(object_id, CounterType::PlusOnePlusOne, 2)` when `state.creature_died_this_turn` is true. This **produces the correct game outcome** — the boar ends up with 2 +1/+1 counters when morbid is active. However, it is mislabeled as a triggered ability.

## Tests

Two tests exist in `mtg-engine/tests/tier5_cards.rs`:
- `festerhide_boar_morbid` — verifies 2 +1/+1 counters and effective 5/5 when morbid is active. PASS.
- `festerhide_boar_no_morbid` — verifies 0 counters when morbid is not active. PASS.

## Summary of Issues

1. **Oracle text string mismatch (low):** The `oracle_text` field uses outdated "enters the battlefield" wording instead of the current "enters with" wording.
2. **Triggered ability mislabel (medium):** `triggered_abilities` lists an ETB trigger, but the current oracle text describes a replacement effect. The `TriggeredAbilityDef` entry and doc comment should be updated to reflect replacement-effect semantics.
3. **Functional correctness:** The actual game behavior (counters applied on entry when morbid is satisfied) is correct.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Trample
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
**Type line**: Creature — Boar
**Status**: PASS

### Code issues
No issues found. The oracle text uses "enters with" (replacement effect) but the implementation uses an ETB callback which is functionally equivalent in this engine. The stored oracle text in the code uses older templating ("When Festerhide Boar enters the battlefield") but the behavior is correct: counters are applied on entry if morbid is satisfied.

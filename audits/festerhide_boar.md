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

## Audit — 2026-04-02 20:58

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Trample
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
**Type line**: Creature — Boar
**Status**: PASS

### Code issues

None found. All previous audit issues have been resolved:
- Oracle text string now matches current Scryfall wording exactly ("This creature enters with..." not the old "enters the battlefield" template).
- No spurious `TriggeredAbilityDef` entries -- `triggered_abilities: vec![]` is empty, correct for a replacement effect.
- Morbid is correctly implemented in `on_resolve` (not `on_enter_battlefield`), which means counters are placed as part of entering the battlefield, before any ETB triggers fire. This faithfully models the "enters with" replacement-effect semantics described in the oracle text and CR 614.1c.
- `Keyword::Trample` is declared; Morbid is an ability word (no rules meaning), correctly omitted from keywords.
- Card data fields (name, cost {3}{G}, type Creature -- Boar, P/T 3/3) all match oracle.

### Tricky interactions checked (min 3)

1. **Replacement effect vs triggered ability**: "Enters with" counters are placed in `on_resolve` after `move_object` but before any ETB triggers process. This means the Boar has its counters before any "when ~ enters" triggers see it, which is correct per CR 614.1c.
2. **Morbid tracks any creature dying, not just own creatures**: `creature_died_this_turn` is set by `destruction.rs` (line 100) and `sba.rs` (lines 96, 144) for any creature death, regardless of controller. This matches "a creature died this turn" without ownership restriction.
3. **Morbid resets each turn**: `creature_died_this_turn` is reset to `false` at the start of each turn in `engine.rs` (line 2888), so morbid only counts deaths in the current turn, not previous turns.
4. **Trample in combat**: `Keyword::Trample` is checked in `combat.rs` (line 198) via `state.has_keyword()`, which reads from the card's keyword list. Trample excess damage assignment works correctly for a 5/5 (morbid) or 3/3 (no morbid).

### Test coverage

- `festerhide_boar_morbid` (tier5_cards.rs:217): Sets `creature_died_this_turn = true`, casts Boar, verifies 2 +1/+1 counters and effective power 5. PASS.
- `festerhide_boar_no_morbid` (tier5_cards.rs:234): Default state (no death), casts Boar, verifies 0 counters and effective power 3. PASS.

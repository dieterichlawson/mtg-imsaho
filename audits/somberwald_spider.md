# Audit: Somberwald Spider

## Oracle (Scryfall)
- **Name:** Somberwald Spider
- **Cost:** {4}{G}
- **Type:** Creature -- Spider
- **Oracle:** Reach. Morbid -- When Somberwald Spider enters the battlefield, if a creature died this turn, put two +1/+1 counters on Somberwald Spider.
- **P/T:** 2/4

## Implementation: `mtg-engine/src/cards/somberwald_spider.rs`
- **Name:** Somberwald Spider ✅
- **Cost:** {4}{G} ✅
- **Type:** Creature ✅
- **Subtypes:** Spider ✅
- **P/T:** 2/4 ✅
- **Keywords:** Reach ✅
- **Triggered ability:** EntersBattlefield ✅
- **Morbid check:** checks `state.creature_died_this_turn` ✅
- **Counters:** adds 2 PlusOnePlusOne counters ✅

## Verdict: PASS -- no issues found

## Audit — 2026-04-02

**Oracle Text:**
> Reach (This creature can block creatures with flying.)
> Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.

**Card Data:**
- Name: Somberwald Spider — correct
- Cost: {4}{G} — correct
- Type: Creature — Spider — correct
- P/T: 2/4 — correct
- Keywords: Reach — correct

**Behavior:**
- ISSUE: The oracle says "enters with two +1/+1 counters" which is a replacement effect (modifies how the creature enters). The implementation uses `on_enter_battlefield` (a triggered ability that fires after the creature is already on the battlefield). The oracle_text in code says "When Somberwald Spider enters the battlefield" (triggered ability wording) but the actual oracle uses "enters with" (replacement effect).
  - Code oracle_text: `"Morbid — When Somberwald Spider enters the battlefield, if a creature died this turn, put two +1/+1 counters on Somberwald Spider."`
  - Actual oracle: `"Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn."`
- The triggered_abilities vec contains an ETB entry, but the oracle describes a replacement effect.
- Morbid condition checked via `creature_died_this_turn` — correct
- Adds 2 PlusOnePlusOne counters — correct

**Result: ISSUE** — Oracle text mismatch: implementation treats the morbid ability as a triggered ability ("When ... enters the battlefield, put two +1/+1 counters") instead of a replacement effect ("enters with two +1/+1 counters"). This matters because replacement effects cannot be responded to, while triggered abilities use the stack.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Reach (This creature can block creatures with flying.)\nMorbid — This creature enters with two +1/+1 counters on it if a creature died this turn." (was old ETB trigger wording with card name). Doc comment updated. Behavior unchanged.

# Audit: Lost in the Mist

## Oracle (Official)
- **Name:** Lost in the Mist
- **Cost:** {3}{U}{U}
- **Type:** Instant
- **Oracle:** Counter target spell. Return target permanent to its owner's hand.
- **P/T:** N/A

## Implementation
- Name: "Lost in the Mist" -- CORRECT
- Cost: {3}{U}{U} -- CORRECT
- Type: Instant -- CORRECT
- Oracle text matches -- CORRECT
- Two targets: spell + permanent via TwoTargets -- CORRECT
- Counters spell (removes from stack, moves to graveyard) -- CORRECT
- Bounces permanent to hand -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Lost in the Mist
- **Cost:** {3}{U}{U}
- **Type:** Instant
- **Oracle Text:** Counter target spell. Return target permanent to its owner's hand.

### Card Data Checks
- [x] Name: "Lost in the Mist" — correct
- [x] Cost: {3}{U}{U} — correct
- [x] Types: Instant — correct
- [x] Oracle text matches — correct

### Behavior Checks
- [x] Requires two targets (spell + permanent) via `TwoTargets` — correct
- [x] Target validation: checks for Stack or Battlefield zone — correct
- [x] On resolve: counters the spell (first target on stack) — correct
- [x] On resolve: bounces the permanent (second target on battlefield) — correct
- [x] Each target checked independently on resolution (if one is illegal, the other still resolves) — correct
- [x] Spell moves to graveyard after resolve — correct

### Result: PASS

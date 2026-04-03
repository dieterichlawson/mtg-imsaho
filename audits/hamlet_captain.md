# Audit: Hamlet Captain

## Oracle Reference (Scryfall)
- Cost: {1}{G}
- Type: Creature -- Human Warrior
- P/T: 2/2
- Oracle: "Whenever Hamlet Captain attacks or blocks, other Human creatures you control get +1/+1 until end of turn."

## Implementation: hamlet_captain.rs

## Issues Found

No issues found. Name, cost ({1}{G}), type (Creature), subtypes (Human, Warrior), P/T (2/2), oracle text, and both triggered abilities (attacks, blocks) all match. The buff correctly targets other Human creatures you control (excluding self) and applies +1/+1 until end of turn via UntilEndOfTurnEffect.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.
```

### Findings
- Name, cost ({1}{G}), type (Creature -- Human Warrior), P/T (2/2) all match.
- Triggered abilities for Attacks and Blocks both call buff_humans -- correct.
- buff_humans correctly finds other Human creatures controlled by same player and applies +1/+1 until end of turn -- correct.
- Correctly excludes self (o.id != self_id) -- correct.

### ISSUE: Oracle text mismatch in code
- **Oracle (Scryfall)**: "Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn."
- **Code oracle_text**: "Whenever Hamlet Captain attacks or blocks, other Human creatures you control get +1/+1 until end of turn."

Two differences: (1) "Hamlet Captain" vs "this creature", (2) "Human creatures" vs "Humans". Behavior is functionally correct regardless.

### Verdict: ISSUE
Oracle text in code does not match current Scryfall oracle wording.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn." (was "Hamlet Captain attacks or blocks, other Human creatures"). Doc comment updated. Behavior unchanged.

## Audit — 2026-04-02 21:12
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/187/hamlet-captain, cached 2026-04-01)
**Oracle text**: Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.
**Type line**: Creature — Human Warrior
**Status**: PASS

### Code issues
None. All card data matches Scryfall:
- Name: "Hamlet Captain" -- correct
- Cost: {1}{G} = Generic(1) + Green -- correct
- Type: Creature -- correct
- Subtypes: Human, Warrior -- correct
- P/T: 2/2 -- correct
- Oracle text: exact match to Scryfall
- Triggered abilities: two TriggeredAbilityDef entries for Attacks and Blocks, both invoking buff_humans -- correct
- buff_humans correctly excludes self (o.id != self_id) and only targets other Humans you control
- Uses UntilEndOfTurnEffect with power_mod: 1, toughness_mod: 1 -- correct for +1/+1

### Tricky interactions checked (min 3)
1. **Self-exclusion**: buff_humans filters `o.id != self_id`, so Hamlet Captain does not buff itself. Matches "other Humans" in oracle text.
2. **Multiple Hamlet Captains**: If two Hamlet Captains attack together, each triggers independently and buffs all other Humans (including the other Captain). Each pushes separate UntilEndOfTurnEffect entries. Correct behavior -- buffs stack.
3. **Buff persists after source leaves**: The UntilEndOfTurnEffect is independent of Hamlet Captain remaining on the battlefield. If Captain dies in combat, the buff remains until end of turn. This is correct for triggered abilities that create a duration-based effect.
4. **Timing**: Triggers fire at declare attackers/blockers (before combat damage), so the +1/+1 is applied before damage is dealt. Confirmed by trigger system: AttackersDeclared/BlockersDeclared events create PendingTriggers that resolve before combat damage step.

### Test coverage
- No dedicated unit tests for Hamlet Captain
- No AI scenario tests found
- Trigger dispatch verified via triggers.rs: AttacksTrigger/BlocksTrigger correctly routed to on_attacks/on_blocks

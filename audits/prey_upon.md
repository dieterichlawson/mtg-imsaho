# Audit: Prey Upon

## Reference (Scryfall/API)
- **Name:** Prey Upon
- **Mana Cost:** {G}
- **Type:** Sorcery
- **Oracle:** Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)

## Implementation: `prey_upon.rs`
- **Name:** Prey Upon -- CORRECT
- **Mana Cost:** {G} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **P/T:** None -- CORRECT
- **Keywords:** None -- CORRECT
- **Target:** TwoTargets(CreatureWithFilter(YouControl), CreatureWithFilter(YouDontControl)) -- CORRECT
- **Behavior:** On resolve, identifies which target is the caster's creature, calls combat::fight(state, my_creature, their_creature, registry) -- CORRECT

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)
**Type line**: Sorcery
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {G}, type Sorcery, two targets (yours and opponent's creature), fight mechanic via combat::fight. Behavior is correct.

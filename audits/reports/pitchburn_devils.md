# Audit: Pitchburn Devils

## Reference (Scryfall/API)
- **Name:** Pitchburn Devils
- **Mana Cost:** {4}{R}
- **Type:** Creature -- Devil
- **Oracle:** When this creature dies, it deals 3 damage to any target.
- **P/T:** 3/3

## Implementation: `pitchburn_devils.rs`
- **Name:** Pitchburn Devils -- CORRECT
- **Mana Cost:** {4}{R} -- CORRECT
- **Type:** Creature -- Devil -- CORRECT (subtypes: ["Devil"])
- **P/T:** 3/3 -- CORRECT
- **Keywords:** None -- CORRECT
- **Triggered ability:** SelfDies -- CORRECT
- **Behavior:** On dies, presents controller with target choice (any target = creatures + players), then deals 3 damage via PendingEffect::DealDamage -- CORRECT

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: When this creature dies, it deals 3 damage to any target.
**Type line**: Creature -- Devil
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {4}{R}, 3/3, Creature -- Devil, dies trigger deals 3 damage to any target. Uses any_targets helper and present_target_choice for proper target selection. Behavior is correct.

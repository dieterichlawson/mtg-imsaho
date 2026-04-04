# Audit: Moment of Heroism

## Reference (Scryfall/API)
- **Name:** Moment of Heroism
- **Mana Cost:** {1}{W}
- **Type:** Instant
- **Oracle:** Target creature gets +2/+2 and gains lifelink until end of turn.

## Implementation: `moment_of_heroism.rs`
- **Name:** Moment of Heroism -- CORRECT
- **Mana Cost:** {1}{W} -- CORRECT
- **Type:** Instant -- CORRECT
- **Target:** Creature (TargetRequirement::Creature) -- CORRECT
- **+2/+2:** UntilEndOfTurnEffect with power_mod=2, toughness_mod=2 -- CORRECT
- **Lifelink:** UntilEndOfTurnKeyword with Keyword::Lifelink -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Target creature gets +2/+2 and gains lifelink until end of turn.
**Type line**: Instant
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Moment of Heroism", cost {1}{W}, type Instant. Targets a creature, applies +2/+2 and lifelink until end of turn via UntilEndOfTurnEffect and UntilEndOfTurnKeyword. Behavior is correct.

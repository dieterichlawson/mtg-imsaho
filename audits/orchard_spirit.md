# Audit: Orchard Spirit

## Reference (Scryfall/API)
- **Name:** Orchard Spirit
- **Mana Cost:** {2}{G}
- **Type:** Creature -- Spirit
- **Oracle:** This creature can't be blocked except by creatures with flying or reach.
- **P/T:** 2/2

## Implementation: `orchard_spirit.rs`
- **Name:** Orchard Spirit -- CORRECT
- **Mana Cost:** {2}{G} -- CORRECT
- **Type:** Creature -- Spirit -- CORRECT (subtypes: ["Spirit"])
- **P/T:** 2/2 -- CORRECT
- **Keywords:** None -- CORRECT
- **oracle_text field:** "Orchard Spirit can't be blocked except by creatures with flying or reach." -- minor: uses card name instead of "This creature" but functionally equivalent
- **Continuous effect:** BlockRestriction with allowed_blockers Or([HasKeyword(Flying), HasKeyword(Reach)]), scope OnSelf -- CORRECT

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: This creature can't be blocked except by creatures with flying or reach.
**Type line**: Creature -- Spirit
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {2}{G}, 2/2, Creature -- Spirit, block restriction allowing only flying/reach blockers applied to self. Behavior is correct.

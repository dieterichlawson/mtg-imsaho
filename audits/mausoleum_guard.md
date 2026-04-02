# Audit: Mausoleum Guard

## Reference (Scryfall/API)
- **Name:** Mausoleum Guard
- **Mana Cost:** {3}{W}
- **Type:** Creature — Human Scout
- **Oracle:** When this creature dies, create two 1/1 white Spirit creature tokens with flying.
- **P/T:** 2/2

## Implementation: `mausoleum_guard.rs`
- **Name:** Mausoleum Guard -- CORRECT
- **Mana Cost:** {3}{W} -- CORRECT
- **Type:** Creature — Human Scout -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Keywords:** None -- CORRECT
- **Triggered ability:** SelfDies, creates two 1/1 white Spirit tokens with Flying -- CORRECT
- **Token details:** name "Spirit", power 1, toughness 1, color White, type Creature, keyword Flying, subtype "Spirit" -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: When this creature dies, create two 1/1 white Spirit creature tokens with flying.
**Type line**: Creature — Human Scout
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Mausoleum Guard", cost {3}{W}, 2/2, type Creature with subtypes Human/Scout, triggered ability on SelfDies creates two 1/1 white Spirit creature tokens with flying. Behavior is correct.

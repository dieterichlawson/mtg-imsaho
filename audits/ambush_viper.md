# Audit: Ambush Viper

## Reference (Scryfall/API)
- **Name:** Ambush Viper
- **Mana Cost:** {1}{G}
- **Type:** Creature — Snake
- **Oracle:** Flash, Deathtouch
- **P/T:** 2/1

## Implementation: `ambush_viper.rs`
- **Name:** Ambush Viper -- CORRECT
- **Mana Cost:** {1}{G} -- CORRECT
- **Type:** Creature — Snake -- CORRECT
- **P/T:** 2/1 -- CORRECT
- **Keywords:** Flash, Deathtouch -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flash\nDeathtouch
**Type line**: Creature — Snake
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Ambush Viper", cost {1}{G}, 2/1, type Creature — Snake, keywords [Flash, Deathtouch]. Vanilla creature with keywords only, no behavior needed beyond card_data.

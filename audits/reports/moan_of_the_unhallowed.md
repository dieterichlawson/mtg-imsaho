# Audit: Moan of the Unhallowed

## Reference (Scryfall/API)
- **Name:** Moan of the Unhallowed
- **Mana Cost:** {2}{B}{B}
- **Type:** Sorcery
- **Oracle:** Create two 2/2 black Zombie creature tokens. / Flashback {5}{B}{B}

## Implementation: `moan_of_the_unhallowed.rs`
- **Name:** Moan of the Unhallowed -- CORRECT
- **Mana Cost:** {2}{B}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Token creation:** Two 2/2 black Zombie creature tokens -- CORRECT
- **Flashback cost:** {5}{B}{B} (Generic(5) + Colored Black + Colored Black) -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Create two 2/2 black Zombie creature tokens.\nFlashback {5}{B}{B}
**Type line**: Sorcery
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Moan of the Unhallowed", cost {2}{B}{B}, type Sorcery. On resolve creates two 2/2 black Zombie creature tokens. Flashback cost {5}{B}{B} correctly defined. Behavior is correct.

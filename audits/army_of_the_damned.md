# Audit: Army of the Damned

## Reference (Scryfall/API)
- **Name:** Army of the Damned
- **Mana Cost:** {5}{B}{B}{B}
- **Type:** Sorcery
- **Oracle:** Create thirteen tapped 2/2 black Zombie creature tokens. Flashback {7}{B}{B}{B}
- **P/T:** N/A

## Implementation: `army_of_the_damned.rs`
- **Name:** Army of the Damned -- CORRECT
- **Mana Cost:** {5}{B}{B}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Flashback:** {7}{B}{B}{B} -- CORRECT
- **Effect:** Creates 13 tokens, each 2/2 black Zombie creature, enters tapped -- CORRECT
- **Token subtypes:** ["Zombie"] -- CORRECT

## Verdict: PASS -- No issues found

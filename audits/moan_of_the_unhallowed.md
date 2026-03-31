# Audit: Moan of the Unhallowed

## Official Oracle
- **Name:** Moan of the Unhallowed
- **Cost:** {2}{B}{B}
- **Type:** Sorcery
- **Oracle:** Create two 2/2 black Zombie creature tokens. Flashback {5}{B}{B}

## Implementation: `mtg-engine/src/cards/moan_of_the_unhallowed.rs`
- **Name:** Moan of the Unhallowed -- CORRECT
- **Cost:** {2}{B}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Flashback:** {5}{B}{B} -- CORRECT
- **on_resolve:** Creates two 2/2 black Zombie tokens with "Zombie" subtype -- CORRECT

## Verdict
**PASS** -- No issues found.

# Audit: Mulch

## Official Oracle
- **Name:** Mulch
- **Cost:** {1}{G}
- **Type:** Sorcery
- **Oracle:** Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.

## Implementation: `mtg-engine/src/cards/mulch.rs`
- **Name:** Mulch -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **on_resolve:** Reveals top 4, lands to hand, rest to graveyard -- CORRECT

## Verdict
**PASS** -- No issues found.

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

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Mulch  
**Type**: Sorcery | **Cost**: {1}{G}  
**Oracle text**: "Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard."

### Checks
- Name: "Mulch" -- PASS
- Cost: {1}{G} -- PASS
- Type: Sorcery -- PASS
- Behavior (reveal): Takes top 4 cards from library (or fewer if library is smaller) -- PASS
- Behavior (lands to hand): Checks card_types for CardType::Land via registry, moves to Hand -- PASS
- Behavior (rest to graveyard): Non-lands moved to Graveyard -- PASS
- Logging: Logs revealed card names -- PASS
- Spell cleanup: Calls `move_spell_after_resolve` -- PASS

**Verdict: PASS**

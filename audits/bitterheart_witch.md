# Audit: Bitterheart Witch

## Oracle Text (Scryfall)
- **Name:** Bitterheart Witch
- **Mana Cost:** {4}{B}
- **Type:** Creature — Human Shaman
- **P/T:** 1/2
- **Oracle Text:** Deathtouch / When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.

## Implementation File
`mtg-engine/src/cards/isd/bitterheart_witch.rs`

## Card Data Checks
- **Name:** Correct
- **Mana Cost:** Correct ({4}{B})
- **Card Types:** Correct (Creature)
- **Subtypes:** Correct (Human, Shaman)
- **P/T:** Correct (1/2)
- **Keywords:** Correct (Deathtouch)
- **Triggered ability:** Correctly registered as `SelfDies`

## Behavior Checks
- **on_dies:** Searches library for a card with "Curse" subtype, moves to battlefield attached to a player, then shuffles library. Correct.
- **Shuffles even on failure:** Correct -- shuffles even when no Curse is found (oracle says "then shuffle" regardless).

### ISSUE: "you may" is not optional in implementation
- **Oracle:** "you **may** search your library" -- this is optional.
- **Implementation:** Automatically searches and puts the Curse onto the battlefield with no opt-out for the controller (line 40-81, no player choice presented).

### ISSUE: Always attaches to opponent instead of target player
- **Oracle:** "attached to **target player**" -- the controller should choose any player.
- **Implementation:** Hardcodes `let opponent = state.opponent(controller)` (line 42) -- always attaches to opponent.

## Verdict: ISSUE
- The "you may" clause is not implemented; the ability always triggers automatically.
- Target player selection is missing; always attaches to the opponent.

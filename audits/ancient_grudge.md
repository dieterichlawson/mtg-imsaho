# Audit: Ancient Grudge

## Reference (Scryfall/API)
- **Name:** Ancient Grudge
- **Mana Cost:** {1}{R}
- **Type:** Instant
- **Oracle:** Destroy target artifact. Flashback {G}
- **P/T:** N/A

## Implementation: `ancient_grudge.rs`
- **Name:** Ancient Grudge -- CORRECT
- **Mana Cost:** {1}{R} -- CORRECT
- **Type:** Instant -- CORRECT
- **Flashback cost:** {G} -- CORRECT
- **Target:** PermanentWithFilter(HasCardType(Artifact)) -- CORRECT
- **Effect:** Destroy target artifact via `resolve_destroy` -- CORRECT (uses destruction pipeline)

## Verdict: PASS -- No issues found
